use super::*;
use nous_core::*;

impl KernelService {
    pub(super) async fn query(&self, input: p::QueryRequest) -> Result<p::QueryResponse> {
        let query = compile_query(input)?;
        let result = self.0.query(query).await?;
        Ok(p::QueryResponse {
            query_id: result.query_id.to_string(),
            status: enum_name(result.status),
            hits: result.results.into_iter().map(hit).collect(),
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
        })
    }
}

fn compile_query(input: p::QueryRequest) -> Result<CognitiveQuery> {
    let expression = required(input.expression, "expression")?;
    let modifiers = expression.modifiers.unwrap_or_default();
    let mut query = CognitiveQuery {
        api_version: API_VERSION,
        subject: SubjectId(id(&input.subject_id)?),
        session: input
            .session_id
            .as_deref()
            .map(id)
            .transpose()?
            .map(SessionId),
        situation: SituationDescriptor::default(),
        targets: Vec::new(),
        cues: Vec::new(),
        constraints: QueryConstraints::default(),
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
    for domain in modifiers.domains {
        match domain.as_str() {
            "memory" => query.targets.push(QueryTarget::Memory),
            "self" => query.targets.push(QueryTarget::SelfCognition),
            "" => {}
            _ => return Err(Error::Invalid("unsupported cognitive query domain".into())),
        }
    }
    for cue in expression.cues {
        append_cue(&mut query, cue)?;
    }
    if let Some(cue) = expression.self_facet {
        if cue.kind.is_none() && cue.key.is_none() {
            return Err(Error::Invalid("self_facet cue needs kind or key".into()));
        }
        query.targets.push(QueryTarget::SelfCognition);
        query.cues.push(Cue::SelfFacet(SelfFacetCue {
            kind: cue.kind,
            key: cue.key,
        }));
    }
    if let Some(situation) = expression.social_situation {
        query.situation.social = Some(SocialSituation {
            participants: situation
                .participants
                .into_iter()
                .map(EntityRef::new)
                .collect::<Result<_>>()?,
            groups: situation
                .groups
                .into_iter()
                .map(EntityRef::new)
                .collect::<Result<_>>()?,
            communities: situation
                .communities
                .into_iter()
                .map(EntityRef::new)
                .collect::<Result<_>>()?,
            channel: situation.channel.map(EntityRef::new).transpose()?,
            context_tokens: situation.context_tokens,
            topic_tokens: situation.topic_tokens,
        });
    }
    if let Some(constraints) = modifiers.constraints {
        query.constraints = constraints_from_proto(constraints)?;
    }
    query.validate()?;
    Ok(query)
}

fn append_cue(query: &mut CognitiveQuery, cue: p::Cue) -> Result<()> {
    match cue
        .cue
        .ok_or_else(|| Error::Invalid("cue is empty".into()))?
    {
        p::cue::Cue::Text(value) => query.cues.push(Cue::Text(TextCue { text: value })),
        p::cue::Cue::Concept(value) => query.cues.push(Cue::Text(TextCue { text: value })),
        p::cue::Cue::SchemaId(value) => query.cues.push(Cue::Schema(SchemaCue {
            schema: CognitiveSchemaId(id(&value)?),
        })),
        p::cue::Cue::Reference(value) => {
            let reference = from_ref(value)?;
            query.targets.push(QueryTarget::Exact { reference });
        }
        p::cue::Cue::SocialRelation(value) => {
            query.targets.push(QueryTarget::Social);
            query.cues.push(Cue::SocialRelation(SocialRelationCue {
                relation_type_key: value.relation_type_key,
                from: value.from.map(EntityRef::new).transpose()?,
                to: value.to.map(EntityRef::new).transpose()?,
                include_views: value.include_views,
            }));
        }
        p::cue::Cue::LanguageConvention(value) => {
            query.targets.push(QueryTarget::Social);
            query
                .cues
                .push(Cue::LanguageConvention(LanguageConventionCue {
                    expression: value.expression,
                    scope: value.scope,
                }));
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

fn hit(value: CognitiveHit) -> p::Hit {
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
    }
}
