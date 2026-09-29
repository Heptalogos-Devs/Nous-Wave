#[path = "test_support/mod.rs"]
mod test_support;

use chrono::{DateTime, Utc};
use nous_core::{
    CognitiveQuery, CognitiveRef, Cue, EntityCue, EntityRef, QueryTarget, TimeInterval,
};
use serde_json::Value;
use sqlx::Row;
use std::time::Instant;
use test_support::{database, open_memory_only_runtime};
use uuid::Uuid;

const SCALE: &str = include_str!("fixtures/memory-reference-r1/scale.json");

fn timestamp(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .expect("timestamp")
        .with_timezone(&Utc)
}

async fn create_subject(
    runtime: &nous_kernel::NousRuntime,
    subject: nous_core::SubjectId,
) -> nous_core::SubjectId {
    runtime
        .subjects
        .create_subject(nous_subject::CreateSubject {
            subject_id: Some(subject),
            operation_id: nous_core::OperationId(Uuid::from_u128(9001)),
            cognitive_seed: nous_subject::CognitiveSeedInput {
                text: "schema_version = 1".into(),
                format: nous_subject::COGNITIVE_SEED_FORMAT.into(),
                provenance: serde_json::json!({"fixture":"scale"}),
            },
            metadata: serde_json::json!({"fixture":"scale"}),
            capabilities: Some(nous_subject::SubjectCapabilities {
                memory: true,
                self_cognition: false,
                social: false,
            }),
        })
        .await
        .expect("scale subject")
        .subject_id
}

