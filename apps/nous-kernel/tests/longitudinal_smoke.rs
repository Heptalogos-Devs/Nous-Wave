mod test_support;

use chrono::{Duration, TimeZone, Utc};
use nous_core::SubjectId;
use nous_kernel::{NousRuntime, RuntimeOptions, transport::KernelService};
use nous_protocol::kernel::{
    authority_service_server::AuthorityServiceServer,
    model_material_service_server::ModelMaterialServiceServer,
};
use nous_retrieval::ServingOptions;
use nous_runtime::ManualCognitiveClock;
use serde::Deserialize;
use std::{path::Path, process::Stdio, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::oneshot,
};
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Status};

struct Server {
    runtime: NousRuntime,
    endpoint: String,
    stop: oneshot::Sender<()>,
    task: tokio::task::JoinHandle<std::result::Result<(), tonic::transport::Error>>,
}
impl Server {
    async fn open(url: &str, root: &Path, clock: Arc<ManualCognitiveClock>, token: &str) -> Self {
        let runtime = NousRuntime::open_with_clock(RuntimeOptions {
            postgres_url:url.into(),max_connections:8,
            object_root:root.join("objects").to_string_lossy().into_owned(),
            serving_options:ServingOptions { root:root.join("serving"),lexical:true,dense:false,topology:false,memory_enabled:true },
            embedding:None,stored_embedding:None,
            core_descriptors: vec![],
        deployment_document:serde_json::json!({"serving":{"lexical":{"enabled":true},"dense":{"enabled":false},"topology":{"enabled":false}}}),
        },clock).await.unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let token = format!("Bearer {token}");
        let auth = move |request: Request<()>| -> std::result::Result<Request<()>, Status> {
            if request
                .metadata()
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                != Some(&token)
            {
                return Err(Status::unauthenticated("invalid smoke Kernel credential"));
            }
            Ok(request)
        };
        let service = KernelService(runtime.clone());
        let (stop, stopped) = oneshot::channel();
        let task = tokio::spawn(
            tonic::transport::Server::builder()
                .add_service(AuthorityServiceServer::with_interceptor(
                    service.clone(),
                    auth.clone(),
                ))
                .add_service(ModelMaterialServiceServer::with_interceptor(service, auth))
                .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                    let _ = stopped.await;
                }),
        );
        Self {
            runtime,
            endpoint,
            stop,
            task,
        }
    }
    async fn close(self) {
        let _ = self.stop.send(());
        tokio::time::timeout(std::time::Duration::from_secs(10), self.task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        self.runtime.store.close().await;
    }
}

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
enum Control {
    Advance { subject: SubjectId, seconds: i64 },
    Restart,
    Done,
}

#[tokio::test]
async fn public_longitudinal_smoke() {
    tokio::time::timeout(std::time::Duration::from_secs(120), run_smoke())
        .await
        .expect("longitudinal smoke deadline");
}
async fn run_smoke() {
    let (root, url, _postgres) = test_support::database().await;
    let clock = Arc::new(ManualCognitiveClock::new(
        Utc.with_ymd_and_hms(2026, 10, 3, 0, 0, 0).unwrap(),
    ));
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let mut server = Server::open(&url, root.path(), clock.clone(), &token).await;
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut child = tokio::process::Command::new(
        std::env::var_os("NOUS_LONGITUDINAL_NODE").unwrap_or_else(|| "node".into()),
    )
    .arg(repo.join("node_modules/tsx/dist/cli.mjs"))
    .arg(repo.join("scripts/smoke/longitudinal.ts"))
    .current_dir(&repo)
    .env("NOUS_LONGITUDINAL_ENDPOINT", &server.endpoint)
    .env("NOUS_LONGITUDINAL_TOKEN", &token)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit())
    .kill_on_drop(true)
    .spawn()
    .expect("launch Core smoke");
    let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut input = child.stdin.take().unwrap();
    while let Some(line) = output.next_line().await.unwrap() {
        let command: Control = serde_json::from_str(&line).expect("smoke control record");
        let response = match command {
            Control::Advance { subject, seconds } => {
                clock
                    .advance_by(subject, Duration::seconds(seconds))
                    .unwrap();
                serde_json::json!({"advanced":true})
            }
            Control::Restart => {
                server.close().await;
                server = Server::open(&url, root.path(), clock.clone(), &token).await;
                serde_json::json!({"endpoint":server.endpoint})
            }
            Control::Done => break,
        };
        input
            .write_all(format!("{response}\n").as_bytes())
            .await
            .unwrap();
        input.flush().await.unwrap();
    }
    drop(input);
    server.close().await;
    assert!(
        child.wait().await.unwrap().success(),
        "public longitudinal smoke failed"
    );
}
