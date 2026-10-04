//! Explicit research harness; semantic writes use normal owners and an injected clock.
mod cognitive_benchmark;
use chrono::{DateTime, Utc};
use nous_core::*;
use nous_kernel::{NousRuntime, RuntimeOptions};
use nous_material::{
    ObservationInput, ObservationMaterial, OccurrenceDescriptor, RuntimeDirective,
};
use nous_memory::{
    CognitiveRole, CreateTagRequest, EvidenceLocator, EvidenceRef, ExplicitMemoryInput,
    FormationMode, RevisionIntent, RevisionSupport, SupportRole,
};
use nous_retrieval::ServingOptions;
use nous_runtime::CognitiveClock;
use postgresql_embedded::{PostgreSQL, SettingsBuilder, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Clone, Deserialize)]
struct Scenario {
    scenario_id: String,
    #[serde(default = "default_suite")]
    suite: String,
    events: Vec<Event>,
}
#[derive(Clone, Deserialize)]
struct Event {
    event_id: String,
    occurred_at: DateTime<Utc>,
    observed_at: DateTime<Utc>,
    formed_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
    valid_time: ValidTime,
    session: String,
    entities: Vec<String>,
    tags: Vec<String>,
    renders: Vec<Render>,
    relations: Vec<Relation>,
}
#[derive(Clone, Deserialize)]
struct ValidTime {
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
}
#[derive(Clone, Deserialize)]
struct Render {
    text: String,
}
#[derive(Clone, Deserialize)]
struct Relation {
    kind: String,
    target: String,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct State {
    subjects: BTreeMap<String, SubjectId>,
    #[serde(skip_serializing)]
    events: BTreeMap<String, EventReceipt>,
    sessions: BTreeMap<String, SessionId>,
    tags: BTreeMap<String, TagId>,
    corpus_digest: String,
    associations: BTreeMap<String, AssociationEvidenceId>,
}
#[derive(Serialize, Deserialize)]
struct EventReceipt {
    occurrence: OccurrenceId,
    memory: MemoryId,
    revision: MemoryRevisionId,
    formed_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
}

/// The first semantic now() of a mutation supplies formation start; subsequent
/// reads supply commit time. This harness is serial and never drives live Core.
struct ImportClock {
    state: Mutex<(DateTime<Utc>, Option<DateTime<Utc>>)>,
}
impl ImportClock {
    fn new() -> Self {
        Self {
            state: Mutex::new((DateTime::UNIX_EPOCH, None)),
        }
    }
    fn set(&self, instant: DateTime<Utc>) {
        *self.state.lock().unwrap() = (instant, None);
    }
    fn mutation(&self, formed: DateTime<Utc>, recorded: DateTime<Utc>) {
        *self.state.lock().unwrap() = (formed, Some(recorded));
    }
}
impl CognitiveClock for ImportClock {
    fn now(&self, _subject: SubjectId) -> DateTime<Utc> {
        let mut state = self.state.lock().unwrap();
        let instant = state.0;
        if let Some(recorded) = state.1.take() {
            state.0 = recorded;
        }
        instant
    }
}
fn default_suite() -> String {
    "nous-cognitive-cc0-v1".into()
}
fn id(key: &str) -> uuid::Uuid {
    let digest = blake3::hash(key.as_bytes());
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest.as_bytes()[..16]);
    uuid::Uuid::from_bytes(bytes)
}
fn operation(key: &str) -> OperationId {
    OperationId(id(key))
}
fn failure(error: impl std::fmt::Display) -> Error {
    Error::Infrastructure(error.to_string())
}
fn append_receipt(path: &std::path::Path, key: &str, receipt: &EventReceipt) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.with_extension("events.jsonl"))
        .map_err(failure)?;
    let mut line = serde_json::to_vec(&(key, receipt)).map_err(failure)?;
    line.push(b'\n');
    file.write_all(&line).map_err(failure)?;
    file.sync_data().map_err(failure)
}
fn load_receipts(path: &std::path::Path, state: &mut State) -> Result<()> {
    let journal = path.with_extension("events.jsonl");
    if !journal.exists() {
        use std::io::Write;
        let staging = journal.with_extension("staging.jsonl");
        let mut file = std::fs::File::create(&staging).map_err(failure)?;
        for (key, receipt) in &state.events {
            let mut line = serde_json::to_vec(&(key, receipt)).map_err(failure)?;
            line.push(b'\n');
            file.write_all(&line).map_err(failure)?;
        }
        file.sync_all().map_err(failure)?;
        std::fs::rename(staging, &journal).map_err(failure)?;
    }
    let bytes = std::fs::read(&journal).map_err(failure)?;
    let complete = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |i| i + 1);
    for line in bytes[..complete]
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let (key, receipt): (String, EventReceipt) =
            serde_json::from_slice(line).map_err(failure)?;
        state.events.insert(key, receipt);
    }
    if complete != bytes.len() {
        std::fs::OpenOptions::new()
            .write(true)
            .open(journal)
            .map_err(failure)?
            .set_len(complete as u64)
            .map_err(failure)?;
    }
    Ok(())
}
fn save(path: &std::path::Path, state: &State) -> Result<()> {
    let staging = path.with_extension("staging.json");
    std::fs::write(&staging, serde_json::to_vec_pretty(state).map_err(failure)?)
        .map_err(failure)?;
    std::fs::rename(staging, path).map_err(failure)
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let corpus = PathBuf::from(args.next().ok_or_else(|| {
        Error::Invalid("usage: cognitive-import <corpus-root> <ignored-run-root>".into())
    })?);
    let root = PathBuf::from(
        args.next()
            .ok_or_else(|| Error::Invalid("research run root required".into()))?,
    );
    let embedding_path = args.next().map(PathBuf::from);
    let vectors_path = args.next().map(PathBuf::from);
    let benchmark = args.next().is_some_and(|value| value == "benchmark");
    let root = std::path::absolute(root).map_err(failure)?;
    let research = std::path::absolute("data/research").map_err(failure)?;
    if !root.starts_with(research) {
        return Err(Error::Invalid(
            "run root must be under data/research".into(),
        ));
    }
    std::fs::create_dir_all(&root).map_err(failure)?;
    let runtime_path = std::env::var_os("NOUS_WAVE_POSTGRES_RUNTIME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/runtime/installed/postgresql"));
    if !runtime_path
        .join(if cfg!(windows) {
            "bin/postgres.exe"
        } else {
            "bin/postgres"
        })
        .is_file()
    {
        return Err(Error::Unavailable(
            "installed PostgreSQL runtime required".into(),
        ));
    }
    verify_corpus(&corpus)?;
    let settings = SettingsBuilder::new()
        .version(VersionReq::parse("=18.6.0").map_err(failure)?)
        .trust_installation_dir(true)
        .host("127.0.0.1")
        .port(0)
        .username("postgres")
        .password("cognitive-research-local")
        .installation_dir(runtime_path)
        .data_dir(root.join("postgres"))
        .password_file(root.join("postgres.pgpass"))
        .timeout(Some(std::time::Duration::from_secs(30)))
        .temporary(false)
        .build();
    let mut postgres = PostgreSQL::new(settings);
    postgres.setup().await.map_err(failure)?;
    postgres.start().await.map_err(failure)?;
    if !postgres
        .database_exists("cognitive_recall")
        .await
        .map_err(failure)?
    {
        postgres
            .create_database("cognitive_recall")
            .await
            .map_err(failure)?;
    }
    let clock = Arc::new(ImportClock::new());
    let runtime = NousRuntime::open_with_clock(RuntimeOptions {
        postgres_url: postgres.settings().url("cognitive_recall"), max_connections: 4, acquire_timeout_ms: 15000,
        object_root: root.join("objects").to_string_lossy().into_owned(),
        serving_options: ServingOptions { root: root.join("serving"), lexical: false, dense: false, topology: false, memory_enabled: true },
        embedding: None, stored_embedding: None, core_descriptors: vec![],
        deployment_document: serde_json::json!({"serving":{"lexical":{"enabled":benchmark},"dense":{"enabled":benchmark},"topology":{"enabled":benchmark}}}),
    }, clock.clone()).await?;
    if let Some(path) = &embedding_path {
        let config =
            serde_json::from_slice(&std::fs::read(path).map_err(failure)?).map_err(failure)?;
        runtime.serving.initialize_embedding(config)?;
    }
    let state_path = root.join("import-state.json");
    let mut state = load_import_state(&corpus, &state_path)?;
    let scenarios = load_scenarios(&corpus)?;
    if benchmark {
        let path = vectors_path
            .as_ref()
            .ok_or_else(|| Error::Invalid("benchmark requires real embedding cache".into()))?;
        Box::pin(cognitive_benchmark::run(
            &runtime,
            &clock,
            &scenarios,
            &corpus,
            &root,
            &mut state,
            &state_path,
            path,
        ))
        .await?;
    } else {
        for scenario in scenarios {
            import_scenario(&runtime, &clock, scenario, &mut state, &state_path).await?;
        }
        if let Some(path) = &vectors_path {
            commit_vectors(&runtime, &root, path).await?;
        }
        if embedding_path.is_some() {
            export_needs(&runtime, &state, &root).await?;
        }
        println!(
            "imported subjects={} events={} clock=serial-formation-start/commit",
            state.subjects.len(),
            state.events.len()
        );
    }
    runtime.store.pool().close().await;
    postgres.stop().await.map_err(failure)?;
    Ok(())
}