#[tokio::test]
#[expect(
    clippy::too_many_lines,
    reason = "scale fixture creates each required structural population in one deterministic transaction"
)]
async fn memory_reference_scale_fixture_keeps_lane_structural_gates() {
    let scale: Value = serde_json::from_str(SCALE).expect("scale JSON");
    assert_eq!(scale["current_memory_revisions"], 10000);
    assert_eq!(scale["historical_memory_revisions"], 3000);
    assert_eq!(scale["association_evidence"], 30000);
    assert_eq!(scale["entity_references"], 2000);
    assert_eq!(scale["tags"], 1000);
    assert_eq!(scale["cognitive_schemas"], 200);

    let (root, url, _postgres) = database().await;
    let runtime = open_memory_only_runtime(&url, &root).await;
    let subject = create_subject(&runtime, nous_core::SubjectId(Uuid::from_u128(7001))).await;
    let fixed = timestamp("2026-09-29T00:00:00Z");
    let mut tx = runtime.store.begin().await.expect("scale transaction");
    sqlx::query("INSERT INTO memory_objects(memory_id,subject_id,cognitive_role,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,accessibility_mode,created_at) SELECT md5('fixture-memory-'||n)::uuid,$1,'declarative',md5('fixture-memory-rev-'||n)::uuid,1,'accepted','valid','normal','normal','auto',$2 FROM generate_series(1,10000) AS values(n)")
        .bind(subject.0)
        .bind(fixed)
        .execute(&mut *tx)
        .await
        .expect("current memory population");
    sqlx::query("INSERT INTO memory_revisions(memory_revision_id,memory_id,subject_id,revision_no,parent_revision_id,revision_intent,formation_mode,grounding_occurrence_id,semantic_role,title,representation_text,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at) SELECT md5('fixture-memory-rev-'||n)::uuid,md5('fixture-memory-'||n)::uuid,$1,1,NULL,NULL,'synthesized',NULL,'scale',NULL,CASE WHEN n=10000 THEN 'scale temporal tail' ELSE 'scale memory '||n END,'inferred','unknown',NULL,NULL,$2,$2 FROM generate_series(1,10000) AS values(n)")
        .bind(subject.0)
        .bind(fixed)
        .execute(&mut *tx)
        .await
        .expect("current memory revisions");
    sqlx::query("INSERT INTO memory_revisions(memory_revision_id,memory_id,subject_id,revision_no,parent_revision_id,revision_intent,formation_mode,grounding_occurrence_id,semantic_role,title,representation_text,epistemic_class,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at) SELECT md5('fixture-history-'||n)::uuid,md5('fixture-memory-'||n)::uuid,$1,2,md5('fixture-memory-rev-'||n)::uuid,'rephrase','synthesized',NULL,'scale','historical','historical scale memory '||n,'inferred','unknown',NULL,NULL,$2,$2 FROM generate_series(1,3000) AS values(n)")
        .bind(subject.0)
        .bind(fixed)
        .execute(&mut *tx)
        .await
        .expect("historical memory revisions");
    sqlx::query("INSERT INTO memory_revision_aboutness(memory_revision_id,entity_ref) SELECT md5('fixture-memory-rev-'||n)::uuid,'entity:scale:'||n FROM generate_series(1,2000) AS values(n)")
        .execute(&mut *tx)
        .await
        .expect("entity references");
    sqlx::query("UPDATE memory_revisions SET valid_time_kind='instant',valid_time_start=$1,formed_at=$1,recorded_at=$1 WHERE memory_revision_id=md5('fixture-memory-rev-10000')::uuid")
        .bind(fixed)
        .execute(&mut *tx)
        .await
        .expect("temporal tail");
    sqlx::query("INSERT INTO tags(tag_id,subject_id,current_revision_id,created_at,status) SELECT md5('fixture-tag-'||n)::uuid,$1,md5('fixture-tag-rev-'||n)::uuid,$2,'active' FROM generate_series(1,1000) AS values(n)")
        .bind(subject.0)
        .bind(fixed)
        .execute(&mut *tx)
        .await
        .expect("tags");
    sqlx::query("INSERT INTO tag_revisions(tag_revision_id,tag_id,revision_no,label,description,kind_hint,origin,producer_signature_id,created_at) SELECT md5('fixture-tag-rev-'||n)::uuid,md5('fixture-tag-'||n)::uuid,1,'scale-tag-'||n,'scale tag','fixture','explicit',NULL,$1 FROM generate_series(1,1000) AS values(n)")
        .bind(fixed)
        .execute(&mut *tx)
        .await
        .expect("tag revisions");
    sqlx::query("INSERT INTO memory_revision_tags(memory_revision_id,tag_id) SELECT md5('fixture-memory-rev-'||n)::uuid,md5('fixture-tag-'||n)::uuid FROM generate_series(1,1000) AS values(n)")
        .execute(&mut *tx)
        .await
        .expect("memory tags");
    sqlx::query("INSERT INTO cognitive_schemas(schema_id,subject_id,current_revision_id,object_epoch,acceptance_state,integrity_state,suppression_state,purge_state,created_at) SELECT md5('fixture-schema-'||n)::uuid,$1,md5('fixture-schema-rev-'||n)::uuid,1,'accepted','valid','normal','normal',$2 FROM generate_series(1,200) AS values(n)")
        .bind(subject.0)
        .bind(fixed)
        .execute(&mut *tx)
        .await
        .expect("schemas");
    sqlx::query("INSERT INTO cognitive_schema_revisions(schema_revision_id,schema_id,revision_no,parent_revision_id,revision_intent,formation_kind,title,structural_claim,applicability_description,aboutness,tags,boundary_definition,valid_time_kind,valid_time_start,valid_time_end,formed_at,recorded_at) SELECT md5('fixture-schema-rev-'||n)::uuid,md5('fixture-schema-'||n)::uuid,1,NULL,NULL,'explicit_import','scale schema '||n,'scale structural claim','scale applicability','{}','{}','scale boundary','unknown',NULL,NULL,$1,$1 FROM generate_series(1,200) AS values(n)")
        .bind(fixed)
        .execute(&mut *tx)
        .await
        .expect("schema revisions");
    sqlx::query("INSERT INTO association_evidence(association_evidence_id,subject_id,from_ref_kind,from_ref,to_ref_kind,to_ref,relation_kind,polarity,support_class,valid_time_kind,valid_time_start,valid_time_end,producer_signature_id,created_at,revoked_at) SELECT md5('fixture-association-'||n)::uuid,$1,'memory_revision',(md5('fixture-memory-rev-'||(((n-1)%10000)+1))::uuid)::text,'memory_revision',(md5('fixture-memory-rev-'||((n%10000)+1))::uuid)::text,'fixture.link','positive','derived_structure','unknown',NULL,NULL,NULL,$2,NULL FROM generate_series(1,30000) AS values(n)")
        .bind(subject.0)
        .bind(fixed)
        .execute(&mut *tx)
        .await
        .expect("association evidence");
    sqlx::query("INSERT INTO association_evidence_supports(association_evidence_id,support_kind,support_ref,support_role,occurrence_id,source_region_id,derived_representation_id,derived_region_id) SELECT md5('fixture-association-'||n)::uuid,'memory_revision',(md5('fixture-memory-rev-'||(((n-1)%10000)+1))::uuid)::text,'direct',NULL,NULL,NULL,NULL FROM generate_series(1,30000) AS values(n)")
        .execute(&mut *tx)
        .await
        .expect("association supports");
    tx.commit().await.expect("commit scale fixture");

    let counts = sqlx::query("SELECT (SELECT count(*) FROM memory_objects WHERE subject_id=$1) AS current_count,(SELECT count(*) FROM memory_revisions WHERE subject_id=$1) AS revision_count,(SELECT count(*) FROM association_evidence WHERE subject_id=$1) AS association_count,(SELECT count(*) FROM memory_revision_aboutness WHERE memory_revision_id IN (SELECT current_revision_id FROM memory_objects WHERE subject_id=$1)) AS entity_count,(SELECT count(*) FROM tags WHERE subject_id=$1) AS tag_count,(SELECT count(*) FROM cognitive_schemas WHERE subject_id=$1) AS schema_count")
        .bind(subject.0)
        .fetch_one(runtime.store.pool())
        .await
        .expect("scale counts");
    assert_eq!(counts.try_get::<i64, _>("current_count").unwrap(), 10000);
    assert_eq!(counts.try_get::<i64, _>("revision_count").unwrap(), 13000);
    assert_eq!(
        counts.try_get::<i64, _>("association_count").unwrap(),
        30000
    );
    assert_eq!(counts.try_get::<i64, _>("entity_count").unwrap(), 2000);
    assert_eq!(counts.try_get::<i64, _>("tag_count").unwrap(), 1000);
    assert_eq!(counts.try_get::<i64, _>("schema_count").unwrap(), 200);

    let tail_revision: Uuid = sqlx::query_scalar("SELECT current_revision_id FROM memory_objects WHERE subject_id=$1 AND memory_id=md5('fixture-memory-10000')::uuid")
        .bind(subject.0)
        .fetch_one(runtime.store.pool())
        .await
        .expect("tail revision");
    let entity_revision: Uuid = sqlx::query_scalar("SELECT current_revision_id FROM memory_objects WHERE subject_id=$1 AND memory_id=md5('fixture-memory-2000')::uuid")
        .bind(subject.0)
        .fetch_one(runtime.store.pool())
        .await
        .expect("entity revision");
    let entity_result = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: vec![QueryTarget::AnyRelevantCognition],
            cues: vec![Cue::Entity(EntityCue {
                entity_ref: EntityRef::new("entity:scale:2000").unwrap(),
            })],
            constraints: Default::default(),
            exploration: Default::default(),
            resources: Default::default(),
            result_need: Default::default(),
            effort: Default::default(),
            capabilities: Default::default(),
            diagnostics: Default::default(),
        })
        .await
        .expect("tail entity query");
    assert!(
        entity_result.results.iter().any(|hit| {
            hit.reference
                == CognitiveRef::MemoryRevision(nous_core::MemoryRevisionId(entity_revision))
        }),
        "entity result diagnostics: {:?}",
        entity_result
    );
    let temporal_result = runtime
        .query(CognitiveQuery {
            api_version: nous_core::API_VERSION,
            subject,
            session: None,
            situation: Default::default(),
            targets: vec![QueryTarget::AnyRelevantCognition],
            cues: Vec::new(),
            constraints: nous_core::QueryConstraints {
                valid: Some(TimeInterval {
                    start: Some(fixed),
                    end: Some(fixed + chrono::Duration::seconds(1)),
                }),
                ..Default::default()
            },
            exploration: Default::default(),
            resources: Default::default(),
            result_need: Default::default(),
            effort: Default::default(),
            capabilities: Default::default(),
            diagnostics: Default::default(),
        })
        .await
        .expect("tail temporal query");
    assert!(
        temporal_result.results.iter().any(|hit| {
            hit.reference
                == CognitiveRef::MemoryRevision(nous_core::MemoryRevisionId(tail_revision))
        }),
        "temporal result diagnostics: {:?}",
        temporal_result
    );

    let mut query_samples_ms = Vec::with_capacity(20);
    for _ in 0..20 {
        let started = Instant::now();
        let result = runtime
            .query(CognitiveQuery {
                api_version: nous_core::API_VERSION,
                subject,
                session: None,
                situation: Default::default(),
                targets: vec![QueryTarget::AnyRelevantCognition],
                cues: vec![Cue::Entity(EntityCue {
                    entity_ref: EntityRef::new("entity:scale:2000").unwrap(),
                })],
                constraints: Default::default(),
                exploration: Default::default(),
                resources: Default::default(),
                result_need: Default::default(),
                effort: Default::default(),
                capabilities: Default::default(),
                diagnostics: Default::default(),
            })
            .await
            .expect("baseline query");
        assert_eq!(result.results.len(), 1);
        query_samples_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    query_samples_ms.sort_by(f64::total_cmp);
    let serving_started = Instant::now();
    runtime
        .serving
        .refresh(subject)
        .await
        .expect("scale serving build");
    let serving_build_ms = serving_started.elapsed().as_secs_f64() * 1000.0;
    println!(
        "PERFORMANCE_BASELINE_JSON={}",
        serde_json::json!({
            "query_samples": query_samples_ms.len(),
            "p50_ms": query_samples_ms[query_samples_ms.len() / 2],
            "p95_ms": query_samples_ms[(query_samples_ms.len() * 95 / 100).min(query_samples_ms.len() - 1)],
            "max_ms": query_samples_ms.last().copied().unwrap_or_default(),
            "serving_build_ms": serving_build_ms,
            "query_count": "NOT_INSTRUMENTED",
            "peak_memory": "NOT_INSTRUMENTED"
        })
    );
}
