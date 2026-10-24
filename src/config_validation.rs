//! Native catalog policy, separate from parsing and public document integrity.
use crate::config::Config;
use crate::config_lexical::{has_sensitive_marker, validate_id, validate_text};
use std::collections::BTreeSet;
const MAX_RESOURCES: usize = 64;
pub(crate) fn validate(config: &Config) -> Result<(), String> {
    if !matches!(config.schema_version, 1 | 2) {
        return Err("unsupported schema_version".into());
    }
    validate_id(config.server.id.as_str())?;
    validate_text(&config.server.title, 80)?;
    validate_text(&config.server.instructions, 500)?;
    if config.schema_version == 2
        && (has_sensitive_marker(config.server.id.as_str())
            || has_sensitive_marker(&config.server.title)
            || has_sensitive_marker(&config.server.instructions))
    {
        return Err("metadata contains a prohibited marker".into());
    }
    let ids = config
        .resources
        .iter()
        .map(|resource| resource.id.as_str())
        .collect::<BTreeSet<_>>();
    if config.resources.is_empty()
        || config.resources.len() > MAX_RESOURCES
        || ids.len() != config.resources.len()
    {
        return Err("config must define 1 to 64 unique public resources".into());
    }
    let mut document_bytes = 0usize;
    for resource in &config.resources {
        validate_id(resource.id.as_str())?;
        validate_text(&resource.title, 80)?;
        validate_text(&resource.summary, 500)?;
        if resource.capabilities.is_empty() || resource.capabilities.len() > 16 {
            return Err("capabilities must contain 1 to 16 entries".into());
        }
        for capability in &resource.capabilities {
            validate_text(capability, 120)?;
        }
        if config.schema_version == 2 {
            let metadata =
                serde_json::to_string(resource).map_err(|_| "invalid public metadata")?;
            if has_sensitive_marker(&metadata) {
                return Err("metadata contains a prohibited marker".into());
            }
        }
        if let Some(document) = &resource.document {
            if config.schema_version != 2 {
                return Err("documents require schema_version 2".into());
            }
            document.validate()?;
            document_bytes = document_bytes
                .checked_add(document.text.len())
                .ok_or("document byte overflow")?;
            if document_bytes > 32768 {
                return Err("aggregate documents exceed 32 KiB".into());
            }
        }
    }
    Ok(())
}
