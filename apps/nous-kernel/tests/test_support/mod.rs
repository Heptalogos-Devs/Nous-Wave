#![allow(dead_code)]

use chrono::Utc;
use nous_core::{EpistemicClass, OperationId, TemporalExtent};
use nous_kernel::{NousRuntime, RuntimeOptions};
use nous_material::{
    ObservationInput, ObservationMaterial, OccurrenceDescriptor, RuntimeDirective,
};
use nous_memory::{
    CognitiveRole, EvidenceLocator, EvidenceRef, ExplicitMemoryInput, FormationMode,
    RevisionSupport, SupportRole,
};
use nous_retrieval::ServingOptions;
use postgresql_embedded::{PostgreSQL, SettingsBuilder, VersionReq};
use std::time::Duration;
use tempfile::TempDir;

pub(crate) async fn database() -> (TempDir, String, PostgreSQL) {
    let root = TempDir::new().expect("temporary PostgreSQL root");
    let settings = SettingsBuilder::new()
        .version(VersionReq::parse("=18.6.0").expect("version"))
        .host("127.0.0.1")
        .port(0)
        .username("postgres")
        .password("nous_wave")
        .installation_dir(root.path().join("install"))
        .data_dir(root.path().join("data"))
        .password_file(root.path().join("postgres.pgpass"))
        .timeout(Some(Duration::from_secs(30)))
        .temporary(true)
        .build();
    let mut postgres = PostgreSQL::new(settings);
    postgres.setup().await.expect("setup PostgreSQL");
    postgres.start().await.expect("start PostgreSQL");
    postgres
        .create_database("nous_reference_profile")
        .await
        .expect("create database");
    let url = postgres.settings().url("nous_reference_profile");
    // Return the root first so test bindings drop PostgreSQL before TempDir.
    // PostgreSQL's Drop implementation stops the server; only then can TempDir
    // remove the installation and data tree without leaving a recent .tmp root.
    (root, url, postgres)
}

pub(crate) async fn open_runtime(url: &str, root: &TempDir) -> NousRuntime {
    open_runtime_with_serving(url, root, false, false, false).await
}

pub(crate) async fn open_memory_only_runtime(url: &str, root: &TempDir) -> NousRuntime {
    NousRuntime::open(RuntimeOptions {
        postgres_url: url.into(),
        max_connections: 4,
        object_root: root.path().join("objects").to_string_lossy().into_owned(),
        max_upload_bytes: 1024 * 1024,
        serving_options: ServingOptions {
            root: root.path().join("serving").to_path_buf(),
            lexical: false,
            dense: false,
            topology: false,
            memory_enabled: true,
            self_enabled: false,
            social_enabled: false,
        },
        embedding: None,
        stored_embedding: None,
        deployment_settings: serde_json::json!({
            "settings": {
                "capabilities": {
                    "process": { "memory": true, "self_cognition": false, "social": false },
                    "subject_defaults": { "memory": true, "self_cognition": false, "social": false }
                },
                "serving": {
                    "lexical": { "enabled": false },
                    "dense": { "enabled": false },
                    "topology": { "enabled": false }
                }
            }
        }),
    })
    .await
    .expect("open memory-only runtime")
}

pub(crate) async fn initial_seed_version(
    runtime: &NousRuntime,
    subject: nous_core::SubjectId,
) -> nous_core::CognitiveSeedVersionId {
    nous_core::CognitiveSeedVersionId(
        sqlx::query_scalar(
            "SELECT seed_version_id FROM cognitive_seed_versions WHERE subject_id=$1 ORDER BY created_at LIMIT 1",
        )
        .bind(subject.0)
        .fetch_one(runtime.store.pool())
        .await
        .expect("initial seed version"),
    )
}

