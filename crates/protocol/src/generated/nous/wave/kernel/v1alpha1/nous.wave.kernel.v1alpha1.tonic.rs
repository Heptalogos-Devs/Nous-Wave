// @generated
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
        async fn find_workflow(
            &self,
            request: tonic::Request<super::FindWorkflowRequest>,
        ) -> std::result::Result<tonic::Response<super::FoundWorkflow>, tonic::Status>;
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
        async fn get_material_limits(
            &self,
            request: tonic::Request<()>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::MaterialLimits>,
            tonic::Status,
        >;
        ///
        async fn list_derived_representations(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::RepresentationListRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::RepresentationListResponse>,
            tonic::Status,
        >;
        ///
        async fn get_producer(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ProducerSignature>,
            tonic::Status,
        >;
        ///
        async fn get_derived_region(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::DerivedRegion>,
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
        async fn create_episode(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::CreateEpisodeRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::EpisodeResponse>,
            tonic::Status,
        >;
        ///
        async fn get_episode(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::EpisodeResponse>,
            tonic::Status,
        >;
        ///
        async fn get_episode_revision(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::EpisodeRevision>,
            tonic::Status,
        >;
        ///
        async fn list_episodes(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ListRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListEpisodesResponse>,
            tonic::Status,
        >;
        ///
        async fn list_episode_revisions(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::ListEpisodeRevisionsRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListEpisodeRevisionsResponse>,
            tonic::Status,
        >;
        ///
        async fn revise_episode(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ReviseEpisodeRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::EpisodeResponse>,
            tonic::Status,
        >;
        ///
        async fn link_episode_revisions(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::LinkEpisodeRevisionsRequest,
            >,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        ///
        async fn suppress_episode(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::EpisodeMutationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::EpisodeResponse>,
            tonic::Status,
        >;
        ///
        async fn restore_episode(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::EpisodeMutationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::EpisodeResponse>,
            tonic::Status,
        >;
        ///
        async fn withdraw_episode(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::EpisodeMutationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::EpisodeResponse>,
            tonic::Status,
        >;
        ///
        async fn reaccept_episode(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::EpisodeMutationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::EpisodeResponse>,
            tonic::Status,
        >;
        ///
        async fn purge_episode(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::EpisodeMutationRequest,
            >,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        ///
        async fn get_journal(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::JournalResponse>,
            tonic::Status,
        >;
        ///
        async fn get_journal_revision(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::JournalRevision>,
            tonic::Status,
        >;
        ///
        async fn list_journals(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ListRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListJournalsResponse>,
            tonic::Status,
        >;
        ///
        async fn list_journal_revisions(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::ListJournalRevisionsRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListJournalRevisionsResponse>,
            tonic::Status,
        >;
        ///
        async fn suppress_journal(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::JournalMutationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::JournalResponse>,
            tonic::Status,
        >;
        ///
        async fn restore_journal(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::JournalMutationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::JournalResponse>,
            tonic::Status,
        >;
        ///
        async fn withdraw_journal(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::JournalMutationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::JournalResponse>,
            tonic::Status,
        >;
        ///
        async fn reaccept_journal(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::JournalMutationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::JournalResponse>,
            tonic::Status,
        >;
        ///
        async fn purge_journal(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::JournalMutationRequest,
            >,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        ///
        async fn set_accessibility(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::SetAccessibilityRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Memory>,
            tonic::Status,
        >;
        ///
        async fn link_revisions(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::LinkRevisionsRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
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
        async fn create_tag(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::CreateTagRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Tag>,
            tonic::Status,
        >;
        ///
        async fn get_tag(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Tag>,
            tonic::Status,
        >;
        ///
        async fn list_tags(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ListRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListTagsResponse>,
            tonic::Status,
        >;
        ///
        async fn create_association(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::CreateAssociationRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Association>,
            tonic::Status,
        >;
        ///
        async fn revoke_association(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::RevokeAssociationRequest,
            >,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        ///
        async fn create_cognitive_schema(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::CreateCognitiveSchemaRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::CognitiveSchema>,
            tonic::Status,
        >;
        ///
        async fn get_cognitive_schema(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::GetCognitiveSchemaRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::CognitiveSchema>,
            tonic::Status,
        >;
        ///
        async fn add_schema_evidence(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::AddSchemaEvidenceRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::CognitiveSchema>,
            tonic::Status,
        >;
        ///
        async fn revise_cognitive_schema(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::ReviseCognitiveSchemaRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::CognitiveSchema>,
            tonic::Status,
        >;
        ///
        async fn split_cognitive_schema(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::SplitCognitiveSchemaRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::SplitCognitiveSchemaResponse>,
            tonic::Status,
        >;
        ///
        async fn merge_cognitive_schemas(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::MergeCognitiveSchemasRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::CognitiveSchema>,
            tonic::Status,
        >;
        ///
        async fn get_neighborhood(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::NeighborhoodRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::NeighborhoodResponse>,
            tonic::Status,
        >;
        ///
        async fn rebind_entity(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::RebindEntityRequest>,
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
        async fn consolidate_memory(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::ConsolidateMemoryRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ConsolidationResponse>,
            tonic::Status,
        >;
        ///
        async fn get_occurrence(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Occurrence>,
            tonic::Status,
        >;
        ///
        async fn get_source_region(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::SourceRegion>,
            tonic::Status,
        >;
        ///
        async fn get_derived_representation(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::DerivedRepresentation>,
            tonic::Status,
        >;
        ///
        async fn bind_identity(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::BindIdentityRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::IdentityBinding>,
            tonic::Status,
        >;
        ///
        async fn resolve_identity(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::ResolveIdentityRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ResolveIdentityResponse>,
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
        async fn create_subject(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::CreateSubjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Subject>,
            tonic::Status,
        >;
        ///
        async fn get_subject(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::SubjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Subject>,
            tonic::Status,
        >;
        ///
        async fn list_subjects(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ListRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListSubjectsResponse>,
            tonic::Status,
        >;
        ///
        async fn get_cognitive_seed(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::SubjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::CognitiveSeedVersion>,
            tonic::Status,
        >;
        ///
        async fn adopt_cognitive_seed(
            &self,
            request: tonic::Request<
                super::super::super::v1alpha1::AdoptCognitiveSeedRequest,
            >,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::CognitiveSeedVersion>,
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
        ///
        async fn get_memory(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Memory>,
            tonic::Status,
        >;
        ///
        async fn list_memories(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ListRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListMemoriesResponse>,
            tonic::Status,
        >;
        ///
        async fn get_memory_revision(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Memory>,
            tonic::Status,
        >;
        ///
        async fn list_memory_revisions(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::MemoryHistoryRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListMemoriesResponse>,
            tonic::Status,
        >;
        ///
        async fn form_memory(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::FormMemoryRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Memory>,
            tonic::Status,
        >;
        ///
        async fn revise_memory(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ReviseMemoryRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Memory>,
            tonic::Status,
        >;
        ///
        async fn suppress_memory(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::MemoryMutationRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Memory>,
            tonic::Status,
        >;
        ///
        async fn restore_memory(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::MemoryMutationRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Memory>,
            tonic::Status,
        >;
        ///
        async fn purge_memory(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::MemoryMutationRequest>,
        ) -> std::result::Result<tonic::Response<()>, tonic::Status>;
        ///
        async fn withdraw_memory(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::MemoryMutationRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Memory>,
            tonic::Status,
        >;
        ///
        async fn reaccept_memory(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::MemoryMutationRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Memory>,
            tonic::Status,
        >;
        ///
        async fn get_artifact(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ObjectRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::Artifact>,
            tonic::Status,
        >;
        ///
        async fn list_artifacts(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::ListRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::ListArtifactsResponse>,
            tonic::Status,
        >;
        ///
        async fn materialize_evidence(
            &self,
            request: tonic::Request<super::super::super::v1alpha1::MaterializeRequest>,
        ) -> std::result::Result<
            tonic::Response<super::super::super::v1alpha1::MaterializedEvidence>,
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetMaterialLimits" => {
                    #[allow(non_camel_case_types)]
                    struct GetMaterialLimitsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<T: AuthorityService> tonic::server::UnaryService<()>
                    for GetMaterialLimitsSvc<T> {
                        type Response = super::super::super::v1alpha1::MaterialLimits;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(&mut self, request: tonic::Request<()>) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::get_material_limits(
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
                        let method = GetMaterialLimitsSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListDerivedRepresentations" => {
                    #[allow(non_camel_case_types)]
                    struct ListDerivedRepresentationsSvc<T: AuthorityService>(
                        pub Arc<T>,
                    );
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::RepresentationListRequest,
                    > for ListDerivedRepresentationsSvc<T> {
                        type Response = super::super::super::v1alpha1::RepresentationListResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::RepresentationListRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::list_derived_representations(
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
                        let method = ListDerivedRepresentationsSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetProducer" => {
                    #[allow(non_camel_case_types)]
                    struct GetProducerSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetProducerSvc<T> {
                        type Response = super::super::super::v1alpha1::ProducerSignature;
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
                                <T as AuthorityService>::get_producer(&inner, request).await
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
                        let method = GetProducerSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetDerivedRegion" => {
                    #[allow(non_camel_case_types)]
                    struct GetDerivedRegionSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetDerivedRegionSvc<T> {
                        type Response = super::super::super::v1alpha1::DerivedRegion;
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
                                <T as AuthorityService>::get_derived_region(&inner, request)
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
                        let method = GetDerivedRegionSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/CreateEpisode" => {
                    #[allow(non_camel_case_types)]
                    struct CreateEpisodeSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::CreateEpisodeRequest,
                    > for CreateEpisodeSvc<T> {
                        type Response = super::super::super::v1alpha1::EpisodeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::CreateEpisodeRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::create_episode(&inner, request)
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
                        let method = CreateEpisodeSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetEpisode" => {
                    #[allow(non_camel_case_types)]
                    struct GetEpisodeSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetEpisodeSvc<T> {
                        type Response = super::super::super::v1alpha1::EpisodeResponse;
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
                                <T as AuthorityService>::get_episode(&inner, request).await
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
                        let method = GetEpisodeSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetEpisodeRevision" => {
                    #[allow(non_camel_case_types)]
                    struct GetEpisodeRevisionSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetEpisodeRevisionSvc<T> {
                        type Response = super::super::super::v1alpha1::EpisodeRevision;
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
                                <T as AuthorityService>::get_episode_revision(
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
                        let method = GetEpisodeRevisionSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListEpisodes" => {
                    #[allow(non_camel_case_types)]
                    struct ListEpisodesSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ListRequest,
                    > for ListEpisodesSvc<T> {
                        type Response = super::super::super::v1alpha1::ListEpisodesResponse;
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
                                <T as AuthorityService>::list_episodes(&inner, request)
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
                        let method = ListEpisodesSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListEpisodeRevisions" => {
                    #[allow(non_camel_case_types)]
                    struct ListEpisodeRevisionsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ListEpisodeRevisionsRequest,
                    > for ListEpisodeRevisionsSvc<T> {
                        type Response = super::super::super::v1alpha1::ListEpisodeRevisionsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ListEpisodeRevisionsRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::list_episode_revisions(
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
                        let method = ListEpisodeRevisionsSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ReviseEpisode" => {
                    #[allow(non_camel_case_types)]
                    struct ReviseEpisodeSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ReviseEpisodeRequest,
                    > for ReviseEpisodeSvc<T> {
                        type Response = super::super::super::v1alpha1::EpisodeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ReviseEpisodeRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::revise_episode(&inner, request)
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
                        let method = ReviseEpisodeSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/LinkEpisodeRevisions" => {
                    #[allow(non_camel_case_types)]
                    struct LinkEpisodeRevisionsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::LinkEpisodeRevisionsRequest,
                    > for LinkEpisodeRevisionsSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::LinkEpisodeRevisionsRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::link_episode_revisions(
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
                        let method = LinkEpisodeRevisionsSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/SuppressEpisode" => {
                    #[allow(non_camel_case_types)]
                    struct SuppressEpisodeSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::EpisodeMutationRequest,
                    > for SuppressEpisodeSvc<T> {
                        type Response = super::super::super::v1alpha1::EpisodeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::EpisodeMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::suppress_episode(&inner, request)
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
                        let method = SuppressEpisodeSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/RestoreEpisode" => {
                    #[allow(non_camel_case_types)]
                    struct RestoreEpisodeSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::EpisodeMutationRequest,
                    > for RestoreEpisodeSvc<T> {
                        type Response = super::super::super::v1alpha1::EpisodeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::EpisodeMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::restore_episode(&inner, request)
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
                        let method = RestoreEpisodeSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/WithdrawEpisode" => {
                    #[allow(non_camel_case_types)]
                    struct WithdrawEpisodeSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::EpisodeMutationRequest,
                    > for WithdrawEpisodeSvc<T> {
                        type Response = super::super::super::v1alpha1::EpisodeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::EpisodeMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::withdraw_episode(&inner, request)
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
                        let method = WithdrawEpisodeSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ReacceptEpisode" => {
                    #[allow(non_camel_case_types)]
                    struct ReacceptEpisodeSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::EpisodeMutationRequest,
                    > for ReacceptEpisodeSvc<T> {
                        type Response = super::super::super::v1alpha1::EpisodeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::EpisodeMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::reaccept_episode(&inner, request)
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
                        let method = ReacceptEpisodeSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/PurgeEpisode" => {
                    #[allow(non_camel_case_types)]
                    struct PurgeEpisodeSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::EpisodeMutationRequest,
                    > for PurgeEpisodeSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::EpisodeMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::purge_episode(&inner, request)
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
                        let method = PurgeEpisodeSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetJournal" => {
                    #[allow(non_camel_case_types)]
                    struct GetJournalSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetJournalSvc<T> {
                        type Response = super::super::super::v1alpha1::JournalResponse;
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
                                <T as AuthorityService>::get_journal(&inner, request).await
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
                        let method = GetJournalSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetJournalRevision" => {
                    #[allow(non_camel_case_types)]
                    struct GetJournalRevisionSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetJournalRevisionSvc<T> {
                        type Response = super::super::super::v1alpha1::JournalRevision;
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
                                <T as AuthorityService>::get_journal_revision(
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
                        let method = GetJournalRevisionSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListJournals" => {
                    #[allow(non_camel_case_types)]
                    struct ListJournalsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ListRequest,
                    > for ListJournalsSvc<T> {
                        type Response = super::super::super::v1alpha1::ListJournalsResponse;
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
                                <T as AuthorityService>::list_journals(&inner, request)
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
                        let method = ListJournalsSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListJournalRevisions" => {
                    #[allow(non_camel_case_types)]
                    struct ListJournalRevisionsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ListJournalRevisionsRequest,
                    > for ListJournalRevisionsSvc<T> {
                        type Response = super::super::super::v1alpha1::ListJournalRevisionsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ListJournalRevisionsRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::list_journal_revisions(
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
                        let method = ListJournalRevisionsSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/SuppressJournal" => {
                    #[allow(non_camel_case_types)]
                    struct SuppressJournalSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::JournalMutationRequest,
                    > for SuppressJournalSvc<T> {
                        type Response = super::super::super::v1alpha1::JournalResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::JournalMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::suppress_journal(&inner, request)
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
                        let method = SuppressJournalSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/RestoreJournal" => {
                    #[allow(non_camel_case_types)]
                    struct RestoreJournalSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::JournalMutationRequest,
                    > for RestoreJournalSvc<T> {
                        type Response = super::super::super::v1alpha1::JournalResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::JournalMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::restore_journal(&inner, request)
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
                        let method = RestoreJournalSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/WithdrawJournal" => {
                    #[allow(non_camel_case_types)]
                    struct WithdrawJournalSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::JournalMutationRequest,
                    > for WithdrawJournalSvc<T> {
                        type Response = super::super::super::v1alpha1::JournalResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::JournalMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::withdraw_journal(&inner, request)
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
                        let method = WithdrawJournalSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ReacceptJournal" => {
                    #[allow(non_camel_case_types)]
                    struct ReacceptJournalSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::JournalMutationRequest,
                    > for ReacceptJournalSvc<T> {
                        type Response = super::super::super::v1alpha1::JournalResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::JournalMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::reaccept_journal(&inner, request)
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
                        let method = ReacceptJournalSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/PurgeJournal" => {
                    #[allow(non_camel_case_types)]
                    struct PurgeJournalSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::JournalMutationRequest,
                    > for PurgeJournalSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::JournalMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::purge_journal(&inner, request)
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
                        let method = PurgeJournalSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/SetAccessibility" => {
                    #[allow(non_camel_case_types)]
                    struct SetAccessibilitySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::SetAccessibilityRequest,
                    > for SetAccessibilitySvc<T> {
                        type Response = super::super::super::v1alpha1::Memory;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::SetAccessibilityRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::set_accessibility(&inner, request)
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
                        let method = SetAccessibilitySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/LinkRevisions" => {
                    #[allow(non_camel_case_types)]
                    struct LinkRevisionsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::LinkRevisionsRequest,
                    > for LinkRevisionsSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::LinkRevisionsRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::link_revisions(&inner, request)
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
                        let method = LinkRevisionsSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/CreateTag" => {
                    #[allow(non_camel_case_types)]
                    struct CreateTagSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::CreateTagRequest,
                    > for CreateTagSvc<T> {
                        type Response = super::super::super::v1alpha1::Tag;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::CreateTagRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::create_tag(&inner, request).await
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
                        let method = CreateTagSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetTag" => {
                    #[allow(non_camel_case_types)]
                    struct GetTagSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetTagSvc<T> {
                        type Response = super::super::super::v1alpha1::Tag;
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
                                <T as AuthorityService>::get_tag(&inner, request).await
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
                        let method = GetTagSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListTags" => {
                    #[allow(non_camel_case_types)]
                    struct ListTagsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ListRequest,
                    > for ListTagsSvc<T> {
                        type Response = super::super::super::v1alpha1::ListTagsResponse;
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
                                <T as AuthorityService>::list_tags(&inner, request).await
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
                        let method = ListTagsSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/CreateAssociation" => {
                    #[allow(non_camel_case_types)]
                    struct CreateAssociationSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::CreateAssociationRequest,
                    > for CreateAssociationSvc<T> {
                        type Response = super::super::super::v1alpha1::Association;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::CreateAssociationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::create_association(&inner, request)
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
                        let method = CreateAssociationSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/RevokeAssociation" => {
                    #[allow(non_camel_case_types)]
                    struct RevokeAssociationSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::RevokeAssociationRequest,
                    > for RevokeAssociationSvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::RevokeAssociationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::revoke_association(&inner, request)
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
                        let method = RevokeAssociationSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/CreateCognitiveSchema" => {
                    #[allow(non_camel_case_types)]
                    struct CreateCognitiveSchemaSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::CreateCognitiveSchemaRequest,
                    > for CreateCognitiveSchemaSvc<T> {
                        type Response = super::super::super::v1alpha1::CognitiveSchema;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::CreateCognitiveSchemaRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::create_cognitive_schema(
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
                        let method = CreateCognitiveSchemaSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetCognitiveSchema" => {
                    #[allow(non_camel_case_types)]
                    struct GetCognitiveSchemaSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::GetCognitiveSchemaRequest,
                    > for GetCognitiveSchemaSvc<T> {
                        type Response = super::super::super::v1alpha1::CognitiveSchema;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::GetCognitiveSchemaRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::get_cognitive_schema(
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
                        let method = GetCognitiveSchemaSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/AddSchemaEvidence" => {
                    #[allow(non_camel_case_types)]
                    struct AddSchemaEvidenceSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::AddSchemaEvidenceRequest,
                    > for AddSchemaEvidenceSvc<T> {
                        type Response = super::super::super::v1alpha1::CognitiveSchema;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::AddSchemaEvidenceRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::add_schema_evidence(
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
                        let method = AddSchemaEvidenceSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ReviseCognitiveSchema" => {
                    #[allow(non_camel_case_types)]
                    struct ReviseCognitiveSchemaSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ReviseCognitiveSchemaRequest,
                    > for ReviseCognitiveSchemaSvc<T> {
                        type Response = super::super::super::v1alpha1::CognitiveSchema;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ReviseCognitiveSchemaRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::revise_cognitive_schema(
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
                        let method = ReviseCognitiveSchemaSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/SplitCognitiveSchema" => {
                    #[allow(non_camel_case_types)]
                    struct SplitCognitiveSchemaSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::SplitCognitiveSchemaRequest,
                    > for SplitCognitiveSchemaSvc<T> {
                        type Response = super::super::super::v1alpha1::SplitCognitiveSchemaResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::SplitCognitiveSchemaRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::split_cognitive_schema(
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
                        let method = SplitCognitiveSchemaSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/MergeCognitiveSchemas" => {
                    #[allow(non_camel_case_types)]
                    struct MergeCognitiveSchemasSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::MergeCognitiveSchemasRequest,
                    > for MergeCognitiveSchemasSvc<T> {
                        type Response = super::super::super::v1alpha1::CognitiveSchema;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::MergeCognitiveSchemasRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::merge_cognitive_schemas(
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
                        let method = MergeCognitiveSchemasSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetNeighborhood" => {
                    #[allow(non_camel_case_types)]
                    struct GetNeighborhoodSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::NeighborhoodRequest,
                    > for GetNeighborhoodSvc<T> {
                        type Response = super::super::super::v1alpha1::NeighborhoodResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::NeighborhoodRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::get_neighborhood(&inner, request)
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
                        let method = GetNeighborhoodSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/RebindEntity" => {
                    #[allow(non_camel_case_types)]
                    struct RebindEntitySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::RebindEntityRequest,
                    > for RebindEntitySvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::RebindEntityRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::rebind_entity(&inner, request)
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
                        let method = RebindEntitySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ConsolidateMemory" => {
                    #[allow(non_camel_case_types)]
                    struct ConsolidateMemorySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ConsolidateMemoryRequest,
                    > for ConsolidateMemorySvc<T> {
                        type Response = super::super::super::v1alpha1::ConsolidationResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ConsolidateMemoryRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::consolidate_memory(&inner, request)
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
                        let method = ConsolidateMemorySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetOccurrence" => {
                    #[allow(non_camel_case_types)]
                    struct GetOccurrenceSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetOccurrenceSvc<T> {
                        type Response = super::super::super::v1alpha1::Occurrence;
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
                                <T as AuthorityService>::get_occurrence(&inner, request)
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
                        let method = GetOccurrenceSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetSourceRegion" => {
                    #[allow(non_camel_case_types)]
                    struct GetSourceRegionSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetSourceRegionSvc<T> {
                        type Response = super::super::super::v1alpha1::SourceRegion;
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
                                <T as AuthorityService>::get_source_region(&inner, request)
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
                        let method = GetSourceRegionSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetDerivedRepresentation" => {
                    #[allow(non_camel_case_types)]
                    struct GetDerivedRepresentationSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetDerivedRepresentationSvc<T> {
                        type Response = super::super::super::v1alpha1::DerivedRepresentation;
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
                                <T as AuthorityService>::get_derived_representation(
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
                        let method = GetDerivedRepresentationSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/BindIdentity" => {
                    #[allow(non_camel_case_types)]
                    struct BindIdentitySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::BindIdentityRequest,
                    > for BindIdentitySvc<T> {
                        type Response = super::super::super::v1alpha1::IdentityBinding;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::BindIdentityRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::bind_identity(&inner, request)
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
                        let method = BindIdentitySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ResolveIdentity" => {
                    #[allow(non_camel_case_types)]
                    struct ResolveIdentitySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ResolveIdentityRequest,
                    > for ResolveIdentitySvc<T> {
                        type Response = super::super::super::v1alpha1::ResolveIdentityResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ResolveIdentityRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::resolve_identity(&inner, request)
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
                        let method = ResolveIdentitySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/CreateSubject" => {
                    #[allow(non_camel_case_types)]
                    struct CreateSubjectSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::CreateSubjectRequest,
                    > for CreateSubjectSvc<T> {
                        type Response = super::super::super::v1alpha1::Subject;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::CreateSubjectRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::create_subject(&inner, request)
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
                        let method = CreateSubjectSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetSubject" => {
                    #[allow(non_camel_case_types)]
                    struct GetSubjectSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::SubjectRequest,
                    > for GetSubjectSvc<T> {
                        type Response = super::super::super::v1alpha1::Subject;
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
                                <T as AuthorityService>::get_subject(&inner, request).await
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
                        let method = GetSubjectSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListSubjects" => {
                    #[allow(non_camel_case_types)]
                    struct ListSubjectsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ListRequest,
                    > for ListSubjectsSvc<T> {
                        type Response = super::super::super::v1alpha1::ListSubjectsResponse;
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
                                <T as AuthorityService>::list_subjects(&inner, request)
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
                        let method = ListSubjectsSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetCognitiveSeed" => {
                    #[allow(non_camel_case_types)]
                    struct GetCognitiveSeedSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::SubjectRequest,
                    > for GetCognitiveSeedSvc<T> {
                        type Response = super::super::super::v1alpha1::CognitiveSeedVersion;
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
                                <T as AuthorityService>::get_cognitive_seed(&inner, request)
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
                        let method = GetCognitiveSeedSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/AdoptCognitiveSeed" => {
                    #[allow(non_camel_case_types)]
                    struct AdoptCognitiveSeedSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::AdoptCognitiveSeedRequest,
                    > for AdoptCognitiveSeedSvc<T> {
                        type Response = super::super::super::v1alpha1::CognitiveSeedVersion;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::AdoptCognitiveSeedRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::adopt_cognitive_seed(
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
                        let method = AdoptCognitiveSeedSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetMemory" => {
                    #[allow(non_camel_case_types)]
                    struct GetMemorySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetMemorySvc<T> {
                        type Response = super::super::super::v1alpha1::Memory;
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
                                <T as AuthorityService>::get_memory(&inner, request).await
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
                        let method = GetMemorySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListMemories" => {
                    #[allow(non_camel_case_types)]
                    struct ListMemoriesSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ListRequest,
                    > for ListMemoriesSvc<T> {
                        type Response = super::super::super::v1alpha1::ListMemoriesResponse;
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
                                <T as AuthorityService>::list_memories(&inner, request)
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
                        let method = ListMemoriesSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetMemoryRevision" => {
                    #[allow(non_camel_case_types)]
                    struct GetMemoryRevisionSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetMemoryRevisionSvc<T> {
                        type Response = super::super::super::v1alpha1::Memory;
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
                                <T as AuthorityService>::get_memory_revision(
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
                        let method = GetMemoryRevisionSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListMemoryRevisions" => {
                    #[allow(non_camel_case_types)]
                    struct ListMemoryRevisionsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::MemoryHistoryRequest,
                    > for ListMemoryRevisionsSvc<T> {
                        type Response = super::super::super::v1alpha1::ListMemoriesResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::MemoryHistoryRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::list_memory_revisions(
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
                        let method = ListMemoryRevisionsSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/FormMemory" => {
                    #[allow(non_camel_case_types)]
                    struct FormMemorySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::FormMemoryRequest,
                    > for FormMemorySvc<T> {
                        type Response = super::super::super::v1alpha1::Memory;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::FormMemoryRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::form_memory(&inner, request).await
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
                        let method = FormMemorySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ReviseMemory" => {
                    #[allow(non_camel_case_types)]
                    struct ReviseMemorySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ReviseMemoryRequest,
                    > for ReviseMemorySvc<T> {
                        type Response = super::super::super::v1alpha1::Memory;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::ReviseMemoryRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::revise_memory(&inner, request)
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
                        let method = ReviseMemorySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/SuppressMemory" => {
                    #[allow(non_camel_case_types)]
                    struct SuppressMemorySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::MemoryMutationRequest,
                    > for SuppressMemorySvc<T> {
                        type Response = super::super::super::v1alpha1::Memory;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::MemoryMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::suppress_memory(&inner, request)
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
                        let method = SuppressMemorySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/RestoreMemory" => {
                    #[allow(non_camel_case_types)]
                    struct RestoreMemorySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::MemoryMutationRequest,
                    > for RestoreMemorySvc<T> {
                        type Response = super::super::super::v1alpha1::Memory;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::MemoryMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::restore_memory(&inner, request)
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
                        let method = RestoreMemorySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/PurgeMemory" => {
                    #[allow(non_camel_case_types)]
                    struct PurgeMemorySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::MemoryMutationRequest,
                    > for PurgeMemorySvc<T> {
                        type Response = ();
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::MemoryMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::purge_memory(&inner, request).await
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
                        let method = PurgeMemorySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/WithdrawMemory" => {
                    #[allow(non_camel_case_types)]
                    struct WithdrawMemorySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::MemoryMutationRequest,
                    > for WithdrawMemorySvc<T> {
                        type Response = super::super::super::v1alpha1::Memory;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::MemoryMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::withdraw_memory(&inner, request)
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
                        let method = WithdrawMemorySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ReacceptMemory" => {
                    #[allow(non_camel_case_types)]
                    struct ReacceptMemorySvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::MemoryMutationRequest,
                    > for ReacceptMemorySvc<T> {
                        type Response = super::super::super::v1alpha1::Memory;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::MemoryMutationRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::reaccept_memory(&inner, request)
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
                        let method = ReacceptMemorySvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/GetArtifact" => {
                    #[allow(non_camel_case_types)]
                    struct GetArtifactSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ObjectRequest,
                    > for GetArtifactSvc<T> {
                        type Response = super::super::super::v1alpha1::Artifact;
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
                                <T as AuthorityService>::get_artifact(&inner, request).await
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
                        let method = GetArtifactSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/ListArtifacts" => {
                    #[allow(non_camel_case_types)]
                    struct ListArtifactsSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::ListRequest,
                    > for ListArtifactsSvc<T> {
                        type Response = super::super::super::v1alpha1::ListArtifactsResponse;
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
                                <T as AuthorityService>::list_artifacts(&inner, request)
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
                        let method = ListArtifactsSvc(inner);
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
                "/nous.wave.kernel.v1alpha1.AuthorityService/MaterializeEvidence" => {
                    #[allow(non_camel_case_types)]
                    struct MaterializeEvidenceSvc<T: AuthorityService>(pub Arc<T>);
                    impl<
                        T: AuthorityService,
                    > tonic::server::UnaryService<
                        super::super::super::v1alpha1::MaterializeRequest,
                    > for MaterializeEvidenceSvc<T> {
                        type Response = super::super::super::v1alpha1::MaterializedEvidence;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::super::super::v1alpha1::MaterializeRequest,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as AuthorityService>::materialize_evidence(
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
                        let method = MaterializeEvidenceSvc(inner);
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
