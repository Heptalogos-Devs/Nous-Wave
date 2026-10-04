mod rerank;
use super::*;
use std::io::Write;

#[derive(Default, Deserialize, Serialize)]
struct InputContext {
    #[serde(default)]
    tag_labels: Vec<String>,
    #[serde(default)]
    current_event_ids: Vec<String>,
    source: Option<String>,
}
#[derive(Deserialize)]
struct Query {
    query_id: String,
    #[serde(default = "default_suite")]
    suite: String,
    subject: String,
    category: String,
    text: String,
    as_of: DateTime<Utc>,
    time_axis: Option<String>,
    oracle: serde_json::Value,
    #[serde(default)]
    required_paths: Vec<serde_json::Value>,
    #[serde(default)]
    session_oracle: Vec<String>,
    #[serde(default)]
    unresolved_evidence: Vec<String>,
    original_question_date: Option<DateTime<Utc>>,
    #[serde(default)]
    input_context: InputContext,
}
#[derive(Deserialize)]
struct Queries {
    queries: Vec<Query>,
}

pub(super) async fn run(
    runtime: &NousRuntime,
    clock: &ImportClock,
    scenarios: &[Scenario],
    corpus: &std::path::Path,
    root: &std::path::Path,
    state: &mut State,
    state_path: &std::path::Path,
    vectors_path: &std::path::Path,
) -> Result<()> {
    let mut queries: Queries =
        serde_json::from_slice(&std::fs::read(corpus.join("queries.json")).map_err(failure)?)
            .map_err(failure)?;
    queries
        .queries
        .sort_by_key(|q| (q.subject.clone(), q.as_of, q.query_id.clone()));
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(corpus.join("manifest.json")).map_err(failure)?)
            .map_err(failure)?;
    let cache = load_vector_cache(vectors_path)?;
    let vectors = cache
        .vectors
        .into_iter()
        .map(|v| (v.text, v.vector))
        .collect::<BTreeMap<_, _>>();
    // No corpus information is admitted beyond the query's recorded as-of.
    // A fresh prefix root is required; result replay is intentionally explicit.
    if root.join("benchmark.jsonl").exists() {
        return Err(Error::Conflict(
            "benchmark output already exists; use a separate prefix root".into(),
        ));
    }
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("benchmark.jsonl"))
        .map_err(failure)?;
    let mut rerank = rerank::Bridge::open().await?;
    let profiles = [
        nous_runtime::CognitiveProfile::BaselineRrf,
        nous_runtime::CognitiveProfile::NousNodePotential,
        nous_runtime::CognitiveProfile::VcpDtsc,
        nous_runtime::CognitiveProfile::VcpRiverMemo,
    ];
    for query in queries.queries {
        let scenario = scenarios
            .iter()
            .find(|s| s.scenario_id == query.subject)
            .ok_or_else(|| Error::Invalid("query Subject absent".into()))?;
        let mut prefix = scenario.clone();
        prefix.events.retain(|e| e.recorded_at <= query.as_of);
        if prefix.events.len()
            < state
                .events
                .keys()
                .filter(|id| id.starts_with(&format!("{}-", query.subject)))
                .count()
        {
            return Err(Error::Conflict(
                "prefix root already contains future events".into(),
            ));
        }
        import_scenario(runtime, clock, prefix, state, state_path).await?;
        let subject = state.subjects[&query.subject];
        prepare_material(runtime, subject, &vectors, &cache.config).await?;
        clock.set(query.as_of);
        let vector = vectors
            .get(&query.text)
            .ok_or_else(|| Error::Unavailable("query embedding cache incomplete".into()))?;
        let requests = vec![nous_retrieval::QueryEmbedding {
            text: query.text.clone(),
            output: nous_retrieval::TextEmbeddingOutput {
                vector: vector.clone(),
                space: cache.config.space.clone(),
                producer: cache.config.producer.clone(),
            },
        }];
        for profile in profiles {
            runtime
                .configuration
                .set_system_override(
                    OperationId::new(),
                    nous_runtime::COGNITIVE_PROFILE.path(),
                    serde_json::to_value(profile).map_err(failure)?,
                )
                .await?;
            let request = request_for(&query, subject, state)?;
            let start = std::time::Instant::now();
            let execution = nous_retrieval::with_query_material(
                requests.clone(),
                runtime.execute_query(request, rerank.as_ref().map(|_| 64)),
            )
            .await?;
            let snapshot = runtime.serving.publisher.snapshot_for(subject);
            let returned = mapped_results(&execution.result, state);
            let row = serde_json::json!({"query_id":query.query_id,"suite":query.suite,"category":query.category,"profile":profile.id(),"profile_digest":profile.digest(),"corpus_digest":state.corpus_digest,"query_set_digest":manifest["queries_sha256"],"embedding_space":cache.config.space,"embedding_producer":cache.config.producer,"config_digest":execution.bound.config_snapshot.effective_digest,"cognitive_config_subset_digest":execution.bound.config_snapshot.digest_for(&["retrieval.cognitive.profile","retrieval.vcp.assets","retrieval.vcp.query","retrieval.vcp.readout"])? ,"authority_watermark":execution.bound.bound_at_authority_seq,"as_of":query.as_of,"clock_source":"research-serial-prefix","track":"controlled-kernel-no-rerank","oracle":query.oracle,"required_paths":query.required_paths,"input_context":query.input_context,"subject":query.subject,"session_oracle":query.session_oracle,"unresolved_evidence":query.unresolved_evidence,"original_question_date":query.original_question_date,"default_same_subject_oracle":{"grade":0,"reason":"distractor"},"returned":returned,"lane_diagnostics":execution.result.diagnostics,"degradation":execution.result.degradation,"latency_ms":start.elapsed().as_secs_f64()*1000.0,"serving_generations":{"dense":snapshot.dense.iter().map(|g|g.generation_id).collect::<Vec<_>>(),"native":snapshot.topology.as_ref().map(|g|g.generation_id),"vcp":snapshot.vcp.as_ref().map(|g|g.generation_id)},"provider_usage":{"query_generation":"precomputed_real_provider_cache","rerank_calls":0},"error":null});
            writeln!(output, "{}", serde_json::to_string(&row).map_err(failure)?)
                .map_err(failure)?;
            output.flush().map_err(failure)?;
            if let Some(bridge) = rerank.as_mut()
                && matches!(
                    profile,
                    nous_runtime::CognitiveProfile::BaselineRrf
                        | nous_runtime::CognitiveProfile::NousNodePotential
                )
            {
                write_reranked(
                    runtime,
                    &query.text,
                    execution,
                    bridge,
                    row.clone(),
                    &mut output,
                    state,
                    start,
                )
                .await?;
            }
            println!(
                "query={} profile={} hits={}",
                query.query_id,
                profile.id(),
                returned.as_array().map_or(0, Vec::len)
            );
        }
    }
    Ok(())
}

