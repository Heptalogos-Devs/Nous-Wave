use super::{BoundQuery, CognitiveContributors, QueryPlan, planned_lanes};
use crate::CognitiveRuntimeService;
use nous_core::*;
use std::collections::{BTreeMap, HashMap};

fn intersect<T: Clone + PartialEq>(parent: &[T], child: &[T]) -> Option<Vec<T>> {
    if parent.is_empty() {
        return Some(child.to_vec());
    }
    if child.is_empty() {
        return Some(parent.to_vec());
    }
    let values: Vec<_> = child
        .iter()
        .filter(|value| parent.contains(value))
        .cloned()
        .collect();
    (!values.is_empty()).then_some(values)
}

fn interval(
    parent: Option<TimeInterval>,
    child: Option<TimeInterval>,
) -> Option<Option<TimeInterval>> {
    match (parent, child) {
        (Some(a), Some(b)) => {
            let start = a.start.into_iter().chain(b.start).max();
            let end = a.end.into_iter().chain(b.end).min();
            if start.zip(end).is_some_and(|(start, end)| start >= end) {
                None
            } else {
                Some(Some(TimeInterval { start, end }))
            }
        }
        (a, b) => Some(a.or(b)),
    }
}

fn constraints(parent: &QueryConstraints, child: &QueryConstraints) -> Option<QueryConstraints> {
    let mut result = child.clone();
    result.current_authority = parent.current_authority.max(child.current_authority);
    result.source_classes_include = intersect(
        &parent.source_classes_include,
        &child.source_classes_include,
    )?;
    result.cognitive_roles_include = intersect(
        &parent.cognitive_roles_include,
        &child.cognitive_roles_include,
    )?;
    result.formation_modes_include = intersect(
        &parent.formation_modes_include,
        &child.formation_modes_include,
    )?;
    result.modalities = intersect(&parent.modalities, &child.modalities)?;
    result.evidence_classes = intersect(&parent.evidence_classes, &child.evidence_classes)?;
    for value in &parent.source_classes_exclude {
        if !result.source_classes_exclude.contains(value) {
            result.source_classes_exclude.push(value.clone());
        }
    }
    for value in &parent.entity_requirements {
        if !result.entity_requirements.contains(value) {
            result.entity_requirements.push(value.clone());
        }
    }
    result.occurred = interval(parent.occurred, child.occurred)?;
    result.observed = interval(parent.observed, child.observed)?;
    result.valid = interval(parent.valid, child.valid)?;
    result.formed = interval(parent.formed, child.formed)?;
    result.recorded = interval(parent.recorded, child.recorded)?;
    if parent
        .authority
        .zip(child.authority)
        .is_some_and(|(a, b)| a != b)
    {
        return None;
    }
    result.authority = child.authority.or(parent.authority);
    result.include_suppressed = parent.include_suppressed && child.include_suppressed;
    Some(result)
}

fn inherit_targets(parent: &[QueryTarget], child: &[QueryTarget]) -> Option<Vec<QueryTarget>> {
    if child.is_empty() {
        return Some(parent.to_vec());
    }
    let domains: Vec<_> = parent
        .iter()
        .filter(|target| {
            !matches!(
                target,
                QueryTarget::Exact { .. } | QueryTarget::AnyRelevantCognition
            )
        })
        .collect();
    if domains.is_empty() {
        return Some(child.to_vec());
    }
    let child_domains: Vec<_> = child
        .iter()
        .filter(|target| {
            !matches!(
                target,
                QueryTarget::Exact { .. } | QueryTarget::AnyRelevantCognition
            )
        })
        .collect();
    let scoped_domains = if child_domains.is_empty() {
        domains
    } else {
        child_domains
            .into_iter()
            .filter(|child| {
                domains
                    .iter()
                    .any(|parent| std::mem::discriminant(*parent) == std::mem::discriminant(*child))
            })
            .collect()
    };
    if scoped_domains.is_empty() {
        return None;
    }
    let mut narrowed: Vec<_> = scoped_domains.into_iter().cloned().collect();
    narrowed.extend(
        child
            .iter()
            .filter(|target| matches!(target, QueryTarget::Exact { .. }))
            .cloned(),
    );
    Some(narrowed)
}
fn better_hit(existing: &CognitiveHit, incoming: &CognitiveHit) -> CognitiveHit {
    if incoming.match_evidence.final_score > existing.match_evidence.final_score {
        incoming.clone()
    } else {
        existing.clone()
    }
}

