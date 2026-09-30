pub mod engine;
pub mod mapping;
pub mod pointer;
pub mod schema;

use schema::ProviderConfig;

/// Loads and validates one provider config file. `schemaVersion` must
/// be 1 (the only version this project understands, per the schema
/// doc).
pub fn load_config(json: &str) -> Result<ProviderConfig, String> {
    let config: ProviderConfig =
        serde_json::from_str(json).map_err(|e| format!("invalid provider config: {e}"))?;
    if config.schema_version != 1 {
        return Err(format!(
            "unsupported schemaVersion {} (only 1 is understood)",
            config.schema_version
        ));
    }
    Ok(config)
}

/// Runs one provider config end-to-end: executes its `source`, then
/// applies its `map`. This is the whole "provider is data" pipeline.
pub fn run_provider(config: &ProviderConfig) -> Result<mapping::Report, String> {
    let captured = engine::run_source(&config.source)?;
    Ok(mapping::apply_map(&config.map, &captured))
}
