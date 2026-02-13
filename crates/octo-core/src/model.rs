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
}

impl fmt::Display for ModelVendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zhipu => write!(f, "Zhipu AI"),
            Self::Moonshot => write!(f, "Moonshot AI"),
            Self::Alibaba => write!(f, "Alibaba"),
            Self::MiniMax => write!(f, "MiniMax"),
            Self::DeepSeek => write!(f, "DeepSeek"),
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

/// All models routed through Atlas Cloud unified API
pub fn builtin_models() -> HashMap<ModelId, Model> {
    let mut m = HashMap::new();

    // GLM-4.7 (Zhipu AI) — 358B MoE, agent-optimized, 128K output
    m.insert(
        ModelId("zai-org/glm-4.7".into()),
        Model {
            id: ModelId("zai-org/glm-4.7".into()),
            vendor: ModelVendor::Zhipu,
            display_name: "GLM-4.7".into(),
            context_window: 200_000,
            max_output_tokens: 128_000,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: true,
                supports_images: true,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 0.52,
                cost_per_1m_output: 2.56,
                cost_per_1m_input_cached: None,
            },
        },
    );

    // GLM-5 (Zhipu AI) — 745B MoE (44B active), MIT license, frontier agentic
    m.insert(
        ModelId("zai-org/glm-5".into()),
        Model {
            id: ModelId("zai-org/glm-5".into()),
            vendor: ModelVendor::Zhipu,
            display_name: "GLM-5".into(),
            context_window: 200_000,
            max_output_tokens: 128_000,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: true,
                supports_images: true,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 0.80,
                cost_per_1m_output: 2.56,
                cost_per_1m_input_cached: None,
            },
        },
    );

    // Kimi K2.5 (Moonshot AI) — ultra-long context, native multimodality
    m.insert(
        ModelId("moonshotai/kimi-k2.5".into()),
        Model {
            id: ModelId("moonshotai/kimi-k2.5".into()),
            vendor: ModelVendor::Moonshot,
            display_name: "Kimi K2.5".into(),
            context_window: 256_000,
            max_output_tokens: 32_768,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: true,
                supports_images: true,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 0.50,
                cost_per_1m_output: 2.50,
                cost_per_1m_input_cached: None,
            },
        },
    );

    // Qwen3 Max (Alibaba) — flagship, ultra-long context, code gen
    m.insert(
        ModelId("qwen/qwen3-max-2026-01-23".into()),
        Model {
            id: ModelId("qwen/qwen3-max-2026-01-23".into()),
            vendor: ModelVendor::Alibaba,
            display_name: "Qwen3 Max".into(),
            context_window: 131_072,
            max_output_tokens: 32_768,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: true,
                supports_images: false,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 1.20,
                cost_per_1m_output: 6.00,
                cost_per_1m_input_cached: None,
            },
        },
    );

    // MiniMax M2.1 — 230B MoE, SWE-bench 74%, MIT license
    m.insert(
        ModelId("minimaxai/minimax-m2.1".into()),
        Model {
            id: ModelId("minimaxai/minimax-m2.1".into()),
            vendor: ModelVendor::MiniMax,
            display_name: "MiniMax M2.1".into(),
            context_window: 128_000,
            max_output_tokens: 65_536,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: false,
                supports_images: false,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 0.30,
                cost_per_1m_output: 0.30,
                cost_per_1m_input_cached: None,
            },
        },
    );

    // DeepSeek V3.2 — 685B MoE, cheapest, tool_use supported
    m.insert(
        ModelId("deepseek-ai/deepseek-v3.2".into()),
        Model {
            id: ModelId("deepseek-ai/deepseek-v3.2".into()),
            vendor: ModelVendor::DeepSeek,
            display_name: "DeepSeek V3.2".into(),
            context_window: 128_000,
            max_output_tokens: 32_768,
            capabilities: ModelCapabilities {
                supports_tool_use: true,
                supports_streaming: true,
                supports_thinking: true,
                supports_images: false,
            },
            pricing: ModelPricing {
                cost_per_1m_input: 0.26,
                cost_per_1m_output: 0.88,
                cost_per_1m_input_cached: None,
            },
        },
    );

    m
}

pub fn get_model(id: &ModelId) -> Option<Model> {
    builtin_models().remove(id)
}

pub fn get_default_model() -> Model {
    get_model(&ModelId("zai-org/glm-4.7".into()))
        .expect("Default model must exist")
}
