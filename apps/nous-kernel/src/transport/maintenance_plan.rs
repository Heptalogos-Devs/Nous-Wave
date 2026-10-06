use super::*;
use nous_core::*;
use sqlx::Row;
use std::collections::BTreeMap;

const MAINTENANCE_TEXT_SAFETY_BYTES: usize = 65536;

impl KernelService {
    pub(super) async fn plan_maintenance(
        &self,
        input: k::PlanMaintenanceRequest,
    ) -> Result<k::MaintenancePlan> {
        let claimed = longitudinal::need(required(input.claimed, "claimed")?)?;
        self.require_plan_lease(&claimed).await?;
        let subject = claimed.subject_id;
        let member_bytes = self
            .0
            .configuration
            .snapshot_for_subject(subject)?
            .get(nous_runtime::MEMBER_TEXT_MAX_BYTES)?;
        let sequence = self.0.store.authority_seq(subject).await?;
        let memory = self.require_memory()?;
        if claimed.kind == "topology_maintenance" {
            return self.topology_maintenance_plan(&claimed).await;
        }
        let scope = match claimed.kind.as_str() {
            "episode_resegment" => {
                memory
                    .plan_episode_review(
                        subject,
                        &claimed.scope_kind,
                        &claimed.scope_ref,
                        claimed.trigger_authority_seq,
                    )
                    .await?
            }
            "journal_review" | "journal_revalidate" => {
                memory
                    .plan_journal_review(subject, &claimed.kind, &claimed.scope_ref)
                    .await?
            }
            "memory_consolidate" => {
                memory
                    .plan_consolidation_scope(subject, &claimed.scope_kind, &claimed.scope_ref)
                    .await?
            }
            _ => return Err(Error::Invalid("maintenance kind has no model plan".into())),
        };
        let source = if claimed.kind == "memory_consolidate" {
            consolidation_source(&scope)
        } else {
            None
        };
        let mut plan = k::MaintenancePlan {
            subject_id: subject.0.to_string(),
            authority_seq: sequence,
            cognitive_now: Some(timestamp(self.0.cognition.now(subject))),
            status: scope.status,
            next_due: scope.next_due.map(timestamp),
            problem_code: scope.problem,
            ..Default::default()
        };
        plan.consolidation_source = source;
        if let Some(journal) = scope.journal {
            plan.target = Some(k::JournalTarget {
                journal_id: journal.object.journal_id.0.to_string(),
                expected_revision_id: journal.revision.journal_revision_id.0.to_string(),
                expected_epoch: journal.object.object_epoch,
                intent: "revalidate".into(),
            });
            plan.journal = Some(journal::journal_revision_proto(journal));
        }
        let mut catalog = BTreeMap::new();
        for episode in &scope.episodes {
            plan.sources.push(k::EpisodePartitionSource {
                revision_id: episode.revision.episode_revision_id.0.to_string(),
                expected_epoch: episode.object.object_epoch,
            });
            let support = RevisionSupport::CognitionDependency(CognitionDependency {
                target_revision: CognitiveRef::EpisodeRevision(
                    episode.revision.episode_revision_id,
                ),
                support_role: SupportRole::Direct,
            });
            catalog.insert(support.canonical_key(), support);
            for support in &episode.supports {
                catalog.insert(support.canonical_key(), support.clone());
            }
            for member in &episode.members {
                if let CognitiveRef::Occurrence(id) = member.reference {
                    let support = RevisionSupport::Evidence(EvidenceRef {
                        occurrence_id: id,
                        locator: EvidenceLocator::WholeOccurrence,
                        support_role: SupportRole::Direct,
                    });
                    catalog.insert(support.canonical_key(), support);
                }
            }
        }
        plan.episodes = scope.episodes.into_iter().map(episode::view).collect();
        let (members, scope_exceeded) = self
            .maintenance_member_catalog(subject, &scope.occurrences, member_bytes as usize)
            .await?;
        plan.members = members;
        if claimed.kind == "episode_resegment" && scope_exceeded {
            plan.status = "blocked".into();
            plan.problem_code = Some("historical_repair_scope_exceeded".into());
            plan.next_due = None;
        }
        plan.supports = catalog
            .into_iter()
            .map(|(key, support)| k::SupportCatalogEntry {
                key,
                support: Some(support_proto(support)),
            })
            .collect();
        if claimed.kind == "memory_consolidate" && plan.status == "ready" {
            self.consolidation_context(&mut plan).await?;
        }
        if self.0.store.authority_seq(subject).await? != sequence {
            return Err(Error::Conflict(
                "maintenance source snapshot changed during planning".into(),
            ));
        }
        Ok(plan)
    }

