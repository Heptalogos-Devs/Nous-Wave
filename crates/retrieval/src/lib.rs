//! Immutable on-disk serving generations shared by runtime and cognitive contributors.
mod mechanisms;
pub use mechanisms::*;
mod artifacts;
mod build;
mod lifecycle;
mod material;
mod provider;
mod query;
pub use material::*;

use nous_core::*;
use nous_object_store::ObjectStore;
use nous_persistence::{AuthorityStore, ServingRecord};
pub use provider::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::Arc,
};

pub const LEXICAL_ENABLED_KEY: nous_configuration::ConfigKey<bool> =
    nous_configuration::ConfigKey::new("serving.lexical.enabled");
pub const DENSE_ENABLED_KEY: nous_configuration::ConfigKey<bool> =
    nous_configuration::ConfigKey::new("serving.dense.enabled");
pub const TOPOLOGY_ENABLED_KEY: nous_configuration::ConfigKey<bool> =
    nous_configuration::ConfigKey::new("serving.topology.enabled");

pub fn register_configuration(
    registry: &mut nous_configuration::ConfigRegistryBuilder,
) -> Result<()> {
    for (key, description, default) in [
        (
            LEXICAL_ENABLED_KEY,
            "Whether lexical Serving is enabled.",
            true,
        ),
        (DENSE_ENABLED_KEY, "Whether dense Serving is enabled.", true),
        (
            TOPOLOGY_ENABLED_KEY,
            "Whether experimental topology Serving is enabled.",
            false,
        ),
    ] {
        registry.register(
            key,
            "serving",
            description,
            default,
            nous_configuration::ConfigExposure::Developer,
            nous_configuration::ConfigScopePolicy::SystemOnly,
            nous_configuration::ConfigApplyMode::RestartProcess,
            nous_configuration::ConfigSemanticEffect::Operational,
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
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProjectionCapabilities {
    pub memory: bool,
}

#[derive(Clone)]
pub struct ServingService {
    pub store: AuthorityStore,
    pub objects: ObjectStore,
    pub configuration: nous_configuration::ConfigurationService,
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
        configuration: nous_configuration::ConfigurationService,
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
