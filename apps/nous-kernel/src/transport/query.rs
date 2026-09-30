use super::*;
use nous_core::*;

impl KernelService {
    pub(super) async fn query(&self, input: p::QueryRequest) -> Result<p::QueryResponse> {
        let query = compile_query(input)?;
        let result = self.0.query(query).await?;
        Ok(p::QueryResponse {
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
        })
    }
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
        resources: ResourceIntent::default(),
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
            preference: 0.0,
            rerank: None,
            r#final: value.match_evidence.final_score,
            baseline_rank: rank as u32,
            final_rank: rank as u32,
            best_lane_rank: value.match_evidence.best_lane_rank,
            variants: value.match_evidence.variants,
        }),
    }
}