    async fn require_plan_lease(&self, claimed: &nous_runtime::MaintenanceNeed) -> Result<()> {
        let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM maintenance_needs WHERE subject_id=$1 AND need_id=$2 AND state='leased' AND lease_token=$3 AND lease_until>clock_timestamp() AND kind=$4 AND scope_kind=$5 AND scope_ref=$6)")
            .bind(claimed.subject_id.0).bind(claimed.need_id).bind(claimed.lease_token).bind(&claimed.kind).bind(&claimed.scope_kind).bind(&claimed.scope_ref).fetch_one(self.0.store.pool()).await.map_err(nous_persistence::database_error)?;
        if !valid {
            return Err(Error::Conflict(
                "maintenance planning lease expired or replaced".into(),
            ));
        }
        Ok(())
    }
    async fn topology_maintenance_plan(
        &self,
        claimed: &nous_runtime::MaintenanceNeed,
    ) -> Result<k::MaintenancePlan> {
        let subject = claimed.subject_id;
        let sequence = self.0.store.authority_seq(subject).await?;
        let memory = self.require_memory()?;
        {
            let focus = parse_reference(&claimed.scope_kind, &claimed.scope_ref)?;
            let topology = match memory.plan_topology(subject, focus).await {
                Ok(plan) => plan,
                Err(Error::NotFound(_)) => {
                    return Ok(k::MaintenancePlan {
                        subject_id: subject.0.to_string(),
                        authority_seq: sequence,
                        status: "obsolete".into(),
                        ..Default::default()
                    });
                }
                Err(error) => return Err(error),
            };
            Ok(k::MaintenancePlan {
                subject_id: subject.0.to_string(),
                authority_seq: topology.authority_seq,
                cognitive_now: Some(timestamp(self.0.cognition.now(subject))),
                status: "ready".into(),
                topology_model_input_json: Some(topology.model_input().to_string()),
                topology_plan_json: Some(
                    serde_json::to_string(&topology)
                        .map_err(|e| Error::Infrastructure(e.to_string()))?,
                ),
                ..Default::default()
            })
        }
    }
    pub(super) async fn commit_topology(
        &self,
        input: k::CommitTopologyRequest,
    ) -> Result<k::CommitTopologyResponse> {
        if input.plan_json.len() > 1024 * 1024 || input.proposal_json.len() > 65536 {
            return Err(Error::Invalid("topology request envelope exceeded".into()));
        }
        let plan = serde_json::from_str(&input.plan_json)
            .map_err(|e| Error::Invalid(format!("invalid topology plan: {e}")))?;
        let proposal = serde_json::from_str(&input.proposal_json)
            .map_err(|e| Error::Invalid(format!("invalid topology proposal: {e}")))?;
        let outcome = self
            .require_memory()?
            .commit_topology(nous_memory::CommitTopologyInput {
                operation_id: OperationId(id(&input.operation_id)?),
                claimed: longitudinal::need(required(input.claimed, "claimed")?)?,
                plan,
                proposal,
                producer: super::convert::from_producer(required(input.producer, "producer")?)?,
            })
            .await?;
        Ok(k::CommitTopologyResponse {
            outcome_json: serde_json::to_string(&outcome)
                .map_err(|e| Error::Infrastructure(e.to_string()))?,
        })
    }
    async fn maintenance_member_catalog(
        &self,
        subject: SubjectId,
        occurrences: &[OccurrenceId],
        member_bytes: usize,
    ) -> Result<(Vec<k::ExperienceMember>, bool)> {
        let ids: Vec<uuid::Uuid> = occurrences.iter().map(|id| id.0).collect();
        let rows=sqlx::query("SELECT o.*,e.recorded_seq,e.session_id,e.active_work_context_id,e.active_work_context_revision,d.derived_representation_id FROM observation_occurrences o LEFT JOIN experience_items e ON e.occurrence_id=o.occurrence_id AND e.subject_id=o.subject_id LEFT JOIN LATERAL (SELECT r.derived_representation_id FROM coverage_needs c JOIN source_regions s USING(source_region_id) JOIN derived_representations r ON r.derived_representation_id=c.current_representation_id WHERE c.subject_id=$1 AND s.artifact_id=o.artifact_id AND c.state='ready' AND r.payload_text IS NOT NULL ORDER BY r.created_at DESC,r.derived_representation_id LIMIT 1) d ON true WHERE o.subject_id=$1 AND o.occurrence_id=ANY($2::uuid[]) ORDER BY o.observed_at,e.recorded_seq NULLS LAST,o.occurrence_id")
            .bind(subject.0).bind(ids).fetch_all(self.0.store.pool()).await.map_err(nous_persistence::database_error)?;
        let mut members = Vec::with_capacity(rows.len());
        let mut remaining = MAINTENANCE_TEXT_SAFETY_BYTES;
        let mut scope_exceeded = false;
        for row in rows {
            let occurrence: uuid::Uuid = row.get("occurrence_id");
            let derived: Option<uuid::Uuid> = row.get("derived_representation_id");
            let reference = derived
                .map(|id| CognitiveRef::DerivedRepresentation(DerivedRepresentationId(id)))
                .unwrap_or(CognitiveRef::Occurrence(OccurrenceId(occurrence)));
            let (text, partial) = if remaining > 0 {
                let source = self
                    .0
                    .materialize(
                        subject,
                        nous_material::MaterializeRequest {
                            reference,
                            byte_range: None,
                            max_bytes: remaining.min(member_bytes) as u64,
                            resource_handle: None,
                            resource: None,
                        },
                    )
                    .await?;
                if source.media_type.starts_with("text/") || source.media_type == "application/json"
                {
                    remaining = remaining.saturating_sub(source.bytes.len());
                    let text = experience_text_prefix(&source.bytes, source.partial)?;
                    (text.to_owned(), source.partial)
                } else {
                    (String::new(), true)
                }
            } else {
                scope_exceeded = true;
                (String::new(), true)
            };
            members.push(k::ExperienceMember {
                key: format!("occurrence:{occurrence}"),
                occurrence_id: occurrence.to_string(),
                recorded_seq: row.get("recorded_seq"),
                observed_at: Some(timestamp(row.get("observed_at"))),
                session_id: row
                    .get::<Option<uuid::Uuid>, _>("session_id")
                    .map(|id| id.to_string()),
                work_context_id: row
                    .get::<Option<uuid::Uuid>, _>("active_work_context_id")
                    .map(|id| id.to_string()),
                work_context_revision: row.get("active_work_context_revision"),
                conversation_ref: row.get("conversation_ref"),
                actor_entity_ref: row.get("actor_entity_ref"),
                source_class: row.get("source_class"),
                text,
                text_partial: partial,
                occurred_time: Some(occurrence_time(&row)?),
            });
        }
        Ok((members, scope_exceeded))
    }
}

