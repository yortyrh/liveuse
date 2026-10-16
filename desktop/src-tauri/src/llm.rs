//! Provider and model choices from Rig's registry and catalog.
//! The API key lives in the app config on this machine.

use rig::DynModel;
use rig::catalog::{Catalog, ModelSpec, ReasoningSupport};
use rig::completion::{Effort, Reasoning};
use rig::operation::Completion;
use rig::providers::registry::{ProviderConfig, ProviderId, ProviderRef};
use rig::rig_reqwest::ReqwestClient;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderChoice {
    pub id: String,
    pub label: String,
    pub base_url: String,
    pub needs_key: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelChoice {
    pub id: String,
    pub name: String,
    pub image: bool,
    pub tools: bool,
    /// Empty when the model does not reason or the catalog does not know it.
    pub reasoning: Vec<ReasoningChoice>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningChoice {
    /// What `parse_reasoning` reads: empty for the model's default, `off`,
    /// an effort word, or `budget:<tokens>`.
    pub value: String,
    pub label: String,
}

const BUDGETS: [u32; 4] = [2048, 8192, 16384, 32768];

pub fn parse_reasoning(value: &str) -> Result<Option<Reasoning>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value == "off" {
        return Ok(Some(Reasoning::Off));
    }
    if let Some(tokens) = value.strip_prefix("budget:") {
        let tokens = tokens
            .parse::<u32>()
            .map_err(|_| format!("Unknown reasoning budget `{value}`."))?;
        return Ok(Some(Reasoning::Budget { tokens }));
    }
    let effort: Effort = serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|_| format!("Unknown reasoning level `{value}`."))?;
    Ok(Some(Reasoning::Effort(effort)))
}

fn reasoning_choices(spec: &ModelSpec) -> Vec<ReasoningChoice> {
    let support: &ReasoningSupport = &spec.reasoning;
    if !support.supported {
        return Vec::new();
    }
    let default_label = match support.default {
        Some(effort) => format!("Default ({})", effort_label(effort).to_lowercase()),
        None => "Default".to_string(),
    };
    let mut rows = vec![choice("", default_label)];
    if support.can_disable {
        rows.push(choice("off", "Off".into()));
    }
    let mut levels = support.levels.clone();
    // OpenRouter turns an effort level into a thinking budget for upstreams
    // that only take a budget or an on/off switch.
    if levels.is_empty() && support.budget.is_none() && spec.provider.vendor() == "openrouter" {
        levels = vec![Effort::Low, Effort::Medium, Effort::High];
    }
    for effort in levels {
        rows.push(choice(effort.as_str(), effort_label(effort).into()));
    }
    if support.levels.is_empty() {
        if let Some(range) = &support.budget {
            for tokens in BUDGETS
                .into_iter()
                .filter(|tokens| range.contains(tokens) && tokens < range.end())
            {
                rows.push(choice(
                    &format!("budget:{tokens}"),
                    format!("{}K token budget", tokens / 1024),
                ));
            }
        }
    }
    if rows.len() < 2 { Vec::new() } else { rows }
}

fn choice(value: &str, label: String) -> ReasoningChoice {
    ReasoningChoice {
        value: value.to_string(),
        label,
    }
}

fn effort_label(effort: Effort) -> &'static str {
    match effort {
        Effort::Minimal => "Minimal",
        Effort::Low => "Low",
        Effort::Medium => "Medium",
        Effort::High => "High",
        Effort::XHigh => "Extra high",
        Effort::Max => "Max",
        _ => effort.as_str(),
    }
}

pub fn canonical_provider(provider: &str) -> Result<String, String> {
    let id = ProviderId::resolve(provider.trim()).map_err(|err| err.to_string())?;
    Ok(id.to_string())
}

pub fn needs_key(provider: &str) -> bool {
    ProviderId::resolve(provider.trim())
        .map(|id| id.requires_credential())
        .unwrap_or(true)
}

