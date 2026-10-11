//! Provider and model choices from Rig's registry and catalog.
//! The API key lives in the app config on this machine.

use rig::DynModel;
use rig::catalog::Catalog;
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

    #[test]
    fn providers_include_openai() {
        let providers = providers();
        assert!(providers.iter().any(|provider| provider.label == "OpenAI"));
        assert_eq!(canonical_provider("openai").unwrap(), "openai/openai");
    }
}