fn consolidation_source(scope: &nous_memory::MaintenanceScope) -> Option<k::ExpectedCognition> {
    if let Some(journal) = &scope.journal {
        return Some(k::ExpectedCognition {
            reference: Some(to_ref(CognitiveRef::JournalRevision(
                journal.revision.journal_revision_id,
            ))),
            expected_epoch: journal.object.object_epoch,
            object_id: journal.object.journal_id.0.to_string(),
        });
    }
    scope.episodes.first().map(|episode| k::ExpectedCognition {
        reference: Some(to_ref(CognitiveRef::EpisodeRevision(
            episode.revision.episode_revision_id,
        ))),
        expected_epoch: episode.object.object_epoch,
        object_id: episode.object.episode_id.0.to_string(),
    })
}

fn occurrence_time(row: &sqlx::postgres::PgRow) -> Result<p::TemporalExtent> {
    let kind: String = row.get("occurred_time_kind");
    let start: Option<chrono::DateTime<chrono::Utc>> = row.get("occurred_time_start");
    let end: Option<chrono::DateTime<chrono::Utc>> = row.get("occurred_time_end");
    let time = match kind.as_str() {
        "instant" => TemporalExtent::Instant {
            at: required(start, "occurred instant")?,
        },
        "interval" => TemporalExtent::Interval { start, end },
        "unknown" => TemporalExtent::Unknown,
        _ => {
            return Err(Error::Infrastructure(
                "invalid occurrence temporal kind".into(),
            ));
        }
    };
    Ok(temporal_proto(&time))
}

fn experience_text_prefix(bytes: &[u8], partial: bool) -> Result<&str> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(error) if partial && error.error_len().is_none() => {
            std::str::from_utf8(&bytes[..error.valid_up_to()])
                .map_err(|error| Error::Invalid(error.to_string()))
        }
        Err(error) => Err(Error::Invalid(format!("experience text encoding: {error}"))),
    }
}
