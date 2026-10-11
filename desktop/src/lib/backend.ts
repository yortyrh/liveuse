import { invoke } from "@tauri-apps/api/core";

export type DeviceStatus = {
  connected: boolean;
  path?: string | null;
  uptime_ms?: number | null;
  firmware?: string | null;
  error?: string | null;
};

export type ShareInfo = {
  token: string;
  port: number;
  urls: string[];
  listening: boolean;
  serverError?: string | null;
};

export type LlmSettings = {
  provider: string;
  baseUrl: string;
  model: string;
  hasApiKey: boolean;
  needsKey: boolean;
};

export type LlmProvider = {
  id: string;
  label: string;
  baseUrl: string;
  needsKey: boolean;
};

export type LlmModel = {
  id: string;
  name: string;
  image: boolean;
  tools: boolean;
};

export type Target =
  | { kind: "local" }
  | { kind: "remote"; url: string; token: string };

export function deviceStatus() {
  return invoke<DeviceStatus>("device_status");
}

export function shareInfo() {
  return invoke<ShareInfo>("share_info");
}

export function regenerateToken() {
  return invoke<ShareInfo>("regenerate_token");
}

export function llmSettings() {
  return invoke<LlmSettings>("llm_settings");
}

export function llmProviders() {
  return invoke<LlmProvider[]>("llm_providers");
}

export function catalogModels(provider: string) {
  return invoke<LlmModel[]>("llm_catalog", { provider });
}

export function listLlmModels(provider: string, baseUrl: string, apiKey: string) {
  return invoke<LlmModel[]>("list_llm_models", {
    provider,
    baseUrl,
    apiKey: apiKey.trim() ? apiKey : null,
  });
}

export type ScrollDirection = {
  inverted: boolean | null;
};

export function scrollDirection() {
  return invoke<ScrollDirection>("scroll_direction");
}

export function saveScrollDirection(inverted: boolean) {
  return invoke<void>("save_scroll_direction", { inverted });
}

export function scrollProbe() {
  return invoke<void>("scroll_probe");
}

export function scrollProbeNudge() {
  return invoke<void>("scroll_probe_nudge");
}

export function saveLlm(provider: string, baseUrl: string, model: string, apiKey: string) {
  return invoke<void>("save_llm", {
    provider,
    baseUrl,
    model,
    apiKey: apiKey.trim() ? apiKey : null,
  });
}

export function perform(target: Target, action: string, args: Record<string, unknown> = {}) {
  return invoke<Record<string, unknown>>("perform", { target, action, args });
}

export function requestAbort() {
  return invoke<void>("request_abort");
}

export function clearAbort() {
  return invoke<void>("clear_abort");
}

export function reveal() {
  return invoke<void>("reveal");
}
