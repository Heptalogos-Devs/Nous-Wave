//! Canonical RPC forwarding keeps transport adaptation separate from owner operations.
// Copyright 2026 Aravine Zhu
// SPDX-License-Identifier: Apache-2.0

macro_rules! rpc_service {
    ($service:path { forward { $( $method:ident ($input:ty) -> $output:ty; )* } custom { $( $custom:item )* } }) => {
        #[tonic::async_trait]
        impl $service for $crate::transport::KernelService {
            $(
                async fn $method(
                    &self,
                    request: tonic::Request<$input>,
                ) -> std::result::Result<tonic::Response<$output>, tonic::Status> {
                    $crate::transport::rpc_reply(
                        $crate::transport::KernelService::$method(self, request.into_inner()),
                    ).await
                }
            )*
            $( $custom )*
        }
    };
}
