//! Immutable on-disk serving generations shared by runtime and cognitive contributors.
mod artifacts;
mod build;
mod lifecycle;
mod material;
mod provider;
pub use material::*;

use nous_authority_store::{AuthorityStore, ServingRecord};
use nous_cognitive_retrieval::*;
use nous_core::*;
use nous_object_store::ObjectStore;
pub use provider::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

pub const LEXICAL_ENABLED_KEY: nous_configuration_service::ConfigKey<bool> =
    nous_configuration_service::ConfigKey::new("serving.lexical.enabled");
pub const DENSE_ENABLED_KEY: nous_configuration_service::ConfigKey<bool> =
    nous_configuration_service::ConfigKey::new("serving.dense.enabled");
pub const TOPOLOGY_ENABLED_KEY: nous_configuration_service::ConfigKey<bool> =
    nous_configuration_service::ConfigKey::new("serving.topology.enabled");

pub fn register_configuration(
    registry: &mut nous_configuration_service::ConfigRegistryBuilder,
) -> Result<()> {
    for (key, description) in [
        (LEXICAL_ENABLED_KEY, "Whether lexical Serving is enabled."),
        (DENSE_ENABLED_KEY, "Whether dense Serving is enabled."),
        (TOPOLOGY_ENABLED_KEY, "Whether topology Serving is enabled."),
    ] {
        registry.register(
            key,
            "serving",
            description,
            true,
            nous_configuration_service::ConfigExposure::Developer,
            nous_configuration_service::ConfigScopePolicy::SystemOnly,
            nous_configuration_service::ConfigApplyMode::RestartProcess,
            nous_configuration_service::ConfigSemanticEffect::Operational,
            |_: &bool| Ok(()),
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServingOptions {
    pub root: PathBuf,
    pub lexical: bool,
    pub dense: bool,
    pub topology: bool,
    pub memory_enabled: bool,
    pub self_enabled: bool,
    pub social_enabled: bool,
}

#[derive(Clone)]
pub struct ServingService {
    pub store: AuthorityStore,
    pub objects: ObjectStore,
    pub configuration: nous_configuration_service::ConfigurationService,
    pub publisher: ServingPublisher,
    pub options: ServingOptions,
    pub embedding: Option<Arc<dyn TextEmbeddingProvider>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectionStatus {
    pub generations: BTreeMap<String, ServingGenerationId>,
    pub rebuilt: Vec<String>,
    pub reopened: Vec<String>,
    pub degradation: Vec<Degradation>,
}

impl ServingService {
    pub fn new(
        store: AuthorityStore,
        objects: ObjectStore,
        options: ServingOptions,
        embedding: Option<Arc<dyn TextEmbeddingProvider>>,
        configuration: nous_configuration_service::ConfigurationService,
    ) -> Result<Self> {
        std::fs::create_dir_all(&options.root).map_err(artifacts::io)?;
        Ok(Self {
            store,
            objects,
            configuration,
            options,
            embedding,
            publisher: ServingPublisher::default(),
        })
    }
}
