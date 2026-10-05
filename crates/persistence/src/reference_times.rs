//! Evidence time metadata, resolved from the actual Subject-scoped sources.
use crate::*;
use chrono::{DateTime, Utc};
use nous_core::*;
use sqlx::Row;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct ReferenceTimes {
    pub sources: Vec<(TemporalExtent, DateTime<Utc>, DateTime<Utc>)>,
    pub formed_at: Option<DateTime<Utc>>,
}
impl ReferenceTimes {
    pub fn matches(&self, constraints: &QueryConstraints) -> bool {
        if constraints.valid.is_some() {
            return false;
        } // Evidence has no world-valid claim.
        if constraints
            .formed
            .is_some_and(|interval| self.formed_at.is_none_or(|at| !interval.contains(at)))
        {
            return false;
        }
        let filtered = constraints.occurred.is_some()
            || constraints.observed.is_some()
            || constraints.recorded.is_some();
        !filtered
            || self.sources.iter().any(|(occurred, observed, recorded)| {
                constraints
                    .occurred
                    .is_none_or(|interval| occurred.overlaps_interval(&interval))
                    && constraints
                        .observed
                        .is_none_or(|interval| interval.contains(*observed))
                    && constraints
                        .recorded
                        .is_none_or(|interval| interval.contains(*recorded))
            })
    }
    pub fn freshness(&self) -> FreshnessDescriptor {
        FreshnessDescriptor {
            occurred: self
                .sources
                .iter()
                .map(|(time, _, _)| time.clone())
                .collect(),
            observed_at: self.sources.iter().map(|(_, at, _)| *at).max(),
            recorded_at: self.sources.iter().map(|(_, _, at)| *at).max(),
            formed_at: self.formed_at,
            valid_time: TemporalExtent::Unknown,
        }
    }
}
impl AuthorityStore {
    pub async fn reference_times(
        &self,
        subject: SubjectId,
        refs: &[CognitiveRef],
    ) -> Result<HashMap<CognitiveRef, ReferenceTimes>> {
        if refs.len() > 256 {
            return Err(Error::Invalid("reference time envelope exceeded".into()));
        }
        let mut kinds = Vec::new();
        let mut values = Vec::new();
        let mut result = HashMap::new();
        for reference in refs {
            let text = reference.to_string();
            let (kind, value) = text
                .split_once(':')
                .ok_or_else(|| Error::Invalid("invalid time reference".into()))?;
            kinds.push(kind.to_owned());
            values.push(value.to_owned());
            result.insert(reference.clone(), ReferenceTimes::default());
        }
        let rows = sqlx::query(r#"
WITH requested AS (SELECT * FROM unnest($2::text[],$3::text[]) AS r(kind,value)),
origins AS (
 SELECT r.kind,r.value,o.occurrence_id,NULL::timestamptz formed_at,o.created_at recorded_at FROM requested r JOIN observation_occurrences o ON r.kind='occurrence' AND o.occurrence_id::text=r.value AND o.subject_id=$1
 UNION ALL
 SELECT r.kind,r.value,o.occurrence_id,NULL::timestamptz,s.created_at FROM requested r JOIN source_regions s ON r.kind='source_region' AND s.source_region_id::text=r.value AND s.subject_id=$1 JOIN observation_occurrences o ON o.artifact_id=s.artifact_id AND o.subject_id=$1
 UNION ALL
 SELECT r.kind,r.value,o.occurrence_id,d.created_at,d.created_at FROM requested r JOIN derived_representations d ON d.subject_id=$1 AND (r.kind='derived_representation' AND d.derived_representation_id::text=r.value OR r.kind='derived_region' AND EXISTS(SELECT 1 FROM derived_regions g WHERE g.subject_id=$1 AND g.derived_region_id::text=r.value AND g.derived_representation_id=d.derived_representation_id))
 JOIN LATERAL representation_source_regions($1,d.derived_representation_id) roots ON true JOIN source_regions s USING(source_region_id) JOIN observation_occurrences o ON o.artifact_id=s.artifact_id AND o.subject_id=$1
)
SELECT DISTINCT p.kind,p.value,p.formed_at,o.occurrence_id,o.occurred_time_kind,o.occurred_time_start,o.occurred_time_end,o.observed_at,p.recorded_at FROM origins p JOIN observation_occurrences o USING(occurrence_id) WHERE o.subject_id=$1
"#).bind(subject.0).bind(&kinds).bind(&values).fetch_all(self.pool()).await.map_err(database_error)?;
        for row in rows {
            let reference = parse_reference(
                &row.get::<String, _>("kind"),
                &row.get::<String, _>("value"),
            )?;
            let kind: String = row.get("occurred_time_kind");
            let start: Option<DateTime<Utc>> = row.get("occurred_time_start");
            let end = row.get("occurred_time_end");
            let occurred = match kind.as_str() {
                "instant" => TemporalExtent::Instant {
                    at: start.ok_or_else(|| {
                        Error::Infrastructure("instant occurrence has no time".into())
                    })?,
                },
                "interval" => TemporalExtent::Interval { start, end },
                _ => TemporalExtent::Unknown,
            };
            let times = result
                .get_mut(&reference)
                .ok_or_else(|| Error::Infrastructure("unexpected reference time row".into()))?;
            times
                .sources
                .push((occurred, row.get("observed_at"), row.get("recorded_at")));
            times.formed_at = row.get("formed_at");
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn correlated_axes_do_not_combine_different_origins_or_invent_unknown_times() {
        let early = DateTime::parse_from_rfc3339("2026-09-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let late = early + chrono::Duration::days(10);
        let times = ReferenceTimes {
            sources: vec![
                (TemporalExtent::Instant { at: early }, late, late),
                (TemporalExtent::Instant { at: late }, early, early),
            ],
            formed_at: None,
        };
        let constraints = QueryConstraints {
            occurred: Some(TimeInterval {
                start: None,
                end: Some(late),
            }),
            observed: Some(TimeInterval {
                start: None,
                end: Some(late),
            }),
            ..Default::default()
        };
        assert!(!times.matches(&constraints));
        assert!(!ReferenceTimes::default().matches(&constraints));
        assert!(times.matches(&QueryConstraints::default()));
        assert!(!times.matches(&QueryConstraints {
            valid: Some(TimeInterval {
                start: None,
                end: Some(late)
            }),
            ..Default::default()
        }));
    }
}