pub(crate) async fn open_runtime_with_social(url: &str, root: &TempDir) -> NousRuntime {
    NousRuntime::open(RuntimeOptions {
        postgres_url: url.into(),
        max_connections: 4,
        object_root: root.path().join("objects").to_string_lossy().into_owned(),
        max_upload_bytes: 1024 * 1024,
        serving_options: ServingOptions {
            root: root.path().join("serving"),
            lexical: false,
            dense: false,
            topology: false,
            memory_enabled: true,
            self_enabled: false,
            social_enabled: true,
        },
        embedding: None,
        stored_embedding: None,
        deployment_settings: serde_json::json!({
            "settings": {
                "capabilities": {
                    "process": { "memory": true, "self_cognition": false, "social": true },
                    "subject_defaults": { "memory": true, "self_cognition": false, "social": false }
                },
                "serving": {
                    "lexical": { "enabled": false },
                    "dense": { "enabled": false },
                    "topology": { "enabled": false }
                }
            }
        }),
    })
    .await
    .expect("open social runtime")
}

pub(crate) async fn open_runtime_with_all_domains(url: &str, root: &TempDir) -> NousRuntime {
    NousRuntime::open(RuntimeOptions {
        postgres_url: url.into(),
        max_connections: 4,
        object_root: root.path().join("objects").to_string_lossy().into_owned(),
        max_upload_bytes: 1024 * 1024,
        serving_options: ServingOptions {
            root: root.path().join("serving"),
            lexical: true,
            dense: false,
            topology: false,
            memory_enabled: true,
            self_enabled: true,
            social_enabled: true,
        },
        embedding: None,
        stored_embedding: None,
        deployment_settings: serde_json::json!({
            "settings": {
                "capabilities": {
                    "process": { "memory": true, "self_cognition": true, "social": true },
                    "subject_defaults": { "memory": true, "self_cognition": true, "social": true }
                },
                "serving": {
                    "lexical": { "enabled": true },
                    "dense": { "enabled": false },
                    "topology": { "enabled": false }
                }
            }
        }),
    })
    .await
    .expect("open all-domain runtime")
}

pub(crate) async fn open_runtime_with_serving(
    url: &str,
    root: &TempDir,
    lexical: bool,
    dense: bool,
    topology: bool,
) -> NousRuntime {
    NousRuntime::open(RuntimeOptions {
        postgres_url: url.into(),
        max_connections: 4,
        object_root: root.path().join("objects").to_string_lossy().into_owned(),
        max_upload_bytes: 1024 * 1024,
        serving_options: ServingOptions {
            root: root.path().join("serving"),
            lexical,
            dense,
            topology,
            memory_enabled: true,
            self_enabled: true,
            social_enabled: false,
        },
        embedding: None,
        stored_embedding: None,
        deployment_settings: serde_json::json!({
            "settings": {
                "capabilities": {
                    "process": { "memory": true, "self_cognition": true, "social": false },
                    "subject_defaults": { "memory": true, "self_cognition": true, "social": false }
                },
                "serving": {
                    "lexical": { "enabled": lexical },
                    "dense": { "enabled": dense },
                    "topology": { "enabled": topology }
                }
            }
        }),
    })
    .await
    .expect("open runtime")
}

pub(crate) async fn observation(
    runtime: &NousRuntime,
    subject: nous_core::SubjectId,
    text: &str,
) -> nous_material::AcceptedObservation {
    runtime
        .material
        .record_observation(ObservationInput {
            subject,
            session: None,
            occurrence: OccurrenceDescriptor {
                source_class: nous_core::SourceClass::Message,
                external_object_ref: None,
                occurred_time: TemporalExtent::Unknown,
                observed_at: Utc::now(),
                conversation_ref: None,
                actor_entity_ref: None,
                context: serde_json::json!({}),
            },
            material: ObservationMaterial::InlineText {
                text: text.into(),
                media_type: "text/plain".into(),
            },
            entities: Vec::new(),
            runtime: RuntimeDirective::default(),
        })
        .await
        .expect("record observation")
}

