// @generated
/// Generated server implementations.
pub mod kernel_configuration_service_server {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with KernelConfigurationServiceServer.
    #[async_trait]
    pub trait KernelConfigurationService: std::marker::Send + std::marker::Sync + 'static {
        ///
        async fn initialize_host_runtime(
            &self,
            request: tonic::Request<super::InitializeHostRuntimeRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
    }
    ///
    #[derive(Debug)]
    pub struct KernelConfigurationServiceServer<T> {
        inner: Arc<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    impl<T> KernelConfigurationServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>>
    for KernelConfigurationServiceServer<T>
    where
        T: KernelConfigurationService,
        B: Body + std::marker::Send + 'static,
        B::Error: Into<StdError> + std::marker::Send + 'static,
    {
        type Response = http::Response<tonic::body::Body>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            match req.uri().path() {
                "/nous.wave.kernel.v1alpha1.KernelConfigurationService/InitializeHostRuntime" => {
                    #[allow(non_camel_case_types)]
                    struct InitializeHostRuntimeSvc<T: KernelConfigurationService>(
                        pub Arc<T>,
                    );
                    impl<
                        T: KernelConfigurationService,
                    > tonic::server::UnaryService<super::InitializeHostRuntimeRequest>
                    for InitializeHostRuntimeSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::InitializeHostRuntimeRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as KernelConfigurationService>::initialize_host_runtime(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = InitializeHostRuntimeSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        let mut response = http::Response::new(
                            tonic::body::Body::default(),
                        );
                        let headers = response.headers_mut();
                        headers
                            .insert(
                                tonic::Status::GRPC_STATUS,
                                (tonic::Code::Unimplemented as i32).into(),
                            );
                        headers
                            .insert(
                                http::header::CONTENT_TYPE,
                                tonic::metadata::GRPC_CONTENT_TYPE,
                            );
                        Ok(response)
                    })
                }
            }
        }
    }
    impl<T> Clone for KernelConfigurationServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    /// Generated gRPC service name
    pub const SERVICE_NAME: &str = "nous.wave.kernel.v1alpha1.KernelConfigurationService";
    impl<T> tonic::server::NamedService for KernelConfigurationServiceServer<T> {
        const NAME: &'static str = SERVICE_NAME;
    }
}
/// Generated server implementations.
pub mod model_material_service_server {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with ModelMaterialServiceServer.
    #[async_trait]
    pub trait ModelMaterialService: std::marker::Send + std::marker::Sync + 'static {
        ///
        async fn segment_description(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::DescriptionSegments>,
            tonic::Status,
        >;
        ///
        async fn find_workflow(
            &self,
            request: tonic::Request<super::FindWorkflowRequest>,
        ) -> std::result::Result<tonic::Response<super::FoundWorkflow>, tonic::Status>;
        ///
        async fn get_resolved_mentions(
            &self,
            request: tonic::Request<super::ResolvedMentionsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ResolvedMentionsResponse>,
            tonic::Status,
        >;
        async fn reserve_workflow(
            &self,
            request: tonic::Request<super::ReserveWorkflowRequest>,
        ) -> std::result::Result<
            tonic::Response<super::WorkflowReservation>,
            tonic::Status,
        >;
        async fn save_workflow(
            &self,
            request: tonic::Request<super::SaveWorkflowRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        async fn release_workflow(
            &self,
            request: tonic::Request<super::ReleaseWorkflowRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        async fn get_embedding_config(
            &self,
            request: tonic::Request<()>,
        ) -> std::result::Result<tonic::Response<super::EmbeddingConfig>, tonic::Status>;
        async fn list_embedding_needs(
            &self,
            request: tonic::Request<super::EmbeddingNeedsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::EmbeddingNeedsResponse>,
            tonic::Status,
        >;
        async fn commit_embedding(
            &self,
            request: tonic::Request<super::CommitEmbeddingRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        async fn commit_interpretation(
            &self,
            request: tonic::Request<super::CommitInterpretationRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::DerivedRepresentation>,
            tonic::Status,
        >;
    }
    ///
    #[derive(Debug)]
    pub struct ModelMaterialServiceServer<T> {
        inner: Arc<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    impl<T> ModelMaterialServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>>
    for ModelMaterialServiceServer<T>
    where
        T: ModelMaterialService,
        B: Body + std::marker::Send + 'static,
        B::Error: Into<StdError> + std::marker::Send + 'static,
    {
        type Response = http::Response<tonic::body::Body>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            match req.uri().path() {
                "/nous.wave.kernel.v1alpha1.ModelMaterialService/SegmentDescription" => {
                    #[allow(non_camel_case_types)]
                    struct SegmentDescriptionSvc<T: ModelMaterialService>(pub Arc<T>);
                    impl<
                        T: ModelMaterialService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for SegmentDescriptionSvc<T> {
                        type Response = super::DescriptionSegments;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ObjectRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ModelMaterialService>::segment_description(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = SegmentDescriptionSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.ModelMaterialService/FindWorkflow" => {
                    #[allow(non_camel_case_types)]
                    struct FindWorkflowSvc<T: ModelMaterialService>(pub Arc<T>);
                    impl<
                        T: ModelMaterialService,
                    > tonic::server::UnaryService<super::FindWorkflowRequest>
                    for FindWorkflowSvc<T> {
                        type Response = super::FoundWorkflow;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FindWorkflowRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ModelMaterialService>::find_workflow(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = FindWorkflowSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.ModelMaterialService/GetResolvedMentions" => {
                    #[allow(non_camel_case_types)]
                    struct GetResolvedMentionsSvc<T: ModelMaterialService>(pub Arc<T>);
                    impl<
                        T: ModelMaterialService,
                    > tonic::server::UnaryService<super::ResolvedMentionsRequest>
                    for GetResolvedMentionsSvc<T> {
                        type Response = super::ResolvedMentionsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ResolvedMentionsRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ModelMaterialService>::get_resolved_mentions(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetResolvedMentionsSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.ModelMaterialService/ReserveWorkflow" => {
                    #[allow(non_camel_case_types)]
                    struct ReserveWorkflowSvc<T: ModelMaterialService>(pub Arc<T>);
                    impl<
                        T: ModelMaterialService,
                    > tonic::server::UnaryService<super::ReserveWorkflowRequest>
                    for ReserveWorkflowSvc<T> {
                        type Response = super::WorkflowReservation;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ReserveWorkflowRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ModelMaterialService>::reserve_workflow(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ReserveWorkflowSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.ModelMaterialService/SaveWorkflow" => {
                    #[allow(non_camel_case_types)]
                    struct SaveWorkflowSvc<T: ModelMaterialService>(pub Arc<T>);
                    impl<
                        T: ModelMaterialService,
                    > tonic::server::UnaryService<super::SaveWorkflowRequest>
                    for SaveWorkflowSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SaveWorkflowRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ModelMaterialService>::save_workflow(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = SaveWorkflowSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.ModelMaterialService/ReleaseWorkflow" => {
                    #[allow(non_camel_case_types)]
                    struct ReleaseWorkflowSvc<T: ModelMaterialService>(pub Arc<T>);
                    impl<
                        T: ModelMaterialService,
                    > tonic::server::UnaryService<super::ReleaseWorkflowRequest>
                    for ReleaseWorkflowSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ReleaseWorkflowRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ModelMaterialService>::release_workflow(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ReleaseWorkflowSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.ModelMaterialService/GetEmbeddingConfig" => {
                    #[allow(non_camel_case_types)]
                    struct GetEmbeddingConfigSvc<T: ModelMaterialService>(pub Arc<T>);
                    impl<T: ModelMaterialService> tonic::server::UnaryService<()>
                    for GetEmbeddingConfigSvc<T> {
                        type Response = super::EmbeddingConfig;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(&mut self, request: tonic::Request<()>) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ModelMaterialService>::get_embedding_config(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetEmbeddingConfigSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.ModelMaterialService/ListEmbeddingNeeds" => {
                    #[allow(non_camel_case_types)]
                    struct ListEmbeddingNeedsSvc<T: ModelMaterialService>(pub Arc<T>);
                    impl<
                        T: ModelMaterialService,
                    > tonic::server::UnaryService<super::EmbeddingNeedsRequest>
                    for ListEmbeddingNeedsSvc<T> {
                        type Response = super::EmbeddingNeedsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::EmbeddingNeedsRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ModelMaterialService>::list_embedding_needs(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ListEmbeddingNeedsSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.ModelMaterialService/CommitEmbedding" => {
                    #[allow(non_camel_case_types)]
                    struct CommitEmbeddingSvc<T: ModelMaterialService>(pub Arc<T>);
                    impl<
                        T: ModelMaterialService,
                    > tonic::server::UnaryService<super::CommitEmbeddingRequest>
                    for CommitEmbeddingSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CommitEmbeddingRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ModelMaterialService>::commit_embedding(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = CommitEmbeddingSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.ModelMaterialService/CommitInterpretation" => {
                    #[allow(non_camel_case_types)]
                    struct CommitInterpretationSvc<T: ModelMaterialService>(pub Arc<T>);
                    impl<
                        T: ModelMaterialService,
                    > tonic::server::UnaryService<super::CommitInterpretationRequest>
                    for CommitInterpretationSvc<T> {
                        type Response = super::super::super::v1alpha1::DerivedRepresentation;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CommitInterpretationRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ModelMaterialService>::commit_interpretation(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = CommitInterpretationSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        let mut response = http::Response::new(
                            tonic::body::Body::default(),
                        );
                        let headers = response.headers_mut();
                        headers
                            .insert(
                                tonic::Status::GRPC_STATUS,
                                (tonic::Code::Unimplemented as i32).into(),
                            );
                        headers
                            .insert(
                                http::header::CONTENT_TYPE,
                                tonic::metadata::GRPC_CONTENT_TYPE,
                            );
                        Ok(response)
                    })
                }
            }
        }
    }
    impl<T> Clone for ModelMaterialServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    /// Generated gRPC service name
    pub const SERVICE_NAME: &str = "nous.wave.kernel.v1alpha1.ModelMaterialService";
    impl<T> tonic::server::NamedService for ModelMaterialServiceServer<T> {
        const NAME: &'static str = SERVICE_NAME;
    }
}
/// Generated server implementations.
pub mod artifact_stream_service_server {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with ArtifactStreamServiceServer.
    #[async_trait]
    pub trait ArtifactStreamService: std::marker::Send + std::marker::Sync + 'static {
        ///
        async fn upload_artifact(
            &self,
            request: tonic::Request<tonic::Streaming<super::UploadChunk>>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Artifact>,
            tonic::Status,
        >;
        /// Server streaming response type for the DownloadArtifact method.
        type DownloadArtifactStream: tonic::codegen::tokio_stream::Stream<
                Item = std::result::Result<super::DownloadChunk, tonic::Status>,
            >
            + std::marker::Send
            + 'static;
        ///
        async fn download_artifact(
            &self,
            request: tonic::Request<super::DownloadRequest>,
        ) -> std::result::Result<
            tonic::Response<Self::DownloadArtifactStream>,
            tonic::Status,
        >;
    }
    ///
    #[derive(Debug)]
    pub struct ArtifactStreamServiceServer<T> {
        inner: Arc<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    impl<T> ArtifactStreamServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>>
    for ArtifactStreamServiceServer<T>
    where
        T: ArtifactStreamService,
        B: Body + std::marker::Send + 'static,
        B::Error: Into<StdError> + std::marker::Send + 'static,
    {
        type Response = http::Response<tonic::body::Body>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            match req.uri().path() {
                "/nous.wave.kernel.v1alpha1.ArtifactStreamService/UploadArtifact" => {
                    #[allow(non_camel_case_types)]
                    struct UploadArtifactSvc<T: ArtifactStreamService>(pub Arc<T>);
                    impl<
                        T: ArtifactStreamService,
                    > tonic::server::ClientStreamingService<super::UploadChunk>
                    for UploadArtifactSvc<T> {
                        type Response = super::super::super::v1alpha1::Artifact;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<tonic::Streaming<super::UploadChunk>>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ArtifactStreamService>::upload_artifact(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = UploadArtifactSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.client_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.ArtifactStreamService/DownloadArtifact" => {
                    #[allow(non_camel_case_types)]
                    struct DownloadArtifactSvc<T: ArtifactStreamService>(pub Arc<T>);
                    impl<
                        T: ArtifactStreamService,
                    > tonic::server::ServerStreamingService<super::DownloadRequest>
                    for DownloadArtifactSvc<T> {
                        type Response = super::DownloadChunk;
                        type ResponseStream = T::DownloadArtifactStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DownloadRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as ArtifactStreamService>::download_artifact(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = DownloadArtifactSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        let mut response = http::Response::new(
                            tonic::body::Body::default(),
                        );
                        let headers = response.headers_mut();
                        headers
                            .insert(
                                tonic::Status::GRPC_STATUS,
                                (tonic::Code::Unimplemented as i32).into(),
                            );
                        headers
                            .insert(
                                http::header::CONTENT_TYPE,
                                tonic::metadata::GRPC_CONTENT_TYPE,
                            );
                        Ok(response)
                    })
                }
            }
        }
    }
    impl<T> Clone for ArtifactStreamServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    /// Generated gRPC service name
    pub const SERVICE_NAME: &str = "nous.wave.kernel.v1alpha1.ArtifactStreamService";
    impl<T> tonic::server::NamedService for ArtifactStreamServiceServer<T> {
        const NAME: &'static str = SERVICE_NAME;
    }
}
/// Generated server implementations.
pub mod authority_service_server {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with AuthorityServiceServer.
    #[async_trait]
    pub trait AuthorityService: std::marker::Send + std::marker::Sync + 'static {
        ///
        async fn get_cognitive_time(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::SubjectRequest>,
        ) -> std::result::Result<
            tonic::Response<::prost_types::Timestamp>,
            tonic::Status,
        >;
        ///
        async fn commit_longitudinal_consolidation(
            &self,
            request: tonic::Request<super::CommitLongitudinalConsolidationRequest>,
        ) -> std::result::Result<
            tonic::Response<super::CommitLongitudinalConsolidationResponse>,
            tonic::Status,
        >;
        ///
        async fn refresh_maintenance(
            &self,
            request: tonic::Request<super::RefreshMaintenanceRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        ///
        async fn plan_maintenance(
            &self,
            request: tonic::Request<super::PlanMaintenanceRequest>,
        ) -> std::result::Result<tonic::Response<super::MaintenancePlan>, tonic::Status>;
        ///
        async fn get_maintenance_policy(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::SubjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::MaintenancePolicy>,
            tonic::Status,
        >;
        ///
        async fn claim_maintenance(
            &self,
            request: tonic::Request<super::ClaimMaintenanceRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ClaimMaintenanceResponse>,
            tonic::Status,
        >;
        ///
        async fn finish_maintenance(
            &self,
            request: tonic::Request<super::FinishMaintenanceRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        ///
        async fn organize_experience(
            &self,
            request: tonic::Request<super::OrganizeExperienceRequest>,
        ) -> std::result::Result<
            tonic::Response<super::OrganizeExperienceResponse>,
            tonic::Status,
        >;
        ///
        async fn apply_episode_partition(
            &self,
            request: tonic::Request<super::ApplyEpisodePartitionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ApplyEpisodePartitionResponse>,
            tonic::Status,
        >;
        ///
        async fn commit_journal(
            &self,
            request: tonic::Request<super::CommitJournalRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::JournalResponse>,
            tonic::Status,
        >;
        ///
        async fn create_work_context(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::CreateWorkContextRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::WorkContextResponse>,
            tonic::Status,
        >;
        ///
        async fn get_work_context(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::GetWorkContextRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::WorkContextResponse>,
            tonic::Status,
        >;
        ///
        async fn list_work_contexts(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::ListWorkContextsRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListWorkContextsResponse>,
            tonic::Status,
        >;
        ///
        async fn update_work_context(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::UpdateWorkContextRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::WorkContextResponse>,
            tonic::Status,
        >;
        ///
        async fn pause_work_context(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::WorkContextMutationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::WorkContextResponse>,
            tonic::Status,
        >;
        ///
        async fn resume_work_context(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::WorkContextMutationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::WorkContextResponse>,
            tonic::Status,
        >;
        ///
        async fn end_work_context(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::WorkContextMutationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::WorkContextResponse>,
            tonic::Status,
        >;
        ///
        async fn set_active_work_context(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::SetActiveWorkContextRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Session>,
            tonic::Status,
        >;
        ///
        async fn put_resource(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::PutResourceRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ResourceDescriptor>,
            tonic::Status,
        >;
        ///
        async fn get_resource(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ResourceDescriptor>,
            tonic::Status,
        >;
        ///
        async fn list_resources(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ListRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListResourcesResponse>,
            tonic::Status,
        >;
        ///
        async fn remove_resource(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        ///
        async fn get_status(
            &self,
            request: tonic::Request<()>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::SystemStatus>,
            tonic::Status,
        >;
        ///
        async fn get_projection_status(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::SubjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ProjectionStatus>,
            tonic::Status,
        >;
        ///
        async fn build_contribution_batch(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ProjectionRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Projection>,
            tonic::Status,
        >;
        ///
        async fn open_session(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::SubjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Session>,
            tonic::Status,
        >;
        ///
        async fn get_session(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Session>,
            tonic::Status,
        >;
        ///
        async fn list_sessions(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ListRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListSessionsResponse>,
            tonic::Status,
        >;
        ///
        async fn close_session(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Session>,
            tonic::Status,
        >;
        ///
        async fn record_observation(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObservationInput>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::AcceptedObservation>,
            tonic::Status,
        >;
        ///
        async fn query(
            &self,
            request: tonic::Request<super::KernelQueryRequest>,
        ) -> std::result::Result<
            tonic::Response<super::KernelQueryResponse>,
            tonic::Status,
        >;
        ///
        async fn finalize_query(
            &self,
            request: tonic::Request<super::FinalizeQueryRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::QueryResponse>,
            tonic::Status,
        >;
        ///
        async fn release_query(
            &self,
            request: tonic::Request<super::ReleaseQueryRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        ///
        async fn report_use(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ReportUseRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ReportUseResponse>,
            tonic::Status,
        >;
    }
    ///
    #[derive(Debug)]
    pub struct AuthorityServiceServer<T> {
        inner: Arc<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    impl<T> AuthorityServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for AuthorityServiceServer<T>
    where
        T: AuthorityService,
        B: Body + std::marker::Send + 'static,
        B::Error: Into<StdError> + std::marker::Send + 'static,
    {
        type Response = http::Response<tonic::body::Body>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            match req.uri().path() {
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetCognitiveTime" => {
                    #[allow(non_camel_case_types)]
                    struct GetCognitiveTimeSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::SubjectRequest,
                    > for GetCognitiveTimeSvc<T> {
                        type Response = ::prost_types::Timestamp;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::SubjectRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::get_cognitive_time(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetCognitiveTimeSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/CommitLongitudinalConsolidation" => {
                    #[allow(non_camel_case_types)]
                    struct CommitLongitudinalConsolidationSvc<T: AuthorityService>(
                        pub Arc<T>,
                    );
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::CommitLongitudinalConsolidationRequest,
                    > for CommitLongitudinalConsolidationSvc<T> {
                        type Response = super::CommitLongitudinalConsolidationResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::CommitLongitudinalConsolidationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::commit_longitudinal_consolidation(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = CommitLongitudinalConsolidationSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/RefreshMaintenance" => {
                    #[allow(non_camel_case_types)]
                    struct RefreshMaintenanceSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<super::RefreshMaintenanceRequest>
                    for RefreshMaintenanceSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::RefreshMaintenanceRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::refresh_maintenance(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = RefreshMaintenanceSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/PlanMaintenance" => {
                    #[allow(non_camel_case_types)]
                    struct PlanMaintenanceSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<super::PlanMaintenanceRequest>
                    for PlanMaintenanceSvc<T> {
                        type Response = super::MaintenancePlan;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::PlanMaintenanceRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::plan_maintenance(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = PlanMaintenanceSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetMaintenancePolicy" => {
                    #[allow(non_camel_case_types)]
                    struct GetMaintenancePolicySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::SubjectRequest,
                    > for GetMaintenancePolicySvc<T> {
                        type Response = super::MaintenancePolicy;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::SubjectRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::get_maintenance_policy(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetMaintenancePolicySvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/ClaimMaintenance" => {
                    #[allow(non_camel_case_types)]
                    struct ClaimMaintenanceSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<super::ClaimMaintenanceRequest>
                    for ClaimMaintenanceSvc<T> {
                        type Response = super::ClaimMaintenanceResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ClaimMaintenanceRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::claim_maintenance(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ClaimMaintenanceSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/FinishMaintenance" => {
                    #[allow(non_camel_case_types)]
                    struct FinishMaintenanceSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<super::FinishMaintenanceRequest>
                    for FinishMaintenanceSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FinishMaintenanceRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::finish_maintenance(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = FinishMaintenanceSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/OrganizeExperience" => {
                    #[allow(non_camel_case_types)]
                    struct OrganizeExperienceSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<super::OrganizeExperienceRequest>
                    for OrganizeExperienceSvc<T> {
                        type Response = super::OrganizeExperienceResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::OrganizeExperienceRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::organize_experience(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = OrganizeExperienceSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/ApplyEpisodePartition" => {
                    #[allow(non_camel_case_types)]
                    struct ApplyEpisodePartitionSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<super::ApplyEpisodePartitionRequest>
                    for ApplyEpisodePartitionSvc<T> {
                        type Response = super::ApplyEpisodePartitionResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ApplyEpisodePartitionRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::apply_episode_partition(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ApplyEpisodePartitionSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/CommitJournal" => {
                    #[allow(non_camel_case_types)]
                    struct CommitJournalSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<super::CommitJournalRequest>
                    for CommitJournalSvc<T> {
                        type Response = super::super::super::v1alpha1::JournalResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CommitJournalRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::commit_journal(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = CommitJournalSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/CreateWorkContext" => {
                    #[allow(non_camel_case_types)]
                    struct CreateWorkContextSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::CreateWorkContextRequest,
                    > for CreateWorkContextSvc<T> {
                        type Response = super::super::super::v1alpha1::WorkContextResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::CreateWorkContextRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::create_work_context(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = CreateWorkContextSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetWorkContext" => {
                    #[allow(non_camel_case_types)]
                    struct GetWorkContextSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::GetWorkContextRequest,
                    > for GetWorkContextSvc<T> {
                        type Response = super::super::super::v1alpha1::WorkContextResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::GetWorkContextRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::get_work_context(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetWorkContextSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListWorkContexts" => {
                    #[allow(non_camel_case_types)]
                    struct ListWorkContextsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ListWorkContextsRequest,
                    > for ListWorkContextsSvc<T> {
                        type Response = super::super::super::v1alpha1::ListWorkContextsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ListWorkContextsRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::list_work_contexts(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ListWorkContextsSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/UpdateWorkContext" => {
                    #[allow(non_camel_case_types)]
                    struct UpdateWorkContextSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::UpdateWorkContextRequest,
                    > for UpdateWorkContextSvc<T> {
                        type Response = super::super::super::v1alpha1::WorkContextResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::UpdateWorkContextRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::update_work_context(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = UpdateWorkContextSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/PauseWorkContext" => {
                    #[allow(non_camel_case_types)]
                    struct PauseWorkContextSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::WorkContextMutationRequest,
                    > for PauseWorkContextSvc<T> {
                        type Response = super::super::super::v1alpha1::WorkContextResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::WorkContextMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::pause_work_context(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = PauseWorkContextSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/ResumeWorkContext" => {
                    #[allow(non_camel_case_types)]
                    struct ResumeWorkContextSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::WorkContextMutationRequest,
                    > for ResumeWorkContextSvc<T> {
                        type Response = super::super::super::v1alpha1::WorkContextResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::WorkContextMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::resume_work_context(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ResumeWorkContextSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/EndWorkContext" => {
                    #[allow(non_camel_case_types)]
                    struct EndWorkContextSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::WorkContextMutationRequest,
                    > for EndWorkContextSvc<T> {
                        type Response = super::super::super::v1alpha1::WorkContextResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::WorkContextMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::end_work_context(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = EndWorkContextSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/SetActiveWorkContext" => {
                    #[allow(non_camel_case_types)]
                    struct SetActiveWorkContextSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::SetActiveWorkContextRequest,
                    > for SetActiveWorkContextSvc<T> {
                        type Response = super::super::super::v1alpha1::Session;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::SetActiveWorkContextRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::set_active_work_context(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = SetActiveWorkContextSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/PutResource" => {
                    #[allow(non_camel_case_types)]
                    struct PutResourceSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::PutResourceRequest,
                    > for PutResourceSvc<T> {
                        type Response = super::super::super::v1alpha1::ResourceDescriptor;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::PutResourceRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::put_resource(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = PutResourceSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetResource" => {
                    #[allow(non_camel_case_types)]
                    struct GetResourceSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetResourceSvc<T> {
                        type Response = super::super::super::v1alpha1::ResourceDescriptor;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ObjectRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::get_resource(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetResourceSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListResources" => {
                    #[allow(non_camel_case_types)]
                    struct ListResourcesSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ListRequest,
                    > for ListResourcesSvc<T> {
                        type Response = super::super::super::v1alpha1::ListResourcesResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ListRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::list_resources(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ListResourcesSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/RemoveResource" => {
                    #[allow(non_camel_case_types)]
                    struct RemoveResourceSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for RemoveResourceSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ObjectRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::remove_resource(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = RemoveResourceSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetStatus" => {
                    #[allow(non_camel_case_types)]
                    struct GetStatusSvc<T: AuthorityService>(pub Arc<T>);
                    impl<T: AuthorityService> tonic::server::UnaryService<()>
                    for GetStatusSvc<T> {
                        type Response = super::super::super::v1alpha1::SystemStatus;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(&mut self, request: tonic::Request<()>) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::get_status(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetStatusSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetProjectionStatus" => {
                    #[allow(non_camel_case_types)]
                    struct GetProjectionStatusSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::SubjectRequest,
                    > for GetProjectionStatusSvc<T> {
                        type Response = super::super::super::v1alpha1::ProjectionStatus;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::SubjectRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::get_projection_status(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetProjectionStatusSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/BuildContributionBatch" => {
                    #[allow(non_camel_case_types)]
                    struct BuildContributionBatchSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ProjectionRequest,
                    > for BuildContributionBatchSvc<T> {
                        type Response = super::super::super::v1alpha1::Projection;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ProjectionRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::build_contribution_batch(
                                        &inner,
                                        request,
                                    )
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = BuildContributionBatchSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/OpenSession" => {
                    #[allow(non_camel_case_types)]
                    struct OpenSessionSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::SubjectRequest,
                    > for OpenSessionSvc<T> {
                        type Response = super::super::super::v1alpha1::Session;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::SubjectRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::open_session(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = OpenSessionSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetSession" => {
                    #[allow(non_camel_case_types)]
                    struct GetSessionSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetSessionSvc<T> {
                        type Response = super::super::super::v1alpha1::Session;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ObjectRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::get_session(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetSessionSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListSessions" => {
                    #[allow(non_camel_case_types)]
                    struct ListSessionsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ListRequest,
                    > for ListSessionsSvc<T> {
                        type Response = super::super::super::v1alpha1::ListSessionsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ListRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::list_sessions(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ListSessionsSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/CloseSession" => {
                    #[allow(non_camel_case_types)]
                    struct CloseSessionSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for CloseSessionSvc<T> {
                        type Response = super::super::super::v1alpha1::Session;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ObjectRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::close_session(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = CloseSessionSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/RecordObservation" => {
                    #[allow(non_camel_case_types)]
                    struct RecordObservationSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObservationInput,
                    > for RecordObservationSvc<T> {
                        type Response = super::super::super::v1alpha1::AcceptedObservation;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ObservationInput,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::record_observation(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = RecordObservationSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/Query" => {
                    #[allow(non_camel_case_types)]
                    struct QuerySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<super::KernelQueryRequest>
                    for QuerySvc<T> {
                        type Response = super::KernelQueryResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::KernelQueryRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::query(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = QuerySvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/FinalizeQuery" => {
                    #[allow(non_camel_case_types)]
                    struct FinalizeQuerySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<super::FinalizeQueryRequest>
                    for FinalizeQuerySvc<T> {
                        type Response = super::super::super::v1alpha1::QueryResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FinalizeQueryRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::finalize_query(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = FinalizeQuerySvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/ReleaseQuery" => {
                    #[allow(non_camel_case_types)]
                    struct ReleaseQuerySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<super::ReleaseQueryRequest>
                    for ReleaseQuerySvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ReleaseQueryRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::release_query(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ReleaseQuerySvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/nous.wave.kernel.v1alpha1.AuthorityService/ReportUse" => {
                    #[allow(non_camel_case_types)]
                    struct ReportUseSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ReportUseRequest,
                    > for ReportUseSvc<T> {
                        type Response = super::super::super::v1alpha1::ReportUseResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ReportUseRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::report_use(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ReportUseSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        let mut response = http::Response::new(
                            tonic::body::Body::default(),
                        );
                        let headers = response.headers_mut();
                        headers
                            .insert(
                                tonic::Status::GRPC_STATUS,
                                (tonic::Code::Unimplemented as i32).into(),
                            );
                        headers
                            .insert(
                                http::header::CONTENT_TYPE,
                                tonic::metadata::GRPC_CONTENT_TYPE,
                            );
                        Ok(response)
                    })
                }
            }
        }
    }
    impl<T> Clone for AuthorityServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    /// Generated gRPC service name
    pub const SERVICE_NAME: &str = "nous.wave.kernel.v1alpha1.AuthorityService";
    impl<T> tonic::server::NamedService for AuthorityServiceServer<T> {
        const NAME: &'static str = SERVICE_NAME;
    }
}
