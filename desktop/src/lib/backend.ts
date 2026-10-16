import { invoke } from "@tauri-apps/api/core";

const DEV_LLM_ORIGIN = "http://127.0.0.1:1429";

function inDesktopApp() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function devJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${DEV_LLM_ORIGIN}${path}`, init);
  const text = await response.text();
  const body = text ? (JSON.parse(text) as { error?: string }) : null;
  if (!response.ok) {
    throw new Error(body?.error || `Request failed (${response.status})`);
  }
  return body as T;
}

export type DeviceStatus = {
  connected: boolean;
  path?: string | null;
  uptime_ms?: number | null;
  firmware?: string | null;
  error?: string | null;
  bootloader?: boolean;
  foreign?: boolean;
};

export type FirmwareOffer = {
  state: string;
  current?: string | null;
  available?: string | null;
  notes?: string | null;
  message?: string | null;
};

export type FlashProgress = {
  phase: string;
  percent: number;
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
  reasoning: string;
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
  reasoning: ReasoningChoice[];
};

export type ReasoningChoice = {
  value: string;
  label: string;
};

export type Target =
  | { kind: "local" }
  | { kind: "remote"; url: string; token: string };

export function deviceStatus() {
  return invoke<DeviceStatus>("device_status");
}

export function firmwareOffer(force = false) {
  return invoke<FirmwareOffer>("firmware_offer", { force });
}

export type FlashReport = {
  outcome: string;
  message?: string | null;
  firstSetup?: boolean;
};

export function flashFirmware(confirmed = false) {
  return invoke<FlashReport>("flash_firmware", { confirmed });
}

export function shareInfo() {
  return invoke<ShareInfo>("share_info");
}

export function regenerateToken() {
  return invoke<ShareInfo>("regenerate_token");
}

export function llmSettings() {
  if (!inDesktopApp()) {
    return devJson<LlmSettings>("/llm/settings");
  }
  return invoke<LlmSettings>("llm_settings");
}

export function llmProviders() {
  if (!inDesktopApp()) {
    return devJson<LlmProvider[]>("/llm/providers");
  }
  return invoke<LlmProvider[]>("llm_providers");
}

export function catalogModels(provider: string) {
  if (!inDesktopApp()) {
    return devJson<LlmModel[]>(`/llm/catalog?provider=${encodeURIComponent(provider)}`);
  }
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

export function saveLlm(
  provider: string,
  baseUrl: string,
  model: string,
  apiKey: string,
  reasoning: string,
) {
  const key = apiKey.trim() ? apiKey : null;
  if (!inDesktopApp()) {
    return devJson<void>("/llm/settings", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ provider, baseUrl, model, apiKey: key, reasoning }),
    });
  }
  return invoke<void>("save_llm", {
    provider,
    baseUrl,
    model,
    apiKey: key,
    reasoning,
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