async fn import_scenario(
    runtime: &NousRuntime,
    clock: &ImportClock,
    mut scenario: Scenario,
    state: &mut State,
    state_path: &std::path::Path,
) -> Result<()> {
    clock.set(
        scenario
            .events
            .iter()
            .map(|e| e.observed_at)
            .min()
            .ok_or_else(|| Error::Invalid("empty scenario".into()))?,
    );
    let subject = SubjectId(id(&format!("cc0-v1:{}", scenario.scenario_id)));
    if !runtime.store.subject_exists(subject).await? {
        runtime.subjects.create_subject(nous_subject::CreateSubject {
        subject_id: Some(subject), operation_id: operation(&format!("{}:create", scenario.scenario_id)),
        cognitive_seed: nous_subject::CognitiveSeedInput { text: "schema_version = 1".into(), format: nous_subject::COGNITIVE_SEED_FORMAT.into(), provenance: serde_json::json!({"suite":scenario.suite}) },
        metadata: serde_json::json!({"corpus":scenario.suite,"scenario":scenario.scenario_id}), capabilities: None,
    }).await?;
    }
    state.subjects.insert(scenario.scenario_id.clone(), subject);
    let mut tags = BTreeMap::new();
    let memory = runtime.require_memory()?;
    for label in scenario
        .events
        .iter()
        .flat_map(|e| &e.tags)
        .collect::<std::collections::BTreeSet<_>>()
    {
        let tag = memory
            .create_tag(
                subject,
                CreateTagRequest {
                    operation_id: operation(&format!("{}:tag:{label}", scenario.scenario_id)),
                    label: label.clone(),
                    description: None,
                    kind_hint: None,
                    origin: "explicit".into(),
                },
            )
            .await?;
        state
            .tags
            .insert(format!("{}:{label}", scenario.scenario_id), tag.tag_id);
        tags.insert(label.clone(), tag.tag_id);
    }
    scenario.events.sort_by_key(|event| event.formed_at);
    for event in &scenario.events {
        if let Some(receipt) = state.events.get(&event.event_id) {
            let revision = memory.revision(subject, receipt.revision).await?;
            if revision.revision.formed_at != event.formed_at
                || revision.revision.recorded_at != event.recorded_at
            {
                return Err(Error::Conflict(
                    "reopened owner timestamps disagree with oracle".into(),
                ));
            }
            continue;
        }
        clock.set(event.observed_at);
        let session = if let Some(session) = state.sessions.get(&event.session) {
            *session
        } else {
            let session = runtime
                .cognition
                .open_session(subject, serde_json::json!({"corpus_session":event.session}))
                .await?
                .session_id;
            state.sessions.insert(event.session.clone(), session);
            save(state_path, state)?;
            session
        };
        let accepted = runtime.material.record_observation_once(ObservationInput {
            subject, session: Some(session), occurrence: OccurrenceDescriptor {
                source_class: SourceClass::Message, external_object_ref: Some(ObjectRef::new(format!("object:cc0:{}",event.event_id))?),
                occurred_time: TemporalExtent::Instant { at: event.occurred_at }, observed_at: Some(event.observed_at),
                conversation_ref: None, actor_entity_ref: None, context: serde_json::json!({"event_id":event.event_id,"suite":scenario.suite,"clock_source":"research-serial-clock"}),
            }, material: ObservationMaterial::InlineText { text: event.renders.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join("\n"), media_type: "text/plain".into() }, entities: Vec::new(), runtime: RuntimeDirective::default(),
        }, Some(id(&format!("{}:observe", event.event_id)))).await?;
        if accepted.occurrence.observed_at != event.observed_at
            || accepted.occurrence.occurred_time
                != (TemporalExtent::Instant {
                    at: event.occurred_at,
                })
        {
            return Err(Error::Conflict(
                "observation time axes disagree with oracle".into(),
            ));
        }
        let input = memory_input(event, subject, accepted.occurrence.occurrence_id, &tags)?;
        clock.mutation(event.formed_at, event.recorded_at);
        let view = commit_memory(runtime, clock, event, state, input).await?;
        if view.revision.formed_at != event.formed_at
            || view.revision.recorded_at != event.recorded_at
        {
            return Err(Error::Conflict(format!(
                "owner clock disagrees for {}",
                event.event_id
            )));
        }
        state.events.insert(
            event.event_id.clone(),
            EventReceipt {
                occurrence: accepted.occurrence.occurrence_id,
                memory: view.object.memory_id,
                revision: view.revision.memory_revision_id,
                formed_at: view.revision.formed_at,
                recorded_at: view.revision.recorded_at,
            },
        );
        append_receipt(state_path, &event.event_id, &state.events[&event.event_id])?;
    }
    save(state_path, state)?;
    import_associations(runtime, clock, &scenario, subject, state, state_path).await?;
    println!(
        "scenario={} events={} verified owner timestamps",
        scenario.scenario_id,
        scenario.events.len()
    );
    Ok(())
}

