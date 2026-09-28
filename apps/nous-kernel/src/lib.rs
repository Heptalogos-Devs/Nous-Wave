//! Static composition of Subject Core, Cognitive Runtime and optional Memory.
mod context;
pub mod transport;

use nous_authority_store::AuthorityStore;
use nous_cognitive_runtime::CognitiveRuntimeService;
use nous_configuration_service::{
    ConfigRegistryBuilder, ConfigurationService, ProcessCapabilities, process_capabilities,
};
use nous_core::*;
use nous_material_service::MaterialService;
use nous_memory_service::MemoryService;
use nous_object_store::ObjectStore;
use nous_self_service::SelfService;
use nous_serving::{ServingOptions, ServingService, TextEmbeddingProvider};
use nous_social_service::SocialService;
use nous_subject_core::SubjectCoreService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct NousRuntime {
    pub configuration: ConfigurationService,
    pub store: AuthorityStore,
    pub subjects: SubjectCoreService,
    pub cognition: CognitiveRuntimeService,
    pub memory: Option<MemoryService>,
    pub self_cognition: Option<SelfService>,
    pub social: Option<SocialService>,
    pub process_capabilities: ProcessCapabilities,
    pub material: MaterialService,
    pub serving: ServingService,
}

pub struct RuntimeOptions {
    pub postgres_url: String,
    pub max_connections: u32,
    pub object_root: String,
    pub max_upload_bytes: u64,
    pub serving_options: ServingOptions,
    pub embedding: Option<Arc<dyn TextEmbeddingProvider>>,
    pub stored_embedding: Option<nous_serving::StoredEmbeddingConfig>,
    pub deployment_settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub api_version: u32,
    pub ready: bool,
    pub capabilities: Vec<CapabilityStatus>,
}

impl NousRuntime {
    pub async fn materialize(
        &self,
        subject: SubjectId,
        request: nous_material_service::MaterializeRequest,
    ) -> Result<nous_material_service::MaterializedEvidence> {
        self.material.materialize(subject, request).await
    }
    pub async fn open(options: RuntimeOptions) -> Result<Self> {
        let store = AuthorityStore::connect(&options.postgres_url, options.max_connections).await?;
        store.migrate().await?;
        let mut registry = ConfigRegistryBuilder::new();
        nous_configuration_service::register_configuration(&mut registry)?;
        nous_cognitive_runtime::register_configuration(&mut registry)?;
        nous_memory_service::register_configuration(&mut registry)?;
        nous_cognitive_retrieval::register_configuration(&mut registry)?;
        nous_cognitive_retrieval::register_wave_configuration(&mut registry)?;
        nous_serving::register_configuration(&mut registry)?;
        nous_social_service::register_configuration(&mut registry)?;
        let configuration = ConfigurationService::open(
            store.clone(),
            registry.finish()?,
            options.deployment_settings,
        )
        .await?;
        let system_snapshot = configuration.active_system_snapshot()?;
        let process_capabilities = process_capabilities(&system_snapshot)?;
        let resident_limit = system_snapshot.get(nous_cognitive_runtime::RESIDENT_LIMIT_KEY)?;
        let objects = ObjectStore::open(&options.object_root).await?;
        let subjects =
            SubjectCoreService::new(store.clone(), objects.clone(), configuration.clone());
        let cognition =
            CognitiveRuntimeService::new(store.clone(), resident_limit, configuration.clone())?;
        let material = MaterialService::new(
            store.clone(),
            objects.clone(),
            cognition.clone(),
            options.max_upload_bytes,
        )?;
        let mut serving_options = options.serving_options;
        serving_options.memory_enabled = process_capabilities.memory;
        serving_options.self_enabled = process_capabilities.self_cognition;
        serving_options.social_enabled = process_capabilities.social;
        serving_options.lexical = system_snapshot.get(nous_serving::LEXICAL_ENABLED_KEY)?;
        serving_options.dense = system_snapshot.get(nous_serving::DENSE_ENABLED_KEY)?;
        serving_options.topology = system_snapshot.get(nous_serving::TOPOLOGY_ENABLED_KEY)?;
        let serving = ServingService::new(
            store.clone(),
            objects.clone(),
            serving_options,
            match options.stored_embedding {
                Some(config) => Some(Arc::new(nous_serving::StoredEmbeddingProvider::new(
                    store.clone(),
                    config,
                )?)),
                None => options.embedding,
            },
            configuration.clone(),
        )?;
        let self_cognition = process_capabilities
            .self_cognition
            .then(|| SelfService::new(store.clone()));
        let social = process_capabilities
            .social
            .then(|| SocialService::new(store.clone(), configuration.clone()));
        let memory = process_capabilities.memory.then(|| {
            MemoryService::new(
                store.clone(),
                objects.clone(),
                cognition.clone(),
                serving.clone(),
                configuration.clone(),
            )
        });
        for subject in store.active_subjects().await? {
            let status = serving.refresh(subject).await?;
            for degradation in status.degradation {
                tracing::warn!(code=%degradation.code, detail=?degradation.detail, "serving projection degraded");
            }
        }
        Ok(Self {
            configuration,
            store,
            subjects,
            cognition,
            memory,
            self_cognition,
            social,
            process_capabilities,
            material,
            serving,
        })
    }

