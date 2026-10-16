import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { Target } from "@/lib/backend";

export type TranscriptLine = {
  id: string;
  role: "user" | "assistant" | "tool" | "reasoning";
  text: string;
  image?: string | null;
  pending?: boolean;
  started?: number;
  ms?: number;
  reset?: boolean;
};

type AgentDelta = {
  id: string;
  text: string;
  done: boolean;
  reset: boolean;
  started?: number | null;
  ms?: number | null;
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
    applyLine(lines, line);
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
        started: millis(delta.started),
        ms: millis(delta.ms),
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

function applyLine(lines: TranscriptLine[], line: TranscriptLine) {
  const index = lines.findIndex((item) => item.id === line.id);
  if (line.reset) {
    if (index >= 0) {
      lines.splice(index, 1);
    }
    return;
  }
  const next: TranscriptLine = {
    id: line.id,
    role: line.role,
    text: line.text,
    image: line.image || undefined,
    pending: line.pending || undefined,
    started: millis(line.started),
    ms: millis(line.ms),
  };
  if (index < 0) {
    lines.push(next);
  } else {
    lines[index] = next;
  }
}

function millis(value: number | null | undefined) {
  return typeof value === "number" ? value : undefined;
}
