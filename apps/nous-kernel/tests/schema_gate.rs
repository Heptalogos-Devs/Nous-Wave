#[path = "test_support/mod.rs"]
mod test_support;

#[tokio::test]
async fn reference_profile_schema_gate() {
    let (root, url, _postgres) = test_support::database().await;
    let runtime = test_support::open_runtime(&url, &root).await;
    let pool = runtime.store.pool().clone();
    let has_column = |table: &'static str, column: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM information_schema.columns WHERE table_schema='public' AND table_name=$1 AND column_name=$2)",
        )
        .bind(table)
        .bind(column)
        .fetch_one(&pool)
        .await
        .expect("schema column query")
        }
    };
    assert!(has_column("subjects", "authority_seq").await);
    assert!(has_column("memory_objects", "object_epoch").await);
    assert!(!has_column("memory_objects", "formation_mode").await);
    assert!(has_column("memory_revisions", "formation_mode").await);
    assert!(has_column("mutation_receipts", "operation_id").await);
    assert!(!has_column("association_evidence", "support_value").await);
    for table in [
        "anchors",
        "anchor_revisions",
        "anchor_support",
        "memory_revision_entities",
    ] {
        let exists: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)")
            .bind(format!("public.{table}"))
            .fetch_one(runtime.store.pool())
            .await
            .expect("old ontology query");
        assert!(exists.is_none(), "old table remains: {table}");
    }
    let primary_key: String = sqlx::query_scalar(
        "SELECT pg_get_constraintdef(oid) FROM pg_constraint WHERE conrelid='public.cognitive_use_events'::regclass AND contype='p'",
    )
    .fetch_one(runtime.store.pool())
    .await
    .expect("use event primary key");
    assert!(primary_key.contains("subject_id"));
    assert!(primary_key.contains("consumer_ref"));
    assert!(primary_key.contains("event_id"));
}
