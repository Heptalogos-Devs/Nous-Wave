use super::*;

rpc_service! {
    p::identity_service_server::IdentityService {
        forward {
            bind_identity(p::BindIdentityRequest) -> p::IdentityBinding;
            resolve_identity(p::ResolveIdentityRequest) -> p::ResolveIdentityResponse;
        }
        custom {}
    }
}