fn leaves(
    node: &CognitiveQueryExpr,
    parent: Option<&CognitiveQueryExpr>,
    output: &mut Vec<Option<CognitiveQueryExpr>>,
) {
    let mut scoped = node.clone();
    let mut eligible = true;
    if let Some(parent) = parent {
        if let Some(value) = constraints(&parent.constraints, &node.constraints) {
            scoped.constraints = value;
        } else {
            eligible = false;
        }
        if let Some(targets) = inherit_targets(&parent.targets, &node.targets) {
            scoped.targets = targets;
        } else {
            eligible = false;
        }
    }
    if node.operation == QueryOperation::Atom {
        scoped.children.clear();
        output.push(eligible.then_some(scoped));
    } else {
        for child in &node.children {
            if eligible {
                leaves(child, Some(&scoped), output);
            } else {
                empty_leaves(child, output);
            }
        }
    }
}
fn empty_leaves(node: &CognitiveQueryExpr, output: &mut Vec<Option<CognitiveQueryExpr>>) {
    if node.operation == QueryOperation::Atom {
        output.push(None);
    } else {
        for child in &node.children {
            empty_leaves(child, output);
        }
    }
}

pub(super) fn combine(
    node: &CognitiveQueryExpr,
    results: &mut impl Iterator<Item = Vec<CognitiveHit>>,
) -> HashMap<CognitiveRef, CognitiveHit> {
    if node.operation == QueryOperation::Atom {
        return results
            .next()
            .unwrap_or_default()
            .into_iter()
            .map(|hit| (hit.reference.clone(), hit))
            .collect();
    }
    let mut children = node.children.iter();
    let mut result = children
        .next()
        .map(|child| combine(child, results))
        .unwrap_or_default();
    for child in children {
        let incoming = combine(child, results);
        if node.operation == QueryOperation::All {
            result.retain(|reference, _| incoming.contains_key(reference));
        }
        for (reference, hit) in incoming {
            if node.operation == QueryOperation::Any || result.contains_key(&reference) {
                result
                    .entry(reference)
                    .and_modify(|existing| {
                        *existing = better_hit(existing, &hit);
                    })
                    .or_insert(hit);
            }
        }
    }
    result
}

fn allocation(total: usize, index: usize, branches: usize) -> usize {
    total / branches + usize::from(index < total % branches)
}