fn memory_input(
    event: &Event,
    subject: SubjectId,
    occurrence: OccurrenceId,
    tags: &BTreeMap<String, TagId>,
) -> Result<ExplicitMemoryInput> {
    Ok(ExplicitMemoryInput {
        producer: None,
        operation_id: operation(&format!("{}:form", event.event_id)),
        subject,
        cognitive_role: CognitiveRole::Declarative,
        formation_mode: FormationMode::Grounded,
        grounding_occurrence_id: Some(occurrence),
        semantic_role: "fact".into(),
        representation_text: event
            .renders
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
        title: Some(event.event_id.clone()),
        supports: vec![RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: occurrence,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        })],
        aboutness: event
            .entities
            .iter()
            .map(|name| EntityRef::new(format!("entity:{name}")))
            .collect::<Result<Vec<_>>>()?,
        tags: event.tags.iter().map(|label| tags[label]).collect(),
        valid_time: TemporalExtent::Interval {
            start: Some(event.valid_time.start),
            end: event.valid_time.end,
        },
        epistemic_class: EpistemicClass::Observed,
    })
}

async fn commit_memory(
    runtime: &NousRuntime,
    clock: &ImportClock,
    event: &Event,
    state: &State,
    input: ExplicitMemoryInput,
) -> Result<nous_memory::MemoryView> {
    let subject = input.subject;
    Ok(
        if let Some(previous) = event.relations.iter().find(|r| r.kind == "supersedes") {
            let prior = state
                .events
                .get(&previous.target)
                .ok_or_else(|| Error::Invalid("update precedes its source".into()))?;
            let current = runtime
                .require_memory()?
                .memory(subject, prior.memory, None)
                .await?;
            // Reading the head above consumes semantic time, so arm only at
            // the actual mutation boundary.
            clock.mutation(event.formed_at, event.recorded_at);
            runtime
                .require_memory()?
                .revise_memory(nous_memory::ReviseMemoryInput {
                    producer: None,
                    operation_id: input.operation_id,
                    subject,
                    memory_id: prior.memory,
                    expected_object_epoch: current.object.object_epoch,
                    intent: RevisionIntent::Correct,
                    formation_mode: input.formation_mode,
                    grounding_occurrence_id: input.grounding_occurrence_id,
                    semantic_role: input.semantic_role,
                    representation_text: input.representation_text,
                    title: input.title,
                    supports: input.supports,
                    aboutness: input.aboutness,
                    valid_time: input.valid_time,
                    epistemic_class: input.epistemic_class,
                })
                .await?
        } else {
            runtime.require_memory()?.form_memory(input).await?
        },
    )
}

