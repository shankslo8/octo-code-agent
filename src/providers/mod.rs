mod openai;

pub use openai::OpenAiProvider;

use crate::core::config::AppConfig;
use crate::core::error::ProviderError;
use crate::core::model::{self, ModelId};
use crate::core::provider::Provider;
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
    // Try to find model in registry; for custom models, create a generic fallback
    let mut model = model::get_model(model_id).unwrap_or_else(|| model::Model {
        id: model_id.clone(),
        vendor: model::ModelVendor::OpenAI, // generic
        display_name: model_id.0.clone(),
        context_window: 128_000,
        max_output_tokens: 32_768,
        capabilities: model::ModelCapabilities {
            supports_tool_use: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_images: false,
        },
        pricing: model::ModelPricing {
            cost_per_1m_input: 0.0,
            cost_per_1m_output: 0.0,
            cost_per_1m_input_cached: None,
        },
    });

    // OpenRouter uses different model IDs than Atlas Cloud
    if config.provider_type == crate::core::config::ProviderType::OpenRouter {
        if let Some(or_id) = model::atlas_to_openrouter_id(&model.id.0) {
            model.id = ModelId(or_id.to_string());
        }
    }

    let api_keys = config.get_active_api_keys();
    if api_keys.is_empty() {
        let hint = match config.provider_type {
            crate::core::config::ProviderType::OpenRouter => {
                "OPENROUTER_API_KEY not set. Set via env var or config file."
            }
            crate::core::config::ProviderType::AtlasCloud => {
                "ATLAS_API_KEY not set. Set via env var or config file."
            }
        };
        return Err(ProviderError::MissingApiKey(hint.into()));
    }

    let base_url = config.get_active_base_url();

    Ok(Arc::new(OpenAiProvider::new(
        api_keys,
        model,
        base_url,
        config.agent.max_tokens,
    )))
}