impl CognitiveRuntimeService {
    #[expect(
        clippy::too_many_lines,
        reason = "tree execution keeps fixed allocation, leaf snapshots and diagnostic aggregation in one boundary"
    )]
    pub(super) async fn query_tree(
        &self,
        bound: BoundQuery,
        contributors: CognitiveContributors<'_>,
        plan: QueryPlan,
        output_limit: usize,
    ) -> Result<super::QueryExecution> {
        let mut scopes = Vec::new();
        leaves(&bound.source_query.expression, None, &mut scopes);
        let count = scopes.len();
        let mut result = CognitiveQueryResult {
            query_id: bound.query_id,
            generation: QueryGenerationTrace::default(),
            status: QueryStatus::Complete,
            results: Vec::new(),
            resource_actions: Vec::new(),
            resource_records: Vec::new(),
            degradation: Vec::new(),
            diagnostics: Some(QueryDiagnostics {
                candidate_counts: BTreeMap::from([
                    ("expression_branches".into(), count),
                    (
                        "allocated_validation_budget".into(),
                        plan.final_validation_budget,
                    ),
                ]),
                lane_status: BTreeMap::new(),
                topology_complete: None,
                topology_discarded_mass: None,
                trace: None,
            }),
        };
        let mut outputs = Vec::new();
        let mut snapshots = Vec::new();
        for (index, scope) in scopes.into_iter().enumerate() {
            let validation_budget = allocation(plan.final_validation_budget, index, count);
            if validation_budget == 0 && scope.is_some() {
                result.status = QueryStatus::Partial;
                result.degradation.push(Degradation {
                    code: "expression_budget_exhausted".into(),
                    detail: Some(format!("branch {index} has no allocated validation work")),
                });
            }
            let Some(scope) = scope.filter(|_| validation_budget > 0) else {
                outputs.push(Vec::new());
                continue;
            };
            let mut branch = bound.clone();
            branch.source_query.expression = scope;
            branch.source_query.expression.preferences.clear();
            branch.exact_bindings.retain(|binding| branch.source_query.expression.targets.iter().any(|target| matches!(target, QueryTarget::Exact { reference } if reference == &binding.requested_ref)));
            let local = planned_lanes(&branch.source_query);
            branch.lane_budgets = plan
                .lane_budgets
                .iter()
                .map(|(&lane, &total)| {
                    (
                        lane,
                        if local.contains(&lane) {
                            allocation(total, index, count)
                        } else {
                            0
                        },
                    )
                })
                .collect();
            branch.enabled_lanes = local
                .into_iter()
                .filter(|lane| branch.lane_budget(*lane) > 0)
                .collect();
            if branch.enabled_lanes.is_empty() && !branch.source_query.requests_resources() {
                outputs.push(Vec::new());
                continue;
            }
            let mut branch_plan = plan.clone();
            branch_plan.enabled_lanes = branch.enabled_lanes.clone();
            branch_plan.lane_budgets = branch.lane_budgets.clone();
            branch_plan.candidate_limit = branch.lane_budgets.values().copied().max().unwrap_or(0);
            branch_plan.final_validation_budget = validation_budget;
            branch_plan.topology_nodes = allocation(plan.topology_nodes, index, count);
            branch_plan.resource_limit = allocation(plan.resource_limit, index, count);
            branch_plan.expand_topology =
                branch.enabled_lanes.contains(&EvidenceFamily::TopologyWave);
            let mut value = self
                .query_atom_with_plan(
                    branch.clone(),
                    &contributors,
                    branch_plan,
                    validation_budget,
                )
                .await?;
            snapshots.push(super::types::BoundLeaf {
                ordinal: index,
                bound: branch,
                hits: value
                    .results
                    .iter()
                    .map(super::types::CandidateStamp::from)
                    .collect(),
            });
            result.generation = value.generation;
            if value.status == QueryStatus::Partial {
                result.status = QueryStatus::Partial;
            } else if value.status == QueryStatus::Degraded
                && result.status == QueryStatus::Complete
            {
                result.status = QueryStatus::Degraded;
            }
            result.degradation.append(&mut value.degradation);
            result.resource_actions.append(&mut value.resource_actions);
            if let Some(diagnostics) = value.diagnostics {
                let target = result.diagnostics.as_mut().ok_or_else(|| {
                    Error::Infrastructure("query tree diagnostics missing".into())
                })?;
                for (key, value) in diagnostics.candidate_counts {
                    target
                        .candidate_counts
                        .insert(format!("branch_{index}_{key}"), value);
                }
                for (key, value) in diagnostics.lane_status {
                    target
                        .lane_status
                        .insert(format!("branch_{index}_{key}"), value);
                }
                if let Some(complete) = diagnostics.topology_complete {
                    target.topology_complete =
                        Some(target.topology_complete.unwrap_or(true) && complete);
                }
                if let Some(mass) = diagnostics.topology_discarded_mass {
                    target.topology_discarded_mass =
                        Some(target.topology_discarded_mass.unwrap_or(0.0) + mass);
                }
            }
            outputs.push(value.results);
        }
        finalize(&bound, &mut result, outputs, output_limit);
        Ok(super::QueryExecution {
            bound,
            result,
            leaves: snapshots,
        })
    }
}

