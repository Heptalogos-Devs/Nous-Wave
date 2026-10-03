use super::*;
use nous_protocol::kernel::kernel_configuration_service_server::KernelConfigurationServiceServer;
use nous_protocol::kernel::{
    artifact_stream_service_server::ArtifactStreamServiceServer,
    authority_service_server::AuthorityServiceServer,
    model_material_service_server::ModelMaterialServiceServer,
};
use nous_protocol::public::configuration_service_server::ConfigurationServiceServer;

const WORKFLOW_PROTOCOL_ENVELOPE_BYTES: usize = 65536;

pub async fn router(runtime: NousRuntime, token: String) -> tonic::transport::server::Router {
    let auth = move |request: Request<()>| -> std::result::Result<Request<()>, Status> {
        let value = request
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        let expected = format!("Bearer {token}");
        if value.len() != expected.len()
            || value
                .bytes()
                .zip(expected.bytes())
                .fold(0u8, |a, (x, y)| a | (x ^ y))
                != 0
        {
            return Err(Status::unauthenticated("invalid Kernel credential"));
        }
        Ok(request)
    };
    let service = KernelService(runtime);
    let (reporter, health) = tonic_health::server::health_reporter();
    reporter
        .set_serving::<AuthorityServiceServer<KernelService>>()
        .await;
    tonic::transport::Server::builder()
        .add_service(KernelConfigurationServiceServer::with_interceptor(
            service.clone(),
            auth.clone(),
        ))
        .add_service(
            nous_protocol::public::material_service_server::MaterialServiceServer::with_interceptor(
                service.clone(),
                auth.clone(),
            ),
        )
        .add_service(
            nous_protocol::public::subject_service_server::SubjectServiceServer::with_interceptor(
                service.clone(),
                auth.clone(),
            ),
        )
        .add_service(
            nous_protocol::public::memory_service_server::MemoryServiceServer::with_interceptor(
                service.clone(),
                auth.clone(),
            ),
        )
        .add_service(
            nous_protocol::public::topology_service_server::TopologyServiceServer::with_interceptor(
                service.clone(),
                auth.clone(),
            ),
        )
        .add_service(
            nous_protocol::public::identity_service_server::IdentityServiceServer::with_interceptor(
                service.clone(),
                auth.clone(),
            ),
        )
        .add_service(ConfigurationServiceServer::with_interceptor(
            service.clone(),
            auth.clone(),
        ))
        .add_service(AuthorityServiceServer::with_interceptor(
            service.clone(),
            auth.clone(),
        ))
        .add_service(tonic::service::interceptor::InterceptedService::new(
            ModelMaterialServiceServer::new(service.clone())
                .max_decoding_message_size(
                    nous_persistence::WORKFLOW_VALUE_MAX_BYTES + WORKFLOW_PROTOCOL_ENVELOPE_BYTES,
                )
                .max_encoding_message_size(
                    nous_persistence::WORKFLOW_VALUE_MAX_BYTES + WORKFLOW_PROTOCOL_ENVELOPE_BYTES,
                ),
            auth.clone(),
        ))
        .add_service(ArtifactStreamServiceServer::with_interceptor(
            service,
            auth.clone(),
        ))
        .add_service(tonic::service::interceptor::InterceptedService::new(
            health, auth,
        ))
}
