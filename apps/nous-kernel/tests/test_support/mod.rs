#![allow(dead_code)]

use chrono::Utc;
use nous_core::{EpistemicClass, OperationId, TemporalExtent};
use nous_kernel::{NousRuntime, RuntimeOptions};
use nous_material::{
    ObservationInput, ObservationMaterial, OccurrenceDescriptor, RuntimeDirective,
};
use nous_memory_domain::{
    CognitiveRole, EvidenceLocator, EvidenceRef, ExplicitMemoryInput, FormationMode,
    RevisionSupport, SupportRole,
};
use nous_serving::ServingOptions;
use postgresql_embedded::{PostgreSQL, SettingsBuilder, VersionReq};
use std::time::Duration;
use tempfile::TempDir;

pub(crate) async fn database() -> (PostgreSQL, String, TempDir) {
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
        .temporary(false)
        .build();
    let mut postgres = PostgreSQL::new(settings);
    postgres.setup().await.expect("setup PostgreSQL");
    postgres.start().await.expect("start PostgreSQL");
    postgres
        .create_database("nous_reference_profile")
        .await
        .expect("create database");
    let url = postgres.settings().url("nous_reference_profile");
    (postgres, url, root)
}

pub(crate) async fn open_runtime(url: &str, root: &TempDir) -> NousRuntime {
    open_runtime_with_serving(url, root, false, false, false).await
}

pub(crate) async fn open_runtime_with_serving(
    url: &str,
    root: &TempDir,
    lexical: bool,
    dense: bool,
    topology: bool,
) -> NousRuntime {
    NousRuntime::open(RuntimeOptions {
        accessibility_policy: Default::default(),
        postgres_url: url.into(),
        max_connections: 4,
        object_root: root.path().join("objects").to_string_lossy().into_owned(),
        max_upload_bytes: 1024 * 1024,
        resident_limit: 256,
        memory_enabled: true,
        serving_options: ServingOptions {
            root: root.path().join("serving"),
            lexical,
            dense,
            topology,
            memory_enabled: true,
        },
        embedding: None,
        stored_embedding: None,
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
