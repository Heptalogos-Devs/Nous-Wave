use super::*;
use chrono::{DateTime, Utc};
use nous_core::{CognitiveRef, Result, TemporalExtent};
use nous_memory::{CognitionDependency, EvidenceLocator, EvidenceRef, RevisionSupport};
use prost_types::{Struct, Timestamp, Value, value::Kind};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::BTreeMap;
use uuid::Uuid;

pub fn id(value: &str) -> Result<Uuid> {
    value
        .parse()
        .map_err(|_| Error::Invalid("invalid UUID".into()))
}
pub fn required<T>(value: Option<T>, name: &str) -> Result<T> {
    value.ok_or_else(|| Error::Invalid(format!("{name} is required")))
}
pub fn enum_value<T: DeserializeOwned>(value: &str) -> Result<T> {
    serde_json::from_value(serde_json::Value::String(value.into()))
        .map_err(|_| Error::Invalid(format!("invalid enum value: {value}")))
}
pub fn enum_name<T: Serialize>(value: T) -> String {
    serde_json::to_value(value).map_or_else(
        |_| "invalid_enum".into(),
        |value| {
            value
                .as_str()
                .map_or_else(|| "invalid_enum".into(), str::to_owned)
        },
    )
}
pub fn time(value: Option<Timestamp>) -> Result<Option<DateTime<Utc>>> {
    value
        .map(|value| {
            if !(0..1_000_000_000).contains(&value.nanos) {
                return Err(Error::Invalid("invalid timestamp nanos".into()));
            }
            DateTime::from_timestamp(value.seconds, value.nanos as u32)
                .ok_or_else(|| Error::Invalid("invalid timestamp".into()))
        })
        .transpose()
}
pub fn timestamp(value: DateTime<Utc>) -> Timestamp {
    Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}

pub fn temporal(value: Option<p::TemporalExtent>) -> Result<TemporalExtent> {
    let Some(value) = value else {
        return Ok(TemporalExtent::Unknown);
    };
    match value.value {
        Some(p::temporal_extent::Value::Instant(value)) => Ok(TemporalExtent::Instant {
            at: time(Some(value))?.ok_or_else(|| Error::Invalid("invalid instant".into()))?,
        }),
        Some(p::temporal_extent::Value::Interval(value)) => Ok(TemporalExtent::Interval {
            start: time(value.start)?,
            end: time(value.end)?,
        }),
        None => Ok(TemporalExtent::Unknown),
    }
}

pub fn temporal_proto(value: &TemporalExtent) -> p::TemporalExtent {
    p::TemporalExtent {
        value: match value {
            TemporalExtent::Unknown => None,
            TemporalExtent::Instant { at } => {
                Some(p::temporal_extent::Value::Instant(timestamp(*at)))
            }
            TemporalExtent::Interval { start, end } => {
                Some(p::temporal_extent::Value::Interval(p::TimeInterval {
                    start: start.map(timestamp),
                    end: end.map(timestamp),
                }))
            }
        },
    }
}

pub fn support(value: p::RevisionSupport) -> Result<RevisionSupport> {
    match value
        .support
        .ok_or_else(|| Error::Invalid("revision support is empty".into()))?
    {
        p::revision_support::Support::Evidence(value) => {
            let locator = match value
                .locator
                .ok_or_else(|| Error::Invalid("evidence locator is empty".into()))?
            {
                p::evidence_ref::Locator::WholeOccurrence(_) => EvidenceLocator::WholeOccurrence,
                p::evidence_ref::Locator::SourceRegionId(value) => {
                    EvidenceLocator::SourceRegion(nous_core::SourceRegionId(id(&value)?))
                }
                p::evidence_ref::Locator::DerivedRepresentationId(value) => {
                    EvidenceLocator::DerivedRepresentation(nous_core::DerivedRepresentationId(id(
                        &value,
                    )?))
                }
                p::evidence_ref::Locator::DerivedRegionId(value) => {
                    EvidenceLocator::DerivedRegion(nous_core::DerivedRegionId(id(&value)?))
                }
            };
            Ok(RevisionSupport::Evidence(EvidenceRef {
                occurrence_id: nous_core::OccurrenceId(id(&value.occurrence_id)?),
                locator,
                support_role: enum_value(&value.support_role)?,
            }))
        }
        p::revision_support::Support::CognitionDependency(value) => {
            Ok(RevisionSupport::CognitionDependency(CognitionDependency {
                target_revision: from_ref(required(value.target_revision, "target_revision")?)?,
                support_role: enum_value(&value.support_role)?,
            }))
        }
        p::revision_support::Support::Seed(value) => {
            let reference = from_ref(required(value.seed_version, "seed_version")?)?;
            match reference {
                CognitiveRef::CognitiveSeedVersion(id) => Ok(RevisionSupport::Seed(
                    nous_core::SeedSupportRef::new(id, value.semantic_path)?,
                )),
                _ => Err(Error::Invalid(
                    "seed support must target a Cognitive Seed version".into(),
                )),
            }
        }
    }
}