pub fn providers() -> Vec<ProviderChoice> {
    let mut rows: Vec<ProviderChoice> = ProviderId::all()
        .filter_map(|id| {
            let config = id.config("")?;
            Some(ProviderChoice {
                id: id.to_string(),
                label: provider_label(id),
                base_url: config.base_url().trim_end_matches('/').to_string(),
                needs_key: id.requires_credential(),
            })
        })
        .collect();
    rows.sort_by(|left, right| left.label.cmp(&right.label));
    rows
}

pub fn catalog_models(provider: &str) -> Result<Vec<ModelChoice>, String> {
    let id = ProviderId::resolve(provider.trim()).map_err(|err| err.to_string())?;
    let vendor = id.vendor();
    let mut models: Vec<ModelChoice> = Catalog::builtin()
        .iter()
        .filter(|spec| spec.provider.vendor() == vendor && !spec.deprecated)
        .map(|spec| ModelChoice {
            id: spec.id.clone(),
            name: spec.display_name.clone(),
            image: spec.input.image,
            tools: spec.tools,
            reasoning: reasoning_choices(spec),
        })
        .collect();
    models.sort_by(|left, right| {
        model_rank(left)
            .cmp(&model_rank(right))
            .then(left.name.cmp(&right.name))
    });
    Ok(models)
}

