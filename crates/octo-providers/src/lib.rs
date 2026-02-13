mod openai;

pub use openai::OpenAiProvider;

use octo_core::config::AppConfig;
use octo_core::error::ProviderError;
use octo_core::model::{self, ModelId};
use octo_core::provider::Provider;
use std::sync::Arc;

/// Model role for orchestration
#[derive(Debug, Clone, Copy)]
pub enum ModelRole {
    /// Primary coding model (default)
    Coder,
    /// Fast/cheap model for simple tasks
    Fast,
    /// Strong reasoning model for complex tasks
    Reasoning,
    /// Long context model for large file analysis
    LongContext,
}

/// Create a provider routed through Atlas Cloud unified API.
/// All models use the OpenAI-compatible endpoint.
pub fn create_provider(
    config: &AppConfig,
    model_id: Option<&ModelId>,
) -> Result<Arc<dyn Provider>, ProviderError> {
    let model_id = model_id
        .cloned()
        .unwrap_or_else(|| config.agent.coder_model.clone());
    create_provider_for_model(config, &model_id)
}

/// Create a provider for a specific model role (orchestration)
pub fn create_provider_for_role(
    config: &AppConfig,
    role: ModelRole,
) -> Result<Arc<dyn Provider>, ProviderError> {
    let model_id = match role {
        ModelRole::Coder => &config.agent.coder_model,
        ModelRole::Fast => &config.agent.fast_model,
        ModelRole::Reasoning => &config.agent.reasoning_model,
        ModelRole::LongContext => &config.agent.long_context_model,
    };
    create_provider_for_model(config, model_id)
}

fn create_provider_for_model(
    config: &AppConfig,
    model_id: &ModelId,
) -> Result<Arc<dyn Provider>, ProviderError> {
    let model = model::get_model(model_id)
        .ok_or_else(|| ProviderError::UnsupportedModel(model_id.to_string()))?;

    let api_key = config.get_api_key().ok_or_else(|| {
        ProviderError::MissingApiKey("ATLAS_API_KEY not set. Set via env var or config file.".into())
    })?;

    Ok(Arc::new(OpenAiProvider::new(
        api_key.to_string(),
        model,
        config.base_url.clone(),
        config.agent.max_tokens,
    )))
}