    pub fn require_memory(&self) -> Result<&MemoryService> {
        self.memory
            .as_ref()
            .ok_or_else(|| Error::Unavailable("Memory MicroSystem is disabled".into()))
    }

    pub async fn query(&self, query: CognitiveQuery) -> Result<CognitiveQueryResult> {
        let bound = self.cognition.bind_query(query).await?;
        let plan = nous_cognitive_runtime::QueryPlan::for_bound_query(&bound);
        let subject_capabilities = self
            .subjects
            .subject(bound.source_query.subject)
            .await?
            .capabilities;
        let projection = self
            .serving
            .prepare_with_snapshot(
                bound.source_query.subject,
                plan.serving_need(&bound.source_query),
                &bound.config_snapshot,
            )
            .await?;
        let mut result = self
            .cognition
            .query_with_plan(
                bound,
                nous_cognitive_runtime::CognitiveContributors {
                    shared: Some(&self.serving),
                    memory: subject_capabilities
                        .memory
                        .then(|| {
                            self.memory.as_ref().map(|memory| {
                                memory as &dyn nous_cognitive_runtime::CognitiveContributor
                            })
                        })
                        .flatten(),
                    self_cognition: subject_capabilities
                        .self_cognition
                        .then(|| {
                            self.self_cognition.as_ref().map(|service| {
                                service as &dyn nous_cognitive_runtime::CognitiveContributor
                            })
                        })
                        .flatten(),
                    social: subject_capabilities
                        .social
                        .then(|| {
                            self.social.as_ref().map(|service| {
                                service as &dyn nous_cognitive_runtime::CognitiveContributor
                            })
                        })
                        .flatten(),
                },
                plan,
            )
            .await?;
        result.degradation.extend(projection.degradation);
        if !result.degradation.is_empty() {
            result.status = if result
                .degradation
                .iter()
                .any(|d| d.code == "required_external_authority_unresolved")
            {
                QueryStatus::Partial
            } else {
                QueryStatus::Degraded
            };
        }
        Ok(result)
    }

    pub async fn status(&self) -> RuntimeStatus {
        let mut capabilities = vec![
            CapabilityStatus {
                capability_id: "subject_core".into(),
                status: Readiness::Ready,
                reason: None,
            },
            CapabilityStatus {
                capability_id: "cognitive_runtime".into(),
                status: Readiness::Ready,
                reason: None,
            },
            CapabilityStatus {
                capability_id: "memory".into(),
                status: if self.memory.is_some() {
                    Readiness::Ready
                } else {
                    Readiness::Unavailable
                },
                reason: self
                    .memory
                    .is_none()
                    .then(|| "disabled in composition".into()),
            },
            CapabilityStatus {
                capability_id: "self_cognition".into(),
                status: if self.self_cognition.is_some() {
                    Readiness::Ready
                } else {
                    Readiness::Unavailable
                },
                reason: self
                    .self_cognition
                    .is_none()
                    .then(|| "disabled in process composition".into()),
            },
            CapabilityStatus {
                capability_id: "social".into(),
                status: if self.social.is_some() {
                    Readiness::Ready
                } else {
                    Readiness::Unavailable
                },
                reason: self
                    .social
                    .is_none()
                    .then(|| "disabled in process composition".into()),
            },
        ];
        if let Some(memory) = &self.memory {
            capabilities.extend(memory.status().await.capabilities);
        }
        RuntimeStatus {
            api_version: API_VERSION,
            ready: self.store.check().await.is_ok(),
            capabilities,
        }
    }

    pub fn require_self(&self) -> Result<&SelfService> {
        self.self_cognition
            .as_ref()
            .ok_or_else(|| Error::Unavailable("Self Authority is disabled".into()))
    }
}
