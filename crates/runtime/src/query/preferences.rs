use chrono::{DateTime, Utc};
use nous_core::*;

pub const PREFERENCE_WEIGHT: f64 = 0.02;
pub const PREFERENCE_CAP: f64 = 0.06;
const RECENCY_SECONDS: f64 = 30.0 * 86400.0;

fn extent_time(value: &TemporalExtent) -> Option<DateTime<Utc>> {
    match value {
        TemporalExtent::Unknown => None,
        TemporalExtent::Instant { at } => Some(*at),
        TemporalExtent::Interval { start, end } => end.or(*start),
    }
}
fn cue_match(cue: &Cue, hit: &CognitiveHit) -> f64 {
    let reference = match cue {
        Cue::Text(value) => {
            return lexical_match(
                &value.text,
                hit.representation.as_deref().unwrap_or_default(),
            );
        }
        Cue::Example(value) => {
            return lexical_match(
                &value.text,
                hit.representation.as_deref().unwrap_or_default(),
            );
        }
        Cue::Entity(value) => return f64::from(hit.entity_refs.contains(&value.entity_ref)),
        Cue::Tag(value) => CognitiveRef::Tag(value.tag),
        Cue::Schema(value) => CognitiveRef::CognitiveSchema(value.schema),
        Cue::Resource(value) => CognitiveRef::Resource(value.resource.clone()),
        Cue::Object(value) => CognitiveRef::ExternalObject(value.object_ref.clone()),
        Cue::Artifact(value) => CognitiveRef::Artifact(value.artifact),
        Cue::MediaRegion(value) => value.region.clone(),
        Cue::Relation(value) => {
            return f64::from(hit.evidence.iter().any(|evidence| {
                evidence.reference == value.from || evidence.reference == value.to
            }));
        }
    };
    f64::from(
        hit.reference == reference
            || hit.preference_refs.contains(&reference)
            || hit
                .evidence
                .iter()
                .any(|evidence| evidence.reference == reference),
    )
}
fn lexical_match(text: &str, candidate: &str) -> f64 {
    let candidate = candidate.to_lowercase();
    let terms: Vec<_> = text
        .split_whitespace()
        .map(str::to_lowercase)
        .filter(|term| !term.is_empty())
        .collect();
    if terms.is_empty() {
        return 0.0;
    }
    terms
        .iter()
        .filter(|term| candidate.contains(term.as_str()))
        .count() as f64
        / terms.len() as f64
}
pub(super) fn score(
    preferences: &[QueryPreference],
    hit: &CognitiveHit,
    now: DateTime<Utc>,
) -> f64 {
    preferences
        .iter()
        .map(|preference| {
            let value = match &preference.operand {
                PreferenceOperand::Cue(cue) => cue_match(cue, hit),
                PreferenceOperand::Exact(reference) => f64::from(
                    &hit.reference == reference || hit.revision.as_ref() == Some(reference),
                ),
                PreferenceOperand::Recent(axis) => {
                    let time = match axis {
                        TimeAxis::Occurred => {
                            hit.freshness.occurred.iter().filter_map(extent_time).max()
                        }
                        TimeAxis::Observed => hit.freshness.observed_at,
                        TimeAxis::Valid => extent_time(&hit.freshness.valid_time),
                        TimeAxis::Formed => hit.freshness.formed_at,
                        TimeAxis::Recorded => hit.freshness.recorded_at,
                    };
                    time.map_or(0.0, |time| {
                        1.0 / (1.0
                            + now.signed_duration_since(time).num_seconds().max(0) as f64
                                / RECENCY_SECONDS)
                    })
                }
            };
            if preference.negative {
                -value * PREFERENCE_WEIGHT
            } else {
                value * PREFERENCE_WEIGHT
            }
        })
        .sum::<f64>()
        .clamp(-PREFERENCE_CAP, PREFERENCE_CAP)
}

pub(super) fn apply(
    hits: &mut [CognitiveHit],
    preferences: &[QueryPreference],
    now: DateTime<Utc>,
) {
    for (index, hit) in hits.iter_mut().enumerate() {
        hit.match_evidence.baseline_rank = (index + 1) as u32;
        hit.match_evidence.preference_score = score(preferences, hit, now);
        hit.match_evidence.final_score =
            hit.match_evidence.base_rank_score + hit.match_evidence.preference_score;
    }
    order(hits);
}

pub(super) fn order(hits: &mut [CognitiveHit]) {
    hits.sort_by(|a, b| {
        b.match_evidence
            .final_score
            .total_cmp(&a.match_evidence.final_score)
            .then_with(|| {
                a.match_evidence
                    .baseline_rank
                    .cmp(&b.match_evidence.baseline_rank)
            })
            .then_with(|| a.reference.to_string().cmp(&b.reference.to_string()))
    });
    for (index, hit) in hits.iter_mut().enumerate() {
        hit.match_evidence.final_rank = (index + 1) as u32;
    }
}