pub fn support_proto(value: RevisionSupport) -> p::RevisionSupport {
    let support = match value {
        RevisionSupport::Evidence(value) => {
            let locator = match value.locator {
                EvidenceLocator::WholeOccurrence => p::evidence_ref::Locator::WholeOccurrence(true),
                EvidenceLocator::SourceRegion(id) => {
                    p::evidence_ref::Locator::SourceRegionId(id.0.to_string())
                }
                EvidenceLocator::DerivedRepresentation(id) => {
                    p::evidence_ref::Locator::DerivedRepresentationId(id.0.to_string())
                }
                EvidenceLocator::DerivedRegion(id) => {
                    p::evidence_ref::Locator::DerivedRegionId(id.0.to_string())
                }
            };
            p::revision_support::Support::Evidence(p::EvidenceRef {
                occurrence_id: value.occurrence_id.0.to_string(),
                locator: Some(locator),
                support_role: enum_name(value.support_role),
            })
        }
        RevisionSupport::CognitionDependency(value) => {
            p::revision_support::Support::CognitionDependency(p::CognitionDependency {
                target_revision: Some(to_ref(value.target_revision)),
                support_role: enum_name(value.support_role),
            })
        }
        RevisionSupport::Seed(value) => p::revision_support::Support::Seed(p::SeedSupportRef {
            seed_version: Some(to_ref(CognitiveRef::CognitiveSeedVersion(
                value.seed_version_id,
            ))),
            semantic_path: value.semantic_path,
        }),
    };
    p::RevisionSupport {
        support: Some(support),
    }
}
pub fn from_ref(value: p::CognitiveRef) -> Result<CognitiveRef> {
    nous_core::parse_reference(&value.kind, &value.value)
}
pub fn to_ref(value: CognitiveRef) -> p::CognitiveRef {
    let (kind, value) = nous_core::reference_parts(&value);
    p::CognitiveRef { kind, value }
}
pub fn json(value: Value) -> serde_json::Value {
    match value.kind {
        None | Some(Kind::NullValue(_)) => serde_json::Value::Null,
        Some(Kind::BoolValue(value)) => value.into(),
        Some(Kind::NumberValue(value)) => serde_json::json!(value),
        Some(Kind::StringValue(value)) => value.into(),
        Some(Kind::ListValue(value)) => {
            serde_json::Value::Array(value.values.into_iter().map(json).collect())
        }
        Some(Kind::StructValue(value)) => object(Some(value)),
    }
}
pub fn object(value: Option<Struct>) -> serde_json::Value {
    serde_json::Value::Object(
        value
            .unwrap_or_default()
            .fields
            .into_iter()
            .map(|(key, value)| (key, json(value)))
            .collect(),
    )
}
pub fn value(json: serde_json::Value) -> Value {
    Value {
        kind: Some(match json {
            serde_json::Value::Null => Kind::NullValue(0),
            serde_json::Value::Bool(v) => Kind::BoolValue(v),
            serde_json::Value::Number(v) => v
                .as_f64()
                .map_or_else(|| Kind::StringValue(v.to_string()), Kind::NumberValue),
            serde_json::Value::String(v) => Kind::StringValue(v),
            serde_json::Value::Array(v) => Kind::ListValue(prost_types::ListValue {
                values: v.into_iter().map(value).collect(),
            }),
            serde_json::Value::Object(v) => Kind::StructValue(Struct {
                fields: v.into_iter().map(|(k, v)| (k, value(v))).collect(),
            }),
        }),
    }
}
pub fn to_object(json: serde_json::Value) -> Option<Struct> {
    match value(json).kind {
        Some(Kind::StructValue(v)) => Some(v),
        _ => Some(Struct {
            fields: BTreeMap::new(),
        }),
    }
}
pub fn page(page: Option<p::Page>, scope: &str) -> Result<(i64, Option<Uuid>)> {
    let page = page.unwrap_or_default();
    if page.page_size > 200 {
        return Err(Error::Invalid("page_size exceeds 200".into()));
    }
    let last = if page.page_token.is_empty() {
        None
    } else {
        let (digest, last) = page
            .page_token
            .split_once('.')
            .ok_or_else(|| Error::Invalid("invalid page token".into()))?;
        if digest != blake3::hash(scope.as_bytes()).to_hex().as_str() {
            return Err(Error::Invalid(
                "page token belongs to another request".into(),
            ));
        }
        Some(id(last)?)
    };
    Ok((
        i64::from(if page.page_size == 0 {
            50
        } else {
            page.page_size
        }),
        last,
    ))
}
pub fn next_token(scope: &str, id: Uuid) -> String {
    format!("{}.{}", blake3::hash(scope.as_bytes()).to_hex(), id)
}
