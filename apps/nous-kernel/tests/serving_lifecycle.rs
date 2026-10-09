// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

mod test_support;

use nous_core::{CognitiveRef, OperationId, ServingNeed};
use test_support::query::{query, subject};
use test_support::{database, form_input, observation, open_runtime_with_serving};

#[tokio::test]
async fn cache_loss_rebuilds_at_the_same_watermark_and_retired_readers_are_protected() {
    let (root, url, _postgres) = database().await;
    let runtime = open_runtime_with_serving(&url, &root, true, false, false).await;
    let subject = subject(&runtime).await;
    let source = observation(
        &runtime,
        subject,
        "Recall relevant cognition from this source",
    )
    .await;
    let memory = runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            source.occurrence.occurrence_id,
            OperationId::new(),
            "Recall relevant cognition from the first source",
        ))
        .await
        .unwrap();
    let first = CognitiveRef::MemoryRevision(memory.revision.memory_revision_id);
    let request = query(subject);
    assert!(
        runtime
            .query(request.clone())
            .await
            .unwrap()
            .results
            .iter()
            .any(|hit| hit.reference == first)
    );
    let old = runtime
        .store
        .serving_current(subject)
        .await
        .unwrap()
        .into_iter()
        .find(|record| record.family == "lexical")
        .unwrap_or_else(|| panic!("Expected a lexical asset"));
    let held = runtime
        .execute_query(request.clone(), Some(5))
        .await
        .unwrap();
    let (_, ticket) = runtime
        .cognition
        .retain_query(held, test_support::query_lease())
        .unwrap();
    let ticket = ticket.expect("held validation ticket");
    let next = runtime
        .require_memory()
        .unwrap()
        .form_memory(form_input(
            subject,
            source.occurrence.occurrence_id,
            OperationId::new(),
            "Recall relevant cognition from a second source",
        ))
        .await
        .unwrap();
    runtime
        .serving
        .prepare(
            subject,
            ServingNeed {
                lexical: true,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let busy = runtime
        .serving
        .reclaim_retired(subject, std::time::Duration::ZERO)
        .await
        .unwrap();
    assert!(busy.readers_active);
    assert!(!busy.reclaimed.contains(&old.generation_id));
    assert!(std::path::Path::new(&old.artifact_location).exists());
    runtime.cognition.release_query(subject, ticket).unwrap();
    let reclaimed = runtime
        .serving
        .reclaim_retired(subject, std::time::Duration::ZERO)
        .await
        .unwrap();
    assert!(reclaimed.reclaimed.contains(&old.generation_id));
    assert!(!std::path::Path::new(&old.artifact_location).exists());
    let watermark = runtime.store.authority_seq(subject).await.unwrap();
    drop(runtime);
    let cache = root.path().join("serving");
    assert!(cache.starts_with(root.path()));
    std::fs::remove_dir_all(cache).unwrap();
    let reopened = open_runtime_with_serving(&url, &root, true, false, false).await;
    let rebuilt = reopened.query(request).await.unwrap();
    assert!(rebuilt.results.iter().any(|hit| hit.reference == first));
    assert!(
        rebuilt
            .results
            .iter()
            .any(|hit| hit.reference
                == CognitiveRef::MemoryRevision(next.revision.memory_revision_id))
    );
    assert_eq!(
        reopened.store.authority_seq(subject).await.unwrap(),
        watermark
    );
    assert!(
        reopened
            .store
            .serving_current(subject)
            .await
            .unwrap()
            .iter()
            .all(|record| std::path::Path::new(&record.artifact_location).exists())
    );
}
