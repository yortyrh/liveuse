import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { Target } from "@/lib/backend";

export type TranscriptLine = {
  id: string;
  role: "user" | "assistant" | "tool";
  text: string;
  image?: string | null;
  pending?: boolean;
};

type AgentDelta = {
  id: string;
  text: string;
  done: boolean;
  reset: boolean;
};

export async function runAgent(options: {
  target: Target;
  history: TranscriptLine[];
  prompt: string;
  onUpdate: (lines: TranscriptLine[]) => void;
  cancelled: { current: boolean };
}) {
  const lines = [
    ...options.history,
    { id: crypto.randomUUID(), role: "user" as const, text: options.prompt },
  ];
  options.onUpdate(lines);
  const stop = await listen<TranscriptLine>("agent-line", (event) => {
    const line = event.payload;
    if (!line?.id) {
      return;
    }
    lines.push({
      id: line.id,
      role: line.role,
      text: line.text,
      image: line.image || undefined,
    });
    options.onUpdate([...lines]);
  });
  const stopDelta = await listen<AgentDelta>("agent-delta", (event) => {
    const delta = event.payload;
    if (!delta?.id) {
      return;
    }
    if (delta.reset) {
      const index = lines.findIndex((line) => line.id === delta.id);
      if (index >= 0) {
        lines.splice(index, 1);
      }
    } else {
      const next: TranscriptLine = {
        id: delta.id,
        role: "assistant",
        text: delta.text,
        pending: !delta.done,
      };
      const index = lines.findIndex((line) => line.id === delta.id);
      if (index < 0) {
        lines.push(next);
      } else {
        lines[index] = next;
      }
    }
    options.onUpdate([...lines]);
  });
  const abort = await listen("agent-abort", () => {
    options.cancelled.current = true;
  });
  try {
    await invoke("run_agent", {
      target: options.target,
      history: options.history
        .filter((line) => line.role === "user" || line.role === "assistant")
        .map((line) => ({ role: line.role, text: line.text })),
      prompt: options.prompt,
    });
  } finally {
    stop();
    stopDelta();
    abort();
  }
}
