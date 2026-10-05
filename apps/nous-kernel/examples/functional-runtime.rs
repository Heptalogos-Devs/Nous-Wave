//! Run-owned persistent research runtime. Semantic writes go through normal Core/Kernel owners.
use chrono::{DateTime, Utc};
use nous_core::*;
use nous_kernel::{NousRuntime, RuntimeOptions};
use nous_retrieval::ServingOptions;
use nous_runtime::{BoundQuery, CognitiveProfile, ManualCognitiveClock};
use postgresql_embedded::{PostgreSQL, SettingsBuilder, VersionReq};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::TcpListenerStream;
fn failure(error: impl std::fmt::Display) -> Error {
    Error::Infrastructure(error.to_string())
}
#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum Control {
    Advance {
        subject: SubjectId,
        instant: DateTime<Utc>,
    },
    Prepare {
        key: String,
        query: Box<CognitiveQuery>,
    },
    Execute {
        key: String,
        profile: CognitiveProfile,
    },
    Inspect,
    Metrics,
    Done,
}
fn main() -> Result<()> {
    let executor = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(failure)?;
    let result = executor.block_on(run());
    executor.shutdown_timeout(std::time::Duration::from_secs(2));
    result
}
async fn run() -> Result<()> {
    let root = std::path::absolute(
        std::env::args()
            .nth(1)
            .ok_or_else(|| Error::Invalid("run root required".into()))?,
    )
    .map_err(failure)?;
    if !root.starts_with(std::path::absolute("data/research").map_err(failure)?) {
        return Err(Error::Invalid("research root required".into()));
    }
    let token = std::env::var("NOUS_RESEARCH_TOKEN")
        .map_err(|_| Error::Invalid("run credential required".into()))?;
    let query_mode = std::env::args()
        .nth(2)
        .is_some_and(|value| value == "query");
    let baseline = directory_bytes(&root)?;
    let postgres = open_database(&root).await?;
    let database_url = postgres.settings().url("functional_cognition");
    let baseline_generations = generation_count(&database_url).await?;
    let clock = Arc::new(ManualCognitiveClock::new(
        "2026-09-01T00:00:00Z".parse().map_err(failure)?,
    ));
    let runtime = NousRuntime::open_with_clock(RuntimeOptions {
        postgres_url:postgres.settings().url("functional_cognition"),max_connections:8,acquire_timeout_ms:15000,
        object_root:root.join("objects").to_string_lossy().into_owned(),
        serving_options:ServingOptions {root:root.join("serving"),lexical:true,dense:false,topology:query_mode,memory_enabled:true},
        embedding:None,stored_embedding:None,core_descriptors:vec![],
        deployment_document:json!({"serving":{"lexical":{"enabled":true},"dense":{"enabled":false},"topology":{"enabled":query_mode}}}),
    },clock.clone()).await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(failure)?;
    let endpoint = format!("http://{}", listener.local_addr().map_err(failure)?);
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let mut server = tokio::spawn(
        nous_kernel::transport::router(runtime.clone(), token)
            .await
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                let _ = stopped.await;
            }),
    );
    println!(
        "{}",
        json!({"ready":true,"endpoint":endpoint,"baselineBytes":baseline,"baselineGenerations":baseline_generations,"metrics":metrics(&runtime,&root).await?})
    );
    let mut prepared = HashMap::<String, BoundQuery>::new();
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    loop {
        let next = tokio::select! {line=lines.next_line()=>line.map_err(failure)?, signal=tokio::signal::ctrl_c()=>{signal.map_err(failure)?;None}};
        let Some(line) = next else {
            break;
        };
        let command = match serde_json::from_str::<Control>(&line) {
            Ok(command) => command,
            Err(error) => {
                println!("{}", json!({"error":error.to_string()}));
                continue;
            }
        };
        let result = match command {
            Control::Advance { subject, instant } => clock
                .advance_to(subject, instant)
                .map(|()| json!({"advanced":true})),
            Control::Prepare { key, query } => prepare(&runtime, &mut prepared, key, *query).await,
            Control::Execute { key, profile } => {
                if query_mode {
                    execute(&runtime, &prepared, &key, profile).await
                } else {
                    Err(Error::Invalid(
                        "query execution requires explicit query mode".into(),
                    ))
                }
            }
            Control::Inspect => inspect(&runtime, &root).await,
            Control::Metrics => metrics(&runtime, &root).await,
            Control::Done => {
                println!(
                    "{}",
                    json!({"done":true,"metrics":metrics(&runtime,&root).await?})
                );
                break;
            }
        };
        println!(
            "{}",
            match result {
                Ok(value) => value,
                Err(error) => json!({"error":error.to_string()}),
            }
        );
    }
    let _ = stop.send(());
    if tokio::time::timeout(std::time::Duration::from_secs(10), &mut server)
        .await
        .is_err()
    {
        server.abort();
        let _ = server.await;
    }
    runtime.store.close().await;
    postgres.stop().await.map_err(failure)?;
    Ok(())
}
async fn open_database(root: &Path) -> Result<PostgreSQL> {
    let installation = std::env::var_os("NOUS_WAVE_POSTGRES_RUNTIME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/runtime/installed/postgresql"));
    if !installation
        .join(if cfg!(windows) {
            "bin/postgres.exe"
        } else {
            "bin/postgres"
        })
        .is_file()
    {
        return Err(Error::Unavailable(
            "installed PostgreSQL runtime required; research never downloads it".into(),
        ));
    }
    let settings = SettingsBuilder::new()
        .version(VersionReq::parse("=18.6.0").map_err(failure)?)
        .trust_installation_dir(true)
        .host("127.0.0.1")
        .port(0)
        .username("postgres")
        .password("functional-research-local")
        .installation_dir(installation)
        .data_dir(root.join("postgres"))
        .password_file(root.join("postgres.pgpass"))
        .temporary(false)
        .timeout(Some(std::time::Duration::from_secs(30)))
        .build();
    let mut postgres = PostgreSQL::new(settings);
    postgres.setup().await.map_err(failure)?;
    postgres.start().await.map_err(failure)?;
    if !postgres
        .database_exists("functional_cognition")
        .await
        .map_err(failure)?
    {
        postgres
            .create_database("functional_cognition")
            .await
            .map_err(failure)?;
    }
    Ok(postgres)
}
async fn metrics(runtime: &NousRuntime, root: &Path) -> Result<Value> {
    let database: i64 = sqlx::query_scalar("SELECT pg_database_size(current_database())")
        .fetch_one(runtime.store.pool())
        .await
        .map_err(failure)?;
    let generations: i64 = sqlx::query_scalar("SELECT count(*) FROM serving_generations")
        .fetch_one(runtime.store.pool())
        .await
        .map_err(failure)?;
    let retired: i64 =
        sqlx::query_scalar("SELECT count(*) FROM serving_generations WHERE state='retired'")
            .fetch_one(runtime.store.pool())
            .await
            .map_err(failure)?;
    let mut largest = std::fs::read_dir(root)
        .map_err(failure)?
        .map(|item| {
            let item = item.map_err(failure)?;
            let bytes = if item.file_type().map_err(failure)?.is_dir() {
                directory_bytes(&item.path())?
            } else {
                item.metadata().map_err(failure)?.len()
            };
            Ok((item.file_name().to_string_lossy().into_owned(), bytes))
        })
        .collect::<Result<Vec<_>>>()?;
    largest.sort_by_key(|item| std::cmp::Reverse(item.1));
    largest.truncate(5);
    Ok(
        json!({"runBytes":directory_bytes(root)?,"servingBytes":directory_bytes(&root.join("serving"))?,"databaseBytes":database,"generations":generations,"retired":retired,"largestPaths":largest}),
    )
}
fn directory_bytes(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let mut bytes = 0;
    for item in std::fs::read_dir(path).map_err(failure)? {
        let item = item.map_err(failure)?;
        let metadata = item.metadata().map_err(failure)?;
        if item.file_type().map_err(failure)?.is_symlink() {
            continue;
        }
        bytes += if metadata.is_dir() {
            directory_bytes(&item.path())?
        } else {
            metadata.len()
        };
    }
    Ok(bytes)
}
async fn inspect(runtime: &NousRuntime, root: &Path) -> Result<Value> {
    let pool = runtime.store.pool();
    let memories:Vec<Value>=sqlx::query_scalar("SELECT jsonb_build_object('subject',r.subject_id,'memory',r.memory_id,'revision',r.memory_revision_id,'text',r.representation_text,'role',r.semantic_role,'epistemicClass',r.epistemic_class,'state',o.acceptance_state,'supports',(SELECT COALESCE(jsonb_agg(to_jsonb(s)),'[]') FROM memory_revision_evidence s WHERE s.memory_revision_id=r.memory_revision_id)||(SELECT COALESCE(jsonb_agg(to_jsonb(d)),'[]') FROM memory_revision_dependencies d WHERE d.memory_revision_id=r.memory_revision_id)) FROM memory_revisions r JOIN memory_objects o ON o.current_revision_id=r.memory_revision_id ORDER BY r.recorded_at,r.memory_revision_id LIMIT 128").fetch_all(pool).await.map_err(failure)?;
    let tags:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(t)||jsonb_build_object('content',to_jsonb(r)) FROM tags t JOIN tag_revisions r ON r.tag_revision_id=t.current_revision_id ORDER BY t.tag_id LIMIT 128").fetch_all(pool).await.map_err(failure)?;
    let associations:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(a)||jsonb_build_object('supports',(SELECT COALESCE(jsonb_agg(to_jsonb(s)),'[]') FROM association_evidence_supports s WHERE s.association_evidence_id=a.association_evidence_id)) FROM association_evidence a ORDER BY a.association_evidence_id LIMIT 256").fetch_all(pool).await.map_err(failure)?;
    let needs: Vec<Value> = sqlx::query_scalar(
        "SELECT to_jsonb(n) FROM maintenance_needs n ORDER BY n.created_at,n.need_id LIMIT 256",
    )
    .fetch_all(pool)
    .await
    .map_err(failure)?;
    let episodes:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(r)||jsonb_build_object('members',(SELECT COALESCE(jsonb_agg(to_jsonb(m) ORDER BY m.ordinal),'[]') FROM episode_revision_members m WHERE m.episode_revision_id=r.episode_revision_id)) FROM episode_revisions r JOIN episode_objects o ON o.current_revision_id=r.episode_revision_id ORDER BY r.recorded_at LIMIT 128").fetch_all(pool).await.map_err(failure)?;
    let journals:Vec<Value>=sqlx::query_scalar("SELECT to_jsonb(r) FROM journal_revisions r JOIN journal_objects o ON o.current_revision_id=r.journal_revision_id ORDER BY r.recorded_at LIMIT 128").fetch_all(pool).await.map_err(failure)?;
    Ok(
        json!({"memories":memories,"tags":tags,"associations":associations,"needs":needs,"episodes":episodes,"journals":journals,"metrics":metrics(runtime,root).await?}),
    )
}