async fn import_associations(
    runtime: &NousRuntime,
    clock: &ImportClock,
    scenario: &Scenario,
    subject: SubjectId,
    state: &mut State,
    state_path: &std::path::Path,
) -> Result<()> {
    for event in &scenario.events {
        clock.set(event.recorded_at);
        for relation in event.relations.iter().filter(|r| r.kind != "supersedes") {
            let key = format!("{}:{}:{}", event.event_id, relation.kind, relation.target);
            if state.associations.contains_key(&key) {
                continue;
            }
            let from = &state.events[&event.event_id];
            let to = &state.events[&relation.target];
            let evidence = EvidenceRef {
                occurrence_id: from.occurrence,
                locator: EvidenceLocator::WholeOccurrence,
                support_role: SupportRole::Direct,
            };
            let association = runtime
                .require_memory()?
                .create_association(
                    nous_memory::CreateAssociationRequest {
                        operation_id: operation(&key),
                        from: CognitiveRef::MemoryRevision(from.revision),
                        to: CognitiveRef::MemoryRevision(to.revision),
                        relation_kind: if relation.kind == "recalls_precursor" {
                            "assoc.sequence"
                        } else {
                            "assoc.related"
                        }
                        .into(),
                        polarity: nous_memory::AssociationPolarity::Positive,
                        support_class: nous_memory::AssociationSupportClass::SourceEvidence,
                        supports: vec![nous_memory::AssociationSupport::Revision(
                            RevisionSupport::Evidence(evidence),
                        )],
                        producer_signature_id: None,
                        valid_time: TemporalExtent::Interval {
                            start: Some(event.occurred_at),
                            end: None,
                        },
                    },
                    subject,
                )
                .await?;
            state
                .associations
                .insert(key, association.association_evidence_id);
            save(state_path, state)?;
        }
    }
    Ok(())
}