pub(crate) async fn external_observation(
    runtime: &NousRuntime,
    subject: nous_core::SubjectId,
    object_ref: &str,
) -> nous_material::AcceptedObservation {
    let object_ref = nous_core::ObjectRef::new(object_ref).expect("object ref");
    runtime
        .material
        .record_observation(ObservationInput {
            subject,
            session: None,
            occurrence: OccurrenceDescriptor {
                source_class: nous_core::SourceClass::Message,
                external_object_ref: Some(object_ref.clone()),
                occurred_time: TemporalExtent::Unknown,
                observed_at: Utc::now(),
                conversation_ref: None,
                actor_entity_ref: None,
                context: serde_json::json!({}),
            },
            material: ObservationMaterial::ExternalObjectRef { object_ref },
            entities: Vec::new(),
            runtime: RuntimeDirective {
                admit: false,
                hold_until: None,
            },
        })
        .await
        .expect("record external observation")
}

pub(crate) async fn external_actor_observation(
    runtime: &NousRuntime,
    subject: nous_core::SubjectId,
    actor: &str,
    object_ref: &str,
) -> nous_material::AcceptedObservation {
    let actor = nous_core::EntityRef::new(actor).expect("actor ref");
    let object_ref = nous_core::ObjectRef::new(object_ref).expect("object ref");
    runtime
        .material
        .record_observation(ObservationInput {
            subject,
            session: None,
            occurrence: OccurrenceDescriptor {
                source_class: nous_core::SourceClass::Message,
                external_object_ref: Some(object_ref.clone()),
                occurred_time: TemporalExtent::Unknown,
                observed_at: Utc::now(),
                conversation_ref: None,
                actor_entity_ref: Some(actor),
                context: serde_json::json!({}),
            },
            material: ObservationMaterial::ExternalObjectRef { object_ref },
            entities: Vec::new(),
            runtime: RuntimeDirective {
                admit: false,
                hold_until: None,
            },
        })
        .await
        .expect("record external actor observation")
}

pub(crate) async fn occurrence_only_observation(
    runtime: &NousRuntime,
    subject: nous_core::SubjectId,
) -> nous_material::AcceptedObservation {
    runtime
        .material
        .record_observation(ObservationInput {
            subject,
            session: None,
            occurrence: OccurrenceDescriptor {
                source_class: nous_core::SourceClass::HostEvent,
                external_object_ref: None,
                occurred_time: TemporalExtent::Unknown,
                observed_at: Utc::now(),
                conversation_ref: None,
                actor_entity_ref: None,
                context: serde_json::json!({}),
            },
            material: ObservationMaterial::ResourceAvailability {
                resource: nous_core::ResourceRef::new("resource:unknown").expect("resource ref"),
            },
            entities: Vec::new(),
            runtime: RuntimeDirective {
                admit: false,
                hold_until: None,
            },
        })
        .await
        .expect("record occurrence-only observation")
}

pub(crate) fn form_input(
    subject: nous_core::SubjectId,
    occurrence: nous_core::OccurrenceId,
    operation_id: OperationId,
    text: &str,
) -> ExplicitMemoryInput {
    ExplicitMemoryInput {
        operation_id,
        subject,
        cognitive_role: CognitiveRole::Declarative,
        formation_mode: FormationMode::Grounded,
        grounding_occurrence_id: Some(occurrence),
        semantic_role: "fact".into(),
        representation_text: text.into(),
        title: None,
        supports: vec![RevisionSupport::Evidence(EvidenceRef {
            occurrence_id: occurrence,
            locator: EvidenceLocator::WholeOccurrence,
            support_role: SupportRole::Direct,
        })],
        aboutness: Vec::new(),
        tags: Vec::new(),
        valid_time: TemporalExtent::Unknown,
        formed_at: Utc::now(),
        epistemic_class: EpistemicClass::Observed,
    }
}