async fn prepare(
    runtime: &NousRuntime,
    queries: &mut HashMap<String, BoundQuery>,
    key: String,
    query: CognitiveQuery,
) -> Result<Value> {
    if key.is_empty() || key.len() > 128 || queries.contains_key(&key) || queries.len() >= 40 {
        return Err(Error::Invalid(
            "invalid or duplicate Prepared Query key".into(),
        ));
    }
    let bound = runtime.cognition.bind_query(query).await?;
    let result = json!({"key":key,"preparedQuery":bound.source_query,"representation":bound.representation,"authorityWatermark":bound.bound_at_authority_seq,"configSnapshotDigest":bound.config_snapshot.effective_digest,"enabledLanes":bound.enabled_lanes});
    queries.insert(key, bound);
    Ok(result)
}
async fn execute(
    runtime: &NousRuntime,
    queries: &HashMap<String, BoundQuery>,
    key: &str,
    profile: CognitiveProfile,
) -> Result<Value> {
    let bound = queries
        .get(key)
        .ok_or_else(|| Error::Invalid("query key was not prepared in this runtime".into()))?
        .clone()
        .for_profile(profile)?;
    let digest = bound.representation.sha256.clone();
    let snapshot = bound.config_snapshot.effective_digest.clone();
    let execution = runtime.execute_bound_query(bound, Some(32)).await?;
    let result = json!({"key":key,"profile":profile,"embeddingDigest":digest,"configSnapshotDigest":snapshot,"result":execution.result});
    drop(execution);
    Ok(result)
}

async fn generation_count(url: &str) -> Result<i64> {
    let catalog = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(url)
        .await
        .map_err(failure)?;
    let existing: bool =
        sqlx::query_scalar("SELECT to_regclass('public.serving_generations') IS NOT NULL")
            .fetch_one(&catalog)
            .await
            .map_err(failure)?;
    let count = if existing {
        sqlx::query_scalar("SELECT count(*) FROM serving_generations")
            .fetch_one(&catalog)
            .await
            .map_err(failure)?
    } else {
        0
    };
    catalog.close().await;
    Ok(count)
}
