//! Static composition of Subject Core, Cognitive Runtime and optional Memory.
mod context;
pub mod transport;

use nous_configuration::{
    ConfigRegistryBuilder, ConfigurationService, ProcessCapabilities, process_capabilities,
};
use nous_core::*;
use nous_material::MaterialService;
use nous_memory::MemoryService;
use nous_object_store::ObjectStore;
use nous_persistence::AuthorityStore;
use nous_retrieval::{ServingOptions, ServingService, TextEmbeddingProvider};
use nous_runtime::CognitiveRuntimeService;
use nous_subject::SubjectCoreService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct NousRuntime {
    pub configuration: ConfigurationService,
    pub store: AuthorityStore,
    pub subjects: SubjectCoreService,
    pub cognition: CognitiveRuntimeService,
    pub memory: Option<MemoryService>,
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
    pub stored_embedding: Option<nous_retrieval::StoredEmbeddingConfig>,
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
        request: nous_material::MaterializeRequest,
    ) -> Result<nous_material::MaterializedEvidence> {
        self.material.materialize(subject, request).await
    }
    pub async fn open(options: RuntimeOptions) -> Result<Self> {
        let store = AuthorityStore::connect(&options.postgres_url, options.max_connections).await?;
        store.migrate().await?;
        let mut registry = ConfigRegistryBuilder::new();
        nous_configuration::register_configuration(&mut registry)?;
        nous_runtime::register_configuration(&mut registry)?;
        nous_memory::register_configuration(&mut registry)?;
        nous_retrieval::register_configuration(&mut registry)?;
        nous_retrieval::register_wave_configuration(&mut registry)?;
        let configuration = ConfigurationService::open(
            store.clone(),
            registry.finish()?,
            options.deployment_settings,
        )
        .await?;
        let system_snapshot = configuration.active_system_snapshot()?;
        let process_capabilities = process_capabilities(&system_snapshot)?;
        let resident_limit = system_snapshot.get(nous_runtime::RESIDENT_LIMIT_KEY)?;
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
        serving_options.lexical = system_snapshot.get(nous_retrieval::LEXICAL_ENABLED_KEY)?;
        serving_options.dense = system_snapshot.get(nous_retrieval::DENSE_ENABLED_KEY)?;
        serving_options.topology = system_snapshot.get(nous_retrieval::TOPOLOGY_ENABLED_KEY)?;
        let serving = ServingService::new(
            store.clone(),
            objects.clone(),
            serving_options,
            match options.stored_embedding {
                Some(config) => Some(Arc::new(nous_retrieval::StoredEmbeddingProvider::new(
                    store.clone(),
                    config,
                )?)),
                None => options.embedding,
            },
            configuration.clone(),
        )?;
        let memory = process_capabilities.memory.then(|| {
            MemoryService::new(
                store.clone(),
                objects.clone(),
                cognition.clone(),
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
        Ok(self.execute_query(query, None).await?.result)
    }
    pub async fn execute_query(
        &self,
        query: CognitiveQuery,
        pool_limit: Option<usize>,
    ) -> Result<nous_runtime::QueryExecution> {
        let mut bound = self.cognition.bind_query(query).await?;
        bound.selected_embedding_space = self
            .serving
            .embedding
            .as_ref()
            .map(|provider| provider.space());
        let plan = nous_runtime::QueryPlan::for_bound_query(&bound);
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
        let mut execution = self
            .cognition
            .execute_query_with_plan(
                bound,
                nous_runtime::CognitiveContributors {
                    shared: Some(&self.serving),
                    memory: subject_capabilities
                        .memory
                        .then(|| {
                            self.memory
                                .as_ref()
                                .map(|memory| memory as &dyn nous_runtime::CognitiveContributor)
                        })
                        .flatten(),
                },
                plan,
                pool_limit,
            )
            .await?;
        let result = &mut execution.result;
        result.degradation.extend(projection.degradation);
        if !result.degradation.is_empty() && result.status != QueryStatus::Partial {
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
        Ok(execution)
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
}