fn verify_corpus(corpus: &std::path::Path) -> Result<()> {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(corpus.join("manifest.json")).map_err(failure)?)
            .map_err(failure)?;
    let files = manifest["scenario_files"]
        .as_array()
        .ok_or_else(|| Error::Invalid("scenario manifest missing".into()))?;
    for file in files {
        let path = file["path"]
            .as_str()
            .ok_or_else(|| Error::Invalid("scenario path missing".into()))?;
        let bytes = std::fs::read(corpus.join(path)).map_err(failure)?;
        if Some(
            Sha256::digest(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
                .as_str(),
        ) != file["sha256"].as_str()
        {
            return Err(Error::Conflict(
                "scenario digest disagrees with frozen manifest".into(),
            ));
        }
    }
    let query_bytes = std::fs::read(corpus.join("queries.json")).map_err(failure)?;
    let query_digest = Sha256::digest(&query_bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if Some(query_digest.as_str()) != manifest["queries_sha256"].as_str() {
        return Err(Error::Conflict(
            "query digest disagrees with frozen manifest".into(),
        ));
    }
    Ok(())
}

async fn export_needs(runtime: &NousRuntime, state: &State, root: &std::path::Path) -> Result<()> {
    let mut needs = Vec::new();
    for subject in state.subjects.values() {
        // The owner has a bounded page. Uncommitted pages are not silently
        // treated as a complete export when this corpus outgrows that bound.
        let page = runtime.serving.embedding_needs(*subject, 256).await?;
        if page.len() == 256 {
            println!("subject embedding page bounded at 256; commit then export next page");
        }
        for need in page {
            needs.push(
                serde_json::json!({"subject":subject,"reference":need.reference,"text":need.text}),
            );
        }
    }
    std::fs::write(
        root.join("embedding-needs.json"),
        serde_json::to_vec_pretty(&needs).map_err(failure)?,
    )
    .map_err(failure)?;
    println!("exported embedding needs={}", needs.len());
    Ok(())
}

#[derive(Deserialize)]
struct Need {
    subject: SubjectId,
    reference: CognitiveRef,
    text: String,
}
#[derive(Deserialize)]
struct CachedVectors {
    config: nous_retrieval::StoredEmbeddingConfig,
    #[serde(default)]
    vectors: Vec<CachedVector>,
    #[serde(default)]
    vector_files: Vec<PathBuf>,
}
#[derive(Deserialize)]
struct CachedVector {
    text: String,
    vector: Vec<f32>,
}
fn load_vector_cache(path: &std::path::Path) -> Result<CachedVectors> {
    let mut cache: CachedVectors =
        serde_json::from_slice(&std::fs::read(path).map_err(failure)?).map_err(failure)?;
    let parent = path
        .parent()
        .ok_or_else(|| Error::Invalid("cache parent missing".into()))?;
    for part in &cache.vector_files {
        let vectors: Vec<CachedVector> =
            serde_json::from_slice(&std::fs::read(parent.join(part)).map_err(failure)?)
                .map_err(failure)?;
        cache.vectors.extend(vectors);
    }
    Ok(cache)
}
async fn commit_vectors(
    runtime: &NousRuntime,
    root: &std::path::Path,
    path: &std::path::Path,
) -> Result<()> {
    let cache = load_vector_cache(path)?;
    let provider = runtime
        .serving
        .embedding()
        .ok_or_else(|| Error::Unavailable("embedding identity required before commit".into()))?;
    if !cache.config.space.compatible_with(&provider.space())
        || cache.config.producer.signature_hash != provider.producer().signature_hash
    {
        return Err(Error::Conflict(
            "embedding cache identity disagrees with runtime".into(),
        ));
    }
    let needs: Vec<Need> =
        serde_json::from_slice(&std::fs::read(root.join("embedding-needs.json")).map_err(failure)?)
            .map_err(failure)?;
    let vectors = cache
        .vectors
        .into_iter()
        .map(|entry| (entry.text, entry.vector))
        .collect::<BTreeMap<_, _>>();
    let mut committed = 0;
    for need in needs {
        let vector = vectors
            .get(&need.text)
            .ok_or_else(|| Error::Unavailable("embedding cache page incomplete".into()))?;
        runtime
            .serving
            .commit_embedding(
                need.subject,
                need.reference,
                need.text,
                &cache.config.space.space_hash,
                &cache.config.producer.signature_hash,
                vector.clone(),
            )
            .await?;
        committed += 1;
    }
    println!("committed embedding references={committed}");
    Ok(())
}

fn load_scenarios(corpus: &std::path::Path) -> Result<Vec<Scenario>> {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(corpus.join("manifest.json")).map_err(failure)?)
            .map_err(failure)?;
    manifest["scenario_files"]
        .as_array()
        .ok_or_else(|| Error::Invalid("scenario manifest missing".into()))?
        .iter()
        .map(|file| {
            let path = file["path"]
                .as_str()
                .ok_or_else(|| Error::Invalid("scenario path missing".into()))?;
            serde_json::from_slice(&std::fs::read(corpus.join(path)).map_err(failure)?)
                .map_err(failure)
        })
        .collect()
}

fn load_import_state(corpus: &std::path::Path, state_path: &std::path::Path) -> Result<State> {
    let mut state: State = if state_path.exists() {
        serde_json::from_slice(&std::fs::read(state_path).map_err(failure)?).map_err(failure)?
    } else {
        State::default()
    };
    let manifest = std::fs::read(corpus.join("manifest.json")).map_err(failure)?;
    let corpus_digest = blake3::hash(&manifest).to_hex().to_string();
    if !state.corpus_digest.is_empty() && state.corpus_digest != corpus_digest {
        return Err(Error::Conflict(
            "corpus changed since import; use a separate run root".into(),
        ));
    }
    load_receipts(state_path, &mut state)?;
    state.corpus_digest = corpus_digest;
    save(state_path, &state)?;
    Ok(state)
}
