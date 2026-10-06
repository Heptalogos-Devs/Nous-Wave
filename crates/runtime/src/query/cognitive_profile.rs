use nous_configuration::*;
use nous_core::Result;
use serde::{Deserialize, Serialize};

/// The fixed, code-owned cognitive retrieval registry. No dynamic loader is
/// selected by configuration. Kernel implementation belongs to Retrieval.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, Default,
)]
pub enum CognitiveProfile {
    #[serde(rename = "baseline-rrf")]
    BaselineRrf,
    #[default]
    #[serde(rename = "nous-node-potential-v1")]
    NousNodePotential,
    #[serde(rename = "vcp-dtsc-v9.2.1-adapter-v1")]
    VcpDtsc,
    #[serde(rename = "vcp-rivermemo-v3.1-adapter-v1")]
    VcpRiverMemo,
}

#[derive(Debug, Clone, Copy)]
pub struct CognitiveProfileRequirements {
    pub topology: bool,
    pub query_embedding: bool,
    pub base_candidates: bool,
}

impl CognitiveProfile {
    pub const ALL: [Self; 4] = [
        Self::BaselineRrf,
        Self::NousNodePotential,
        Self::VcpDtsc,
        Self::VcpRiverMemo,
    ];
    pub const fn id(self) -> &'static str {
        match self {
            Self::BaselineRrf => "baseline-rrf",
            Self::NousNodePotential => "nous-node-potential-v1",
            Self::VcpDtsc => "vcp-dtsc-v9.2.1-adapter-v1",
            Self::VcpRiverMemo => "vcp-rivermemo-v3.1-adapter-v1",
        }
    }
    pub const fn requirements(self) -> CognitiveProfileRequirements {
        let reference = matches!(self, Self::VcpDtsc | Self::VcpRiverMemo);
        CognitiveProfileRequirements {
            topology: !matches!(self, Self::BaselineRrf),
            query_embedding: reference,
            base_candidates: reference,
        }
    }
    pub fn digest(self) -> String {
        blake3::hash(self.id().as_bytes()).to_hex().to_string()
    }
}

pub const COGNITIVE_PROFILE: ConfigKey<CognitiveProfile> =
    ConfigKey::new("retrieval.cognitive.profile");

pub(super) fn register_configuration(registry: &mut ConfigRegistryBuilder) -> Result<()> {
    registry.register(
        COGNITIVE_PROFILE,
        "runtime",
        "Fixed cognitive lane profile selected before candidate generation.",
        CognitiveProfile::default(),
        ConfigExposure::Developer,
        ConfigScopePolicy::SubjectOverrideAllowed,
        ConfigApplyMode::Live,
        ConfigSemanticEffect::QueryPolicy,
        |_: &CognitiveProfile| Ok(()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_registry_rejects_dynamic_names_and_describes_requirements() {
        for profile in CognitiveProfile::ALL {
            let encoded = serde_json::to_value(profile).expect("profile value");
            assert_eq!(encoded.as_str(), Some(profile.id()));
            assert_eq!(
                serde_json::from_value::<CognitiveProfile>(encoded).expect("profile"),
                profile
            );
        }
        assert!(
            serde_json::from_value::<CognitiveProfile>(serde_json::json!("arbitrary-plugin"))
                .is_err()
        );
        assert!(!CognitiveProfile::BaselineRrf.requirements().topology);
        assert!(
            !CognitiveProfile::NousNodePotential
                .requirements()
                .query_embedding
        );
        assert!(
            CognitiveProfile::VcpRiverMemo
                .requirements()
                .query_embedding
        );
    }
}