fn finalize(
    bound: &BoundQuery,
    result: &mut CognitiveQueryResult,
    outputs: Vec<Vec<CognitiveHit>>,
    output_limit: usize,
) {
    let memberships = outputs
        .iter()
        .map(|hits| {
            hits.iter()
                .map(|hit| hit.reference.clone())
                .collect::<std::collections::HashSet<_>>()
        })
        .collect::<Vec<_>>();
    result.results = combine(&bound.source_query.expression, &mut outputs.into_iter())
        .into_values()
        .collect();
    result.results.sort_by(|a, b| {
        b.match_evidence
            .base_rank_score
            .total_cmp(&a.match_evidence.base_rank_score)
            .then_with(|| a.reference.to_string().cmp(&b.reference.to_string()))
    });
    for (rank, hit) in result.results.iter_mut().enumerate() {
        let mut preferences = Vec::new();
        collect_preferences(
            &bound.source_query.expression,
            &hit.reference,
            &memberships,
            &mut 0,
            &mut preferences,
        );
        hit.match_evidence.baseline_rank = (rank + 1) as u32;
        hit.match_evidence.preference_score =
            super::preferences::score(&preferences, hit, bound.bound_at);
        hit.match_evidence.final_score =
            hit.match_evidence.base_rank_score + hit.match_evidence.preference_score;
    }
    super::preferences::order(&mut result.results);
    result.results.truncate(output_limit);
}

fn collect_preferences(
    node: &CognitiveQueryExpr,
    reference: &CognitiveRef,
    memberships: &[std::collections::HashSet<CognitiveRef>],
    leaf: &mut usize,
    preferences: &mut Vec<QueryPreference>,
) -> bool {
    let mut local = Vec::new();
    let matched = if node.operation == QueryOperation::Atom {
        let matched = memberships[*leaf].contains(reference);
        *leaf += 1;
        matched
    } else {
        let values: Vec<_> = node
            .children
            .iter()
            .map(|child| collect_preferences(child, reference, memberships, leaf, &mut local))
            .collect();
        if node.operation == QueryOperation::All {
            values.into_iter().all(|value| value)
        } else {
            values.into_iter().any(|value| value)
        }
    };
    if matched {
        preferences.extend(node.preferences.clone());
        preferences.extend(local);
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn branch_allocation_preserves_total_and_scopes_cannot_broaden_filters() {
        for count in 1..65 {
            assert_eq!(
                (0..count)
                    .map(|index| allocation(48, index, count))
                    .sum::<usize>(),
                48
            );
        }
        let parent = QueryConstraints {
            current_authority: CurrentAuthorityNeed::Required,
            source_classes_include: vec![SourceClass::from("web".to_owned())],
            ..Default::default()
        };
        let child = QueryConstraints {
            source_classes_include: vec![SourceClass::from("file".to_owned())],
            ..Default::default()
        };
        assert!(constraints(&parent, &child).is_none());
        assert_eq!(
            constraints(&parent, &QueryConstraints::default())
                .unwrap()
                .source_classes_include,
            parent.source_classes_include
        );
        assert_eq!(
            constraints(&parent, &QueryConstraints::default())
                .unwrap()
                .current_authority,
            CurrentAuthorityNeed::Required
        );
        assert_eq!(
            constraints(
                &QueryConstraints::default(),
                &QueryConstraints {
                    current_authority: CurrentAuthorityNeed::Required,
                    ..Default::default()
                }
            )
            .unwrap()
            .current_authority,
            CurrentAuthorityNeed::Required
        );
        let exact = QueryTarget::Exact {
            reference: CognitiveRef::Artifact(ArtifactId::new()),
        };
        let narrowed =
            inherit_targets(&[QueryTarget::Memory], std::slice::from_ref(&exact)).unwrap();
        assert!(
            narrowed
                .iter()
                .any(|target| matches!(target, QueryTarget::Memory))
        );
        assert!(inherit_targets(&[QueryTarget::Memory], &[QueryTarget::Evidence, exact]).is_none());
        let narrowed = inherit_targets(
            &[
                QueryTarget::Memory,
                QueryTarget::Schema,
                QueryTarget::Episode,
                QueryTarget::Journal,
            ],
            &[QueryTarget::Schema, QueryTarget::Journal],
        )
        .unwrap();
        assert_eq!(
            narrowed
                .iter()
                .filter_map(QueryTarget::domain_name)
                .collect::<Vec<_>>(),
            vec!["schema", "journal"]
        );
        assert!(inherit_targets(&[QueryTarget::Episode], &[QueryTarget::Journal]).is_none());
    }
}
