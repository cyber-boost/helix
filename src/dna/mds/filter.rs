use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    pub enabled: bool,
    pub rules: Vec<FilterRule>,
    pub cache_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    pub pattern: String,
    pub action: FilterAction,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterAction {
    Allow,
    Deny,
    Log,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: Vec::new(),
            cache_enabled: false,
        }
    }
}

pub fn create_default_filter() -> FilterConfig {
    FilterConfig::default()
}

pub fn validate_filter_config(config: &FilterConfig) -> Result<(), String> {
    for rule in &config.rules {
        if rule.pattern.is_empty() {
            return Err("Filter rule pattern cannot be empty".to_string());
        }
    }
    Ok(())
}