pub async fn account_models(
    provider: &str,
    base_url: &str,
    key: &str,
) -> Result<Vec<ModelChoice>, String> {
    let id = ProviderId::resolve(provider.trim()).map_err(|err| err.to_string())?;
    let config = provider_config(provider, base_url, key)?;
    let list = match config {
        ProviderConfig::OpenAi(config) => config.client().list_models().await,
        ProviderConfig::Anthropic(config) => config.client().list_models().await,
        ProviderConfig::Gemini(config) => config.client().list_models().await,
    }
    .map_err(|err| err.to_string())?;
    let mut models: Vec<ModelChoice> = list
        .iter()
        .filter(|model| usable_account_model(&model.id, model.r#type.as_deref()))
        .map(|model| ModelChoice {
            id: model.id.clone(),
            name: model.display_name().to_string(),
            image: false,
            tools: true,
            reasoning: Catalog::builtin()
                .find(id, &model.id)
                .map(reasoning_choices)
                .unwrap_or_default(),
        })
        .collect();
    models.sort_by(|left, right| left.name.cmp(&right.name));
    models.truncate(400);
    Ok(models)
}

pub fn completion_model(
    provider: &str,
    model: &str,
    base_url: &str,
    key: &str,
) -> Result<DynModel<Completion>, String> {
    let model = model.trim();
    if model.is_empty() {
        return Err("Choose a model.".into());
    }
    let config = provider_config(provider, base_url, key)?;
    let reference = ProviderRef::configured(config, model).map_err(|err| err.to_string())?;
    Ok(reference.completion_model_with(key, ReqwestClient::default()))
}

fn provider_config(provider: &str, base_url: &str, key: &str) -> Result<ProviderConfig, String> {
    let id = ProviderId::resolve(provider.trim()).map_err(|err| err.to_string())?;
    if id.requires_credential() && key.trim().is_empty() {
        return Err("Set the API key for this provider.".into());
    }
    let config = id
        .config(key)
        .ok_or_else(|| "This provider cannot be configured.".to_string())?;
    let base = base_url.trim().trim_end_matches('/');
    if base.is_empty() {
        Ok(config)
    } else {
        Ok(config.with_base_url(base))
    }
}

fn provider_label(id: ProviderId) -> String {
    let name = match id.vendor() {
        "openai" => "OpenAI".to_string(),
        "anthropic" => "Anthropic".to_string(),
        "gcp.gemini" => "Gemini".to_string(),
        "groq" => "Groq".to_string(),
        "openrouter" => "OpenRouter".to_string(),
        "mistral" => "Mistral".to_string(),
        "deepseek" => "DeepSeek".to_string(),
        "ollama" => "Ollama".to_string(),
        "xai" => "xAI".to_string(),
        "together" => "Together".to_string(),
        "cohere" => "Cohere".to_string(),
        "perplexity" => "Perplexity".to_string(),
        "huggingface" => "Hugging Face".to_string(),
        "azure.openai" => "Azure OpenAI".to_string(),
        "moonshot" => "Moonshot".to_string(),
        "minimax" => "MiniMax".to_string(),
        "zai" => "Z.ai".to_string(),
        "xiaomimimo" => "Xiaomi MiMo".to_string(),
        "llamacpp" => "llama.cpp".to_string(),
        "copilot" => "GitHub Copilot".to_string(),
        "chatgpt" => "ChatGPT".to_string(),
        "hyperbolic" => "Hyperbolic".to_string(),
        "venice" => "Venice".to_string(),
        "doubleword" => "Doubleword".to_string(),
        "mira" => "Mira".to_string(),
        other => title_vendor(other),
    };
    let formats = ProviderId::vendor_selections(id.vendor()).count();
    match (formats > 1, id.format()) {
        (true, Some(format)) => format!("{name} ({})", format.as_str()),
        _ => name,
    }
}

fn title_vendor(vendor: &str) -> String {
    vendor
        .split(['.', '-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn model_rank(model: &ModelChoice) -> u8 {
    match (model.image, model.tools) {
        (true, true) => 0,
        (_, true) => 1,
        (true, false) => 2,
        (false, false) => 3,
    }
}

fn usable_account_model(id: &str, kind: Option<&str>) -> bool {
    let id = id.to_ascii_lowercase();
    let skipped = [
        "embed",
        "whisper",
        "tts",
        "dall-e",
        "moderation",
        "transcri",
        "realtime",
    ];
    if skipped.iter().any(|part| id.contains(part)) {
        return false;
    }
    if let Some(kind) = kind {
        let kind = kind.to_ascii_lowercase();
        if kind.contains("embed") || kind.contains("image") || kind.contains("audio") {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_catalog_lists_gpt_4o_mini_as_vision() {
        let models = catalog_models("openai").unwrap();
        let mini = models
            .iter()
            .find(|model| model.id == "gpt-4o-mini")
            .unwrap();
        assert!(mini.image);
        assert!(mini.tools);
        assert!(
            models
                .windows(2)
                .all(|pair| model_rank(&pair[0]) <= model_rank(&pair[1]))
        );
    }

    fn reasoning_values(provider: &str, model: &str) -> Vec<String> {
        catalog_models(provider)
            .unwrap()
            .into_iter()
            .find(|row| row.id == model)
            .unwrap()
            .reasoning
            .into_iter()
            .map(|row| row.value)
            .collect()
    }

    #[test]
    fn openrouter_sonnet_reasoning_choices() {
        assert_eq!(
            reasoning_values("openrouter", "anthropic/claude-sonnet-4.5"),
            ["", "off", "low", "medium", "high"]
        );
        assert_eq!(
            reasoning_values("openrouter", "anthropic/claude-sonnet-4.6"),
            ["", "off", "low", "medium", "high", "max"]
        );
    }

    #[test]
    fn anthropic_budget_model_offers_budgets() {
        let values = reasoning_values("anthropic", "claude-haiku-4-5-20251001");
        assert_eq!(&values[..2], ["", "off"]);
        assert!(values[2..].iter().all(|value| value.starts_with("budget:")));
        assert!(!values[2..].is_empty());
    }

    #[test]
    fn every_reasoning_choice_parses() {
        for model in catalog_models("openrouter").unwrap() {
            for row in model.reasoning {
                parse_reasoning(&row.value).unwrap();
            }
        }
        assert_eq!(parse_reasoning("").unwrap(), None);
        assert_eq!(parse_reasoning("off").unwrap(), Some(Reasoning::Off));
        assert_eq!(
            parse_reasoning("budget:8192").unwrap(),
            Some(Reasoning::Budget { tokens: 8192 })
        );
        assert!(parse_reasoning("loud").is_err());
    }

    #[test]
    fn providers_include_openai() {
        let providers = providers();
        assert!(providers.iter().any(|provider| provider.label == "OpenAI"));
        assert_eq!(canonical_provider("openai").unwrap(), "openai/openai");
    }
}
