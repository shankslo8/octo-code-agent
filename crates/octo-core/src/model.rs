use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelId(pub String);

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ModelId {
    fn from(s: &str) -> Self {
        ModelId(s.to_string())
    }
}

/// Model vendor (informational, not used for routing)
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelVendor {
    Zhipu,
    Moonshot,
    Alibaba,
    MiniMax,
    DeepSeek,
    Anthropic,
    OpenAI,
    Google,
    Meta,
    Mistral,
}

impl fmt::Display for ModelVendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zhipu => write!(f, "Zhipu AI"),
            Self::Moonshot => write!(f, "Moonshot AI"),
            Self::Alibaba => write!(f, "Alibaba"),
            Self::MiniMax => write!(f, "MiniMax"),
            Self::DeepSeek => write!(f, "DeepSeek"),
            Self::Anthropic => write!(f, "Anthropic"),
            Self::OpenAI => write!(f, "OpenAI"),
            Self::Google => write!(f, "Google"),
            Self::Meta => write!(f, "Meta"),
            Self::Mistral => write!(f, "Mistral"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub supports_tool_use: bool,
    pub supports_streaming: bool,
    pub supports_thinking: bool,
    pub supports_images: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub cost_per_1m_input: f64,
    pub cost_per_1m_output: f64,
    pub cost_per_1m_input_cached: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: ModelId,
    pub vendor: ModelVendor,
    pub display_name: String,
    pub context_window: u64,
    pub max_output_tokens: u64,
    pub capabilities: ModelCapabilities,
    pub pricing: ModelPricing,
}

impl Model {
    pub fn calculate_cost(&self, input_tokens: u64, output_tokens: u64) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.pricing.cost_per_1m_input;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.pricing.cost_per_1m_output;
        input_cost + output_cost
    }
}

/// All models (Atlas Cloud is the source of truth)
pub fn builtin_models() -> HashMap<ModelId, Model> {
    atlas_cloud_models()
}

/// Models available on Atlas Cloud (api.atlascloud.ai)
/// Only these models are also registered for OpenRouter.
pub fn atlas_cloud_models() -> HashMap<ModelId, Model> {
    let mut m = HashMap::new();

    // GLM-5 (Zhipu AI) — 745B MoE, strong coding & reasoning
    m.insert(
        ModelId("z-ai/glm-5".into()),
        Model {
            id: ModelId("z-ai/glm-5".into()),
            vendor: ModelVendor::Zhipu,
            display_name: "GLM-5".into(),
            context_window: 202_752,
            max_output_tokens: 131_072,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: true,
                supports_images: true,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 0.80,
                cost_per_1m_output: 2.56,
                cost_per_1m_input_cached: Some(0.16),
            },
        },
    );

    // GLM-4.7 (Zhipu AI) — efficient MoE, good coding
    m.insert(
        ModelId("z-ai/glm-4.7".into()),
        Model {
            id: ModelId("z-ai/glm-4.7".into()),
            vendor: ModelVendor::Zhipu,
            display_name: "GLM-4.7".into(),
            context_window: 202_752,
            max_output_tokens: 131_072,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: true,
                supports_images: true,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 0.52,
                cost_per_1m_output: 1.75,
                cost_per_1m_input_cached: None,
            },
        },
    );

    // DeepSeek V3.2 (DeepSeek) — very cheap, strong coding
    m.insert(
        ModelId("deepseek/deepseek-v3.2".into()),
        Model {
            id: ModelId("deepseek/deepseek-v3.2".into()),
            vendor: ModelVendor::DeepSeek,
            display_name: "DeepSeek V3.2".into(),
            context_window: 163_840,
            max_output_tokens: 65_536,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: true,
                supports_images: false,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 0.26,
                cost_per_1m_output: 0.38,
                cost_per_1m_input_cached: None,
            },
        },
    );

    // Qwen3 Max (Alibaba) — flagship, ultra-long context, strong reasoning
    m.insert(
        ModelId("qwen/qwen3-max-2026-01-23".into()),
        Model {
            id: ModelId("qwen/qwen3-max-2026-01-23".into()),
            vendor: ModelVendor::Alibaba,
            display_name: "Qwen3 Max".into(),
            context_window: 252_000,
            max_output_tokens: 32_000,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: true,
                supports_images: true,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 1.20,
                cost_per_1m_output: 6.00,
                cost_per_1m_input_cached: None,
            },
        },
    );

    // Qwen3 Coder (Alibaba) — 480B MoE, code-optimized, 262K context
    m.insert(
        ModelId("Qwen/Qwen3-Coder".into()),
        Model {
            id: ModelId("Qwen/Qwen3-Coder".into()),
            vendor: ModelVendor::Alibaba,
            display_name: "Qwen3 Coder".into(),
            context_window: 262_144,
            max_output_tokens: 65_536,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: false,
                supports_images: false,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 0.78,
                cost_per_1m_output: 3.80,
                cost_per_1m_input_cached: None,
            },
        },
    );

    // Kimi K2 Thinking (Moonshot) — deep reasoning, long context
    m.insert(
        ModelId("moonshotai/kimi-k2-thinking".into()),
        Model {
            id: ModelId("moonshotai/kimi-k2-thinking".into()),
            vendor: ModelVendor::Moonshot,
            display_name: "Kimi K2 Thinking".into(),
            context_window: 262_144,
            max_output_tokens: 65_536,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: true,
                supports_images: false,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 0.60,
                cost_per_1m_output: 2.50,
                cost_per_1m_input_cached: None,
            },
        },
    );

    // MiniMax M2.5 — lightweight, fast & cheap
    m.insert(
        ModelId("minimax/minimax-m2.5".into()),
        Model {
            id: ModelId("minimax/minimax-m2.5".into()),
            vendor: ModelVendor::MiniMax,
            display_name: "MiniMax M2.5".into(),
            context_window: 196_608,
            max_output_tokens: 65_536,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: false,
                supports_images: false,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 0.29,
                cost_per_1m_output: 0.95,
                cost_per_1m_input_cached: Some(0.03),
            },
        },
    );

    m
}

/// OpenRouter models — same as Atlas Cloud (only Atlas Cloud models are registered)
pub fn openrouter_models() -> HashMap<ModelId, Model> {
    atlas_cloud_models()
}

pub fn get_model(id: &ModelId) -> Option<Model> {
    builtin_models().remove(id)
}

pub fn get_default_model() -> Model {
    get_model(&ModelId("z-ai/glm-5".into()))
        .expect("Default model must exist")
}
