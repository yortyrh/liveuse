import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { CaptureView } from "@/components/capture-view";
import { MarkdownMessage } from "@/components/markdown-message";
import type { TranscriptLine } from "@/lib/agent";
import { STOP_SHORTCUT } from "@/lib/shortcut";

const EXAMPLES = ["What is on the screen?", "Open Safari and search for the weather"];

export function AgentChat({
  lines,
  draft,
  running,
  onDraft,
  onSend,
  onStop,
  onClear,
}: {
  lines: TranscriptLine[];
  draft: string;
  running: boolean;
  onDraft: (value: string) => void;
  onSend: () => void;
  onStop: () => void;
  onClear: () => void;
}) {
  const scroller = useRef<HTMLDivElement>(null);
  const field = useRef<HTMLTextAreaElement>(null);
  const stick = useRef(true);
  const streaming = lines.some((line) => line.pending);
  const announcement = announcementFor(lines);
  const now = useNow(streaming);

  useEffect(() => {
    const el = scroller.current;
    if (el && stick.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [lines, running]);

  return (
    <div className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-xl border border-border bg-card">
      <div className="flex items-center justify-between gap-3 border-b border-border px-4 py-2">
        <h2 className="text-sm font-medium">Chat</h2>
        <div className="flex items-center gap-3">
          <p className="sr-only" role="status">
            {announcement}
          </p>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={lines.length === 0 && !draft.trim() && !running}
            onClick={onClear}
          >
            Clear
          </Button>
        </div>
      </div>
      <div
        ref={scroller}
        className="min-h-0 flex-1 overflow-y-auto px-4 py-4"
        onScroll={(event) => {
          const el = event.currentTarget;
          stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 48;
        }}
      >
        {lines.length === 0 ? (
          <div className="grid justify-items-start gap-3 py-8">
            <div>
              <p className="text-sm font-medium">Nothing sent yet</p>
              <p className="mt-1 max-w-md text-sm text-muted-foreground">
                The agent looks at the screen and uses the mouse and keyboard.
              </p>
            </div>
            <div className="flex flex-wrap gap-2">
              {EXAMPLES.map((example) => (
                <Button
                  key={example}
                  type="button"
                  variant="outline"
                  size="sm"
                  disabled={running}
                  onClick={() => {
                    onDraft(example);
                    field.current?.focus();
                  }}
                >
                  {example}
                </Button>
              ))}
            </div>
          </div>
        ) : (
          <div className="grid gap-4">
            {lines.map((line) => (
              <Message key={line.id} line={line} now={now} />
            ))}
            {running && !streaming ? (
              <p className="text-xs text-muted-foreground">Using the computer…</p>
            ) : null}
          </div>
        )}
      </div>
      <form
        className="grid gap-2 border-t border-border p-3"
        onSubmit={(event) => {
          event.preventDefault();
          onSend();
        }}
      >
        <div className="flex items-center justify-between gap-3">
          <label htmlFor="agent-message" className="text-xs font-medium">
            Message
          </label>
          <p className="text-xs text-muted-foreground">
            {running ? `${STOP_SHORTCUT} also stops` : "Enter sends. Shift+Enter adds a line."}
          </p>
        </div>
        <div className="flex items-end gap-2">
          <Textarea
            ref={field}
            id="agent-message"
            value={draft}
            placeholder="For example: open Safari and search for the weather"
            className="max-h-32 min-h-10 flex-1"
            disabled={running}
            onChange={(event) => onDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey) {
                event.preventDefault();
                onSend();
              }
            }}
          />
          {running ? (
            <Button type="button" variant="destructive" onClick={onStop}>
              Stop
            </Button>
          ) : (
            <Button type="submit" disabled={!draft.trim()}>
              Send
            </Button>
          )}
        </div>
      </form>
    </div>
  );
}

function Message({ line, now }: { line: TranscriptLine; now: number }) {
  const elapsed = elapsedMs(line, now);
  if (line.role === "tool" || line.role === "reasoning") {
    return (
      <div className="grid gap-2 border-l-2 border-border pl-3">
        <p className="text-xs break-words text-muted-foreground">
          {line.text}
          <Duration ms={elapsed} />
        </p>
        {line.image ? <CaptureView src={line.image} alt={line.text || "Screen capture"} /> : null}
      </div>
    );
  }

  if (line.role === "user") {
    return (
      <article className="ml-auto max-w-[min(32rem,85%)]" aria-label="You">
        <p className="mb-1 text-right text-xs text-muted-foreground">You</p>
        <div className="rounded-lg bg-primary px-3 py-2 text-sm whitespace-pre-wrap text-primary-foreground">
          {line.text}
        </div>
      </article>
    );
  }

  return (
    <article className="max-w-[42rem]" aria-label="Agent" aria-busy={line.pending || undefined}>
      <p className="mb-1 text-xs text-muted-foreground">
        Agent
        <Duration ms={elapsed} />
      </p>
      {line.text ? (
        <MarkdownMessage text={line.text} />
      ) : line.pending ? (
        <p className="text-sm text-muted-foreground">
          Writing
          <span className="ml-1 inline-block h-4 w-px animate-pulse bg-foreground align-middle" aria-hidden />
        </p>
      ) : null}
      {line.image ? (
        <div className="mt-2">
          <CaptureView src={line.image} alt={line.text || "Screen capture"} />
        </div>
      ) : null}
    </article>
  );
}

function announcementFor(lines: TranscriptLine[]) {
  const last = [...lines].reverse().find((line) => !line.pending && line.text.trim());
  if (!last) {
    return "";
  }
  const brief = briefText(last.text);
  const time = last.ms != null ? `, ${formatDuration(last.ms)}` : "";
  if (last.role === "tool" || last.role === "reasoning") {
    return `${brief}${time}`;
  }
  if (last.role === "assistant") {
    return `Agent: ${brief}${time}`;
  }
  return "";
}

function Duration({ ms }: { ms: number | null }) {
  if (ms == null) {
    return null;
  }
  return (
    <span className="tabular-nums">
      <span aria-hidden="true"> · </span>
      {formatDuration(ms)}
    </span>
  );
}

function elapsedMs(line: TranscriptLine, now: number) {
  if (!line.pending && line.ms != null) {
    return line.ms;
  }
  if (line.started != null) {
    return Math.max(0, now - line.started);
  }
  return line.ms ?? null;
}

function formatDuration(ms: number) {
  const clamped = Math.max(0, Math.round(ms));
  if (clamped < 10_000) {
    return `${(clamped / 1000).toFixed(1)}s`;
  }
  if (clamped < 60_000) {
    return `${Math.round(clamped / 1000)}s`;
  }
  const total = Math.round(clamped / 1000);
  const minutes = Math.floor(total / 60);
  const seconds = total % 60;
  return `${minutes}m ${seconds.toString().padStart(2, "0")}s`;
}

function useNow(ticking: boolean) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!ticking) {
      return;
    }
    const update = () => setNow(Date.now());
    update();
    const id = window.setInterval(update, 100);
    return () => window.clearInterval(id);
  }, [ticking]);
  return now;
}

function briefText(text: string) {
  const flat = text.replace(/\s+/g, " ").trim();
  return flat.length > 180 ? `${flat.slice(0, 180)}…` : flat;
}
