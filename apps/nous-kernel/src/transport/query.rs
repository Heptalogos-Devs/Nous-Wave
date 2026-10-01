use super::*;
use nous_core::*;

impl KernelService {
    pub(super) async fn query(
        &self,
        input: p::QueryRequest,
        pool_limit: Option<usize>,
    ) -> Result<k::KernelQueryResponse> {
        let execution = self
            .0
            .execute_query(compile_query(input)?, pool_limit)
            .await?;
        let (result, ticket) =
            if pool_limit.is_some() || !execution.result.resource_actions.is_empty() {
                let (result, ticket) = self.0.cognition.retain_query(execution)?;
                (result, ticket.map(|value| value.to_string()))
            } else {
                (execution.result, None)
            };
        Ok(k::KernelQueryResponse {
            response: Some(query_response(result)),
            validation_ticket: ticket,
        })
    }
    pub(super) async fn finalize_query(
        &self,
        input: k::FinalizeQueryRequest,
    ) -> Result<p::QueryResponse> {
        let subject = SubjectId(id(&input.subject_id)?);
        let order = input
            .order
            .into_iter()
            .map(|item| {
                Ok((
                    from_ref(required(item.reference, "reference")?)?,
                    item.score,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let result = self
            .0
            .cognition
            .finalize_query(
                subject,
                id(&input.validation_ticket)?,
                order,
                input
                    .external_results
                    .into_iter()
                    .map(|value| {
                        Ok(ExternalResourceResult {
                            action_id: id(&value.action_id)?,
                            resource_ref: ResourceRef::new(value.resource_ref)?,
                            status: value.status,
                            records: value
                                .records
                                .into_iter()
                                .map(from_resource_record)
                                .collect::<Result<Vec<_>>>()?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
                nous_runtime::CognitiveContributors {
                    shared: None,
                    memory: self
                        .0
                        .memory
                        .as_ref()
                        .map(|owner| owner as &dyn nous_runtime::CognitiveContributor),
                },
            )
            .await?;
        Ok(query_response(result))
    }
}

fn query_response(result: CognitiveQueryResult) -> p::QueryResponse {
    p::QueryResponse {
        query_id: result.query_id.to_string(),
        status: enum_name(result.status),
        hits: result
            .results
            .into_iter()
            .enumerate()
            .map(|(rank, value)| hit(value, rank + 1))
            .collect(),
        resource_actions: result
            .resource_actions
            .into_iter()
            .map(|value| p::ResourceAction {
                resource_ref: value.resource.as_str().into(),
                action: value.action,
                reason: value.reason,
                current_authority: value.current_authority,
                action_id: value.action_id.to_string(),
                query_text: value.query_text,
                limit: value.limit as u32,
                materialize: value.materialize,
                adapter_kind: value.adapter_kind,
                provider_profile: value.provider_profile,
                provider_locator: value.provider_locator,
                descriptor_digest: value.descriptor_digest,
            })
            .collect(),
        degradation: result
            .degradation
            .into_iter()
            .map(|value| p::Degradation {
                code: value.code,
                detail: value.detail.unwrap_or_default(),
            })
            .collect(),
        bound_query: None,
        diagnostics: result.diagnostics.map(|value| p::QueryDiagnostics {
            candidate_counts: value
                .candidate_counts
                .into_iter()
                .map(|(key, value)| (key, value as u64))
                .collect(),
            lane_status: value.lane_status.into_iter().collect(),
            topology_complete: value.topology_complete,
            topology_discarded_mass: value.topology_discarded_mass,
        }),
        invocations: Vec::new(),
        rerank: None,
        resource_records: result
            .resource_records
            .into_iter()
            .map(to_resource_record)
            .collect(),
    }
}

fn to_resource_record(value: ExternalResourceRecord) -> p::ExternalResourceRecord {
    let reference = value.reference;
    p::ExternalResourceRecord {
        resource_ref: value.resource_ref.as_str().into(),
        reference: Some(p::StableExternalRef {
            provider_kind: reference.provider_kind,
            provider_profile: reference.provider_profile,
            profile_digest: reference.profile_digest,
            resource_ref: reference.resource_ref.as_str().into(),
            provider_resource_id: reference.provider_resource_id,
            entry_id: reference.entry_id,
            entry_version: reference.entry_version,
            content_digest: reference.content_digest,
            source_locator: reference.source_locator,
            retrieved_at: reference.retrieved_at,
            access_scope: reference.access_scope,
        }),
        title: value.title,
        content: value.content,
        provider_rank: value.provider_rank,
        provider_score: value.provider_score,
        version_status: value.version_status,
        access_status: value.access_status,
    }
}
fn from_resource_record(value: p::ExternalResourceRecord) -> Result<ExternalResourceRecord> {
    let reference = required(value.reference, "external reference")?;
    Ok(ExternalResourceRecord {
        resource_ref: ResourceRef::new(value.resource_ref)?,
        reference: StableExternalRef {
            provider_kind: reference.provider_kind,
            provider_profile: reference.provider_profile,
            profile_digest: reference.profile_digest,
            resource_ref: ResourceRef::new(reference.resource_ref)?,
            provider_resource_id: reference.provider_resource_id,
            entry_id: reference.entry_id,
            entry_version: reference.entry_version,
            content_digest: reference.content_digest,
            source_locator: reference.source_locator,
            retrieved_at: reference.retrieved_at,
            access_scope: reference.access_scope,
        },
        title: value.title,
        content: value.content,
        provider_rank: value.provider_rank,
        provider_score: value.provider_score,
        version_status: value.version_status,
        access_status: value.access_status,
    })
}
fn compile_query(input: p::QueryRequest) -> Result<CognitiveQuery> {
    let expression = required(input.expression, "expression")?;
    let modifiers = expression.modifiers.clone().unwrap_or_default();
    let query = CognitiveQuery {
        api_version: API_VERSION,
        subject: SubjectId(id(&input.subject_id)?),
        session: input
            .session_id
            .as_deref()
            .map(id)
            .transpose()?
            .map(SessionId),
        situation: SituationDescriptor::default(),
        expression: compile_expression(expression, true, 0)?,
        exploration: enum_value(&modifiers.exploration).unwrap_or_default(),
        resources: ResourceIntent {
            current_authority: if modifiers.current_authority.is_empty() {
                CurrentAuthorityNeed::None
            } else {
                enum_value(&modifiers.current_authority)?
            },
            synopsis_only: false,
        },
        result_need: ResultNeed {
            limit: modifiers.limit.unwrap_or(12) as usize,
            need_evidence: true,
            need_materialization_handles: modifiers.materialize,
        },
        effort: enum_value(&modifiers.effort).unwrap_or_default(),
        capabilities: CapabilityPolicy::default(),
        diagnostics: enum_value(&modifiers.diagnostics).unwrap_or_default(),
    };
    query.validate()?;
    Ok(query)
}

fn compile_expression(
    expression: p::QueryExpr,
    root: bool,
    depth: usize,
) -> Result<CognitiveQueryExpr> {
    if depth > 16 {
        return Err(Error::Invalid("query expression depth exceeded".into()));
    }
    let modifiers = expression.modifiers.unwrap_or_default();
    if !root
        && (!modifiers.effort.is_empty()
            || modifiers.limit.is_some()
            || !modifiers.diagnostics.is_empty()
            || !modifiers.exploration.is_empty()
            || modifiers.materialize)
    {
        return Err(Error::Invalid("execution controls are root-only".into()));
    }
    let mut node = CognitiveQueryExpr {
        operation: match expression.operation.as_str() {
            "atom" => QueryOperation::Atom,
            "all" => QueryOperation::All,
            "any" => QueryOperation::Any,
            _ => return Err(Error::Invalid("unknown query expression operation".into())),
        },
        ..Default::default()
    };
    for domain in modifiers.domains {
        node.targets.push(match domain.as_str() {
            "memory" => QueryTarget::Memory,
            "evidence" => QueryTarget::Evidence,
            "resource" => QueryTarget::Resource,
            _ => return Err(Error::Invalid("unsupported cognitive query domain".into())),
        });
    }
    for cue in expression.cues {
        append_cue(&mut node, cue)?;
    }
    for preference in modifiers.preferences {
        let operand = if let Some(cue) = preference.cue {
            if !preference.key.is_empty() {
                return Err(Error::Invalid("preference must have one operand".into()));
            }
            let mut value = CognitiveQueryExpr::default();
            append_cue(&mut value, cue)?;
            if let Some(cue) = value.cues.pop() {
                PreferenceOperand::Cue(cue)
            } else if let Some(QueryTarget::Exact { reference }) = value.targets.pop() {
                PreferenceOperand::Exact(reference)
            } else {
                return Err(Error::Invalid("empty preference cue".into()));
            }
        } else {
            PreferenceOperand::Recent(match preference.key.as_str() {
                "recent:occurred" => TimeAxis::Occurred,
                "recent:observed" => TimeAxis::Observed,
                "recent:valid" => TimeAxis::Valid,
                "recent:formed" => TimeAxis::Formed,
                "recent:recorded" => TimeAxis::Recorded,
                _ => {
                    return Err(Error::Invalid(
                        "recent requires an explicit time axis".into(),
                    ));
                }
            })
        };
        node.preferences.push(QueryPreference {
            negative: preference.negative,
            operand,
        });
    }
    if let Some(constraints) = modifiers.constraints {
        node.constraints = constraints_from_proto(constraints)?;
    }
    node.children = expression
        .children
        .into_iter()
        .map(|child| compile_expression(child, false, depth + 1))
        .collect::<Result<_>>()?;
    Ok(node)
}

fn append_cue(query: &mut CognitiveQueryExpr, cue: p::Cue) -> Result<()> {
    match cue
        .cue
        .ok_or_else(|| Error::Invalid("cue is empty".into()))?
    {
        p::cue::Cue::Text(value) => query.cues.push(Cue::Text(TextCue { text: value })),
        p::cue::Cue::Concept(value) => query.cues.push(Cue::Text(TextCue { text: value })),
        p::cue::Cue::SchemaId(value) => query.cues.push(Cue::Schema(SchemaCue {
            schema: CognitiveSchemaId(id(&value)?),
        })),
        p::cue::Cue::EntityRef(value) => query.cues.push(Cue::Entity(EntityCue {
            entity_ref: EntityRef::new(value)?,
        })),
        p::cue::Cue::TagId(value) => query.cues.push(Cue::Tag(TagCue {
            tag: TagId(id(&value)?),
        })),
        p::cue::Cue::ResourceRef(value) => query.cues.push(Cue::Resource(ResourceCue {
            resource: ResourceRef::new(value)?,
        })),
        p::cue::Cue::ExternalObjectRef(value) => query.cues.push(Cue::Object(ObjectCue {
            object_ref: ObjectRef::new(value)?,
        })),
        p::cue::Cue::Reference(value) => {
            let reference = from_ref(value)?;
            query.targets.push(QueryTarget::Exact { reference });
        }
    }
    Ok(())
}

fn interval(value: Option<p::TimeInterval>) -> Result<Option<TimeInterval>> {
    value
        .map(|value| {
            Ok(TimeInterval {
                start: time(value.start)?,
                end: time(value.end)?,
            })
        })
        .transpose()
}

fn constraints_from_proto(value: p::QueryConstraints) -> Result<QueryConstraints> {
    Ok(QueryConstraints {
        source_classes_include: value
            .source_classes_include
            .into_iter()
            .map(SourceClass::from)
            .collect(),
        source_classes_exclude: value
            .source_classes_exclude
            .into_iter()
            .map(SourceClass::from)
            .collect(),
        cognitive_roles_include: value.cognitive_roles,
        formation_modes_include: value.formation_modes,
        entity_requirements: value
            .entity_requirements
            .into_iter()
            .map(EntityRef::new)
            .collect::<Result<_>>()?,
        occurred: interval(value.occurred)?,
        observed: interval(value.observed)?,
        valid: interval(value.valid)?,
        formed: interval(value.formed)?,
        recorded: interval(value.recorded)?,
        include_suppressed: value.include_suppressed,
        authority: value.authority.map(|v| enum_value(&v)).transpose()?,
        modalities: value
            .modalities
            .into_iter()
            .map(|v| enum_value(&v))
            .collect::<Result<_>>()?,
        evidence_classes: value.evidence_classes,
    })
}

fn hit(value: CognitiveHit, rank: usize) -> p::Hit {
    p::Hit {
        reference: Some(to_ref(value.reference)),
        revision: value.revision.map(to_ref),
        text: value.representation,
        authority: enum_name(value.authority),
        evidence: value
            .evidence
            .into_iter()
            .map(|e| p::Evidence {
                reference: Some(to_ref(e.reference)),
                support_role: e.support_role,
            })
            .collect(),
        evidence_families: value
            .match_evidence
            .families
            .into_iter()
            .map(enum_name)
            .collect(),
        entity_refs: value
            .entity_refs
            .into_iter()
            .map(|v| v.as_str().into())
            .collect(),
        lexical_ref: None,
        semantic_role: value.semantic_role,
        cognitive_role: value.cognitive_role,
        formation_mode: value.formation_mode,
        formed_at: value
            .freshness
            .formed_at
            .map(|v| timestamp(v).seconds.to_string()),
        recorded_at: value
            .freshness
            .recorded_at
            .map(|v| timestamp(v).seconds.to_string()),
        score: Some(p::HitScore {
            baseline: value.match_evidence.base_rank_score,
            preference: value.match_evidence.preference_score,
            rerank: value.match_evidence.rerank_score,
            r#final: value.match_evidence.final_score,
            baseline_rank: value.match_evidence.baseline_rank,
            final_rank: rank as u32,
            best_lane_rank: value.match_evidence.best_lane_rank,
            variants: value.match_evidence.variants,
        }),
    }
}
