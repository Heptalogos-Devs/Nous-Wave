use super::*;

rpc_service! {
    p::resource_registry_service_server::ResourceRegistryService {
        forward {
            put_resource(p::PutResourceRequest) -> p::ResourceDescriptor;
            get_resource(p::ObjectRequest) -> p::ResourceDescriptor;
            list_resources(p::ListRequest) -> p::ListResourcesResponse;
            remove_resource(p::ObjectRequest) -> ();
        }
        custom {}
    }
}