async fn prepare_material(
    runtime: &NousRuntime,
    subject: SubjectId,
    vectors: &BTreeMap<String, Vec<f32>>,
    config: &nous_retrieval::StoredEmbeddingConfig,
) -> Result<()> {
    loop {
        let needs = runtime.serving.embedding_needs(subject, 256).await?;
        if needs.is_empty() {
            break;
        }
        for need in needs {
            let vector = vectors.get(&need.text).ok_or_else(|| {
                Error::Unavailable("prefix embedding cache missing source text".into())
            })?;
            runtime
                .serving
                .commit_embedding(
                    subject,
                    need.reference,
                    need.text,
                    &config.space.space_hash,
                    &config.producer.signature_hash,
                    vector.clone(),
                )
                .await?;
        }
    }
    Ok(())
}
fn request_for(query: &Query, subject: SubjectId, state: &State) -> Result<CognitiveQuery> {
    let mut constraints = QueryConstraints::default();
    if let Some(axis) = &query.time_axis {
        // Temporal queries explicitly name an RFC3339 timestamp. Parse only
        // query input; relevance labels never enter candidate construction.
        let instant = query
            .text
            .char_indices()
            .filter(|(_, c)| *c == '2')
            .find_map(|(i, _)| {
                query
                    .text
                    .get(i..i + 20)
                    .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
                    .map(|date| date.with_timezone(&Utc))
            })
            .ok_or_else(|| Error::Invalid("temporal query lacks explicit timestamp".into()))?;
        let interval = TimeInterval {
            start: Some(instant),
            end: Some(instant + chrono::Duration::seconds(1)),
        };
        match axis.as_str() {
            "occurred" => constraints.occurred = Some(interval),
            "observed" => constraints.observed = Some(interval),
            "valid" => constraints.valid = Some(interval),
            "formed" => constraints.formed = Some(interval),
            "recorded" => constraints.recorded = Some(interval),
            _ => {}
        }
    }
    let mut cues = vec![Cue::Text(TextCue {
        text: query.text.clone(),
    })];
    for label in &query.input_context.tag_labels {
        let tag = state
            .tags
            .get(&format!("{}:{label}", query.subject))
            .ok_or_else(|| {
                Error::Invalid("query input Tag is absent from current prefix".into())
            })?;
        cues.push(Cue::Tag(TagCue { tag: *tag }));
    }
    let current_refs = query
        .input_context
        .current_event_ids
        .iter()
        .map(|event| {
            if !event.starts_with(&format!("{}-", query.subject)) {
                return Err(Error::Invalid(
                    "current situation reference crosses Subject".into(),
                ));
            }
            state
                .events
                .get(event)
                .map(|receipt| CognitiveRef::MemoryRevision(receipt.revision))
                .ok_or_else(|| {
                    Error::Invalid("current situation event is absent from prefix".into())
                })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(CognitiveQuery {
        api_version: API_VERSION,
        subject,
        session: None,
        situation: SituationDescriptor {
            consumer: Some("research-cognitive-recall".into()),
            current_refs,
            current_objects: Vec::new(),
        },
        expression: CognitiveQueryExpr {
            targets: vec![QueryTarget::Memory],
            cues,
            constraints,
            ..Default::default()
        },
        exploration: ExplorationIntent::BoundedAssociative,
        resources: Default::default(),
        result_need: ResultNeed {
            limit: 10,
            ..Default::default()
        },
        effort: Default::default(),
        capabilities: Default::default(),
        diagnostics: DiagnosticsRequest::Full,
    })
}

fn mapped_results(result: &CognitiveQueryResult, state: &State) -> serde_json::Value {
    serde_json::json!(
        result
            .results
            .iter()
            .take(10)
            .map(|hit| {
                let event = state
                    .events
                    .iter()
                    .find(|(_, receipt)| {
                        CognitiveRef::MemoryRevision(receipt.revision) == hit.reference
                    })
                    .map(|(id, _)| id.clone());
                serde_json::json!({"event_id":event,"hit":hit})
            })
            .collect::<Vec<_>>()
    )
}

async fn write_reranked(
    runtime: &NousRuntime,
    text: &str,
    execution: nous_runtime::QueryExecution,
    bridge: &mut rerank::Bridge,
    row: serde_json::Value,
    output: &mut std::fs::File,
    state: &State,
    start: std::time::Instant,
) -> Result<()> {
    let subject = execution.bound.source_query.subject;
    let profile = row["profile"]
        .as_str()
        .ok_or_else(|| Error::Internal("profile missing".into()))?
        .to_owned();
    let rank_start = std::time::Instant::now();
    let (result, usage) = rerank::apply(runtime, subject, text, execution, bridge).await?;
    let mut reranked = row;
    reranked["profile"] = serde_json::json!(format!("{}+model-rerank", profile));
    reranked["track"] = serde_json::json!("controlled-kernel-production-rerank");
    reranked["returned"] = mapped_results(&result, state);
    reranked["lane_diagnostics"] = serde_json::to_value(result.diagnostics).map_err(failure)?;
    reranked["degradation"] = serde_json::to_value(result.degradation).map_err(failure)?;
    reranked["latency_ms"] = serde_json::json!(start.elapsed().as_secs_f64() * 1000.0);
    reranked["rerank_and_final_validation_ms"] =
        serde_json::json!(rank_start.elapsed().as_secs_f64() * 1000.0);
    reranked["provider_usage"] = serde_json::json!({"query_generation":"precomputed_real_provider_cache","rerank_calls":if usage.get("skipped").is_some(){0}else{1},"rerank_throttle_ms":6000,"rerank":usage});
    writeln!(
        output,
        "{}",
        serde_json::to_string(&reranked).map_err(failure)?
    )
    .map_err(failure)?;
    output.flush().map_err(failure)?;
    Ok(())
}
