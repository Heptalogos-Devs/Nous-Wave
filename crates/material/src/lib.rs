//! Material and evidence semantic owner plus its persistence/service operations.

mod domain;
pub use domain::*;

pub mod service;
pub use service::*;

pub const MAX_UPLOAD_BYTES: nous_configuration::ConfigKey<u64> =
    nous_configuration::ConfigKey::new("object_store.max_upload_bytes");
pub const ABSOLUTE_UPLOAD_BYTES: u64 = 9_007_199_254_740_991;
pub fn register_configuration(
    registry: &mut nous_configuration::ConfigRegistryBuilder,
) -> nous_core::Result<()> {
    use nous_configuration::*;
    registry.register(
        MAX_UPLOAD_BYTES,
        "material",
        "Artifact upload byte budget.",
        8 * 1024 * 1024 * 1024,
        ConfigExposure::Advanced,
        ConfigScopePolicy::SystemOnly,
        ConfigApplyMode::RestartProcess,
        ConfigSemanticEffect::Operational,
        |value| {
            if *value > 0 && *value <= ABSOLUTE_UPLOAD_BYTES {
                Ok(())
            } else {
                Err(nous_core::Error::Invalid(
                    "upload budget exceeds bounds".into(),
                ))
            }
        },
    )?;
    registry.describe(MAX_UPLOAD_BYTES.path(), |d| {
        d.unit = Some("bytes".into());
        d.json_schema["minimum"] = serde_json::json!(1);
        d.json_schema["maximum"] = serde_json::json!(ABSOLUTE_UPLOAD_BYTES);
    })
}
