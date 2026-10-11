import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { ScrollDirection } from "@/components/scroll-direction";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { AgentChat } from "@/components/agent-chat";
import { ModelPicker } from "@/components/model-picker";
import { CaptureView } from "@/components/capture-view";
import { runAgent, type TranscriptLine } from "@/lib/agent";
import { STOP_SHORTCUT } from "@/lib/shortcut";
import {
  deviceStatus,
  llmProviders,
  llmSettings,
  previewFrame,
  regenerateToken,
  requestAbort,
  saveLlm,
  shareInfo,
  type DeviceStatus,
  type LlmProvider,
  type LlmSettings,
  type ShareInfo,
  type Target,
} from "@/lib/backend";

export function MainWindow() {
  const [device, setDevice] = useState<DeviceStatus>({ connected: false });
  const [share, setShare] = useState<ShareInfo | null>(null);
  const [urlIndex, setUrlIndex] = useState(0);
  const [llm, setLlm] = useState<LlmSettings>({
    provider: "openai/openai",
    baseUrl: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    hasApiKey: false,
    needsKey: true,
  });
  const [providers, setProviders] = useState<LlmProvider[]>([]);
  const [apiKey, setApiKey] = useState("");
  const [mode, setMode] = useState<"local" | "remote">("local");
  const [remoteUrl, setRemoteUrl] = useState("");
  const [remoteToken, setRemoteToken] = useState("");
  const [frame, setFrame] = useState<string | null>(null);
  const [lines, setLines] = useState<TranscriptLine[]>([]);
  const [draft, setDraft] = useState("");
  const [running, setRunning] = useState(false);
  const cancelled = useMemo(() => ({ current: false }), []);

  useEffect(() => {
    const refresh = () => {
      void deviceStatus().then(setDevice).catch(() => undefined);
      void shareInfo().then(setShare).catch(() => undefined);
    };
    refresh();
    void llmSettings().then(setLlm).catch(() => undefined);
    void llmProviders().then(setProviders).catch(() => undefined);
    const timer = window.setInterval(refresh, 2000);
    const unlisten = listen<DeviceStatus>("device-status", (event) => {
      setDevice(event.payload);
    });
    const abortListen = listen("agent-abort", () => {
      cancelled.current = true;
    });
    return () => {
      window.clearInterval(timer);
      void unlisten.then((stop) => stop());
      void abortListen.then((stop) => stop());
    };
  }, []);

  const target: Target = useMemo(() => {
    if (mode === "remote") {
      return { kind: "remote", url: remoteUrl.trim(), token: remoteToken.trim() };
    }
    return { kind: "local" };
  }, [mode, remoteUrl, remoteToken]);

  useEffect(() => {
    if (mode !== "remote" || !remoteUrl.trim() || !remoteToken.trim()) {
      setFrame(null);
      return;
    }
    let alive = true;
    const tick = () => {
      void previewFrame(target)
        .then((jpeg) => {
          if (alive) {
            setFrame(`data:image/jpeg;base64,${jpeg}`);
          }
        })
        .catch(() => {
          if (alive) {
            setFrame(null);
          }
        });
    };
    tick();
    const timer = window.setInterval(tick, 1000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [mode, remoteUrl, remoteToken, target]);

  const selectedUrl = share?.urls[urlIndex] ?? share?.urls[0] ?? "";
  const mcpConfig = selectedUrl
    ? JSON.stringify(
        {
          mcpServers: {
            liveuse: {
              url: `${selectedUrl}/mcp`,
              headers: { Authorization: `Bearer ${share?.token ?? ""}` },
            },
          },
        },
        null,
        2,
      )
    : "";

  async function copy(value: string, label: string) {
    await navigator.clipboard.writeText(value);
    toast.success(`${label} copied`);
  }

  async function send() {
    const prompt = draft.trim();
    if (!prompt || running) {
      return;
    }
    if (mode === "remote" && (!remoteUrl.trim() || !remoteToken.trim())) {
      toast.error("The remote machine needs a URL and a token");
      return;
    }
    if (llm.needsKey && !llm.hasApiKey) {
      toast.error("Save the API key on this Mac");
      return;
    }
    setDraft("");
    setRunning(true);
    cancelled.current = false;
    try {
      await runAgent({
        target,
        history: lines,
        prompt,
        cancelled,
        onUpdate: setLines,
      });
    } catch (error) {
      const text = error instanceof Error ? error.message : String(error);
      toast.error(text);
      setLines((current) => [
        ...current,
        { id: crypto.randomUUID(), role: "assistant", text },
      ]);
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="mx-auto flex h-screen max-w-5xl flex-col gap-4 p-4">
      <header className="flex items-center justify-between gap-3">
        <div>
          <h1 className="text-lg font-medium">LiveUse</h1>
          <p className="text-sm text-muted-foreground">
            The local service starts only while the XIAO is connected. Closing the window leaves it in the menu bar.
          </p>
        </div>
        <Badge variant={device.connected ? "default" : "secondary"}>
          {device.connected ? "XIAO connected" : "No device"}
        </Badge>
      </header>
      <Tabs defaultValue="device" className="min-h-0 flex-1">
        <TabsList>
          <TabsTrigger value="device">Device</TabsTrigger>
          <TabsTrigger value="share">Share</TabsTrigger>
          <TabsTrigger value="control">Control</TabsTrigger>
        </TabsList>
        <TabsContent value="device" className="grid min-h-0 content-start gap-4 overflow-y-auto">
          <Card>
            <CardHeader>
              <CardTitle>XIAO</CardTitle>
              <CardDescription>Mouse and keyboard over the USB cable, commands over the serial port.</CardDescription>
            </CardHeader>
            <CardContent className="grid gap-2 text-sm">
              <StatusRow label="Status" value={device.connected ? "Connected" : "Disconnected"} />
              <StatusRow label="Port" value={device.path || "—"} />
              <StatusRow label="Firmware" value={device.firmware || "—"} />
              <StatusRow
                label="Uptime"
                value={device.uptime_ms != null ? `${Math.round(device.uptime_ms / 1000)} s` : "—"}
              />
              <StatusRow label="Service" value={share?.listening ? `listening on port ${share.port}` : "stopped"} />
              {device.error ? <p className="text-destructive">{device.error}</p> : null}
              {share?.serverError ? <p className="text-destructive">{share.serverError}</p> : null}
            </CardContent>
          </Card>
          <ScrollDirection connected={device.connected} />
        </TabsContent>
        <TabsContent value="share" className="min-h-0">
          <Card>
            <CardHeader>
              <CardTitle>Local network</CardTitle>
              <CardDescription>
                These credentials are read here, on the physical machine. Anyone with the token on the LAN can move the mouse and the keyboard.
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4">
              <div className="grid gap-2">
                <span className="text-sm font-medium">API URL</span>
                {share?.urls.length ? (
                  share.urls.map((url, index) => (
                    <div key={url} className="flex gap-2">
                      <Button
                        variant={index === urlIndex ? "default" : "outline"}
                        onClick={() => setUrlIndex(index)}
                      >
                        {url}
                      </Button>
                      <Button variant="outline" onClick={() => void copy(url, "URL")}>
                        Copy
                      </Button>
                    </div>
                  ))
                ) : (
                  <p className="text-sm text-muted-foreground">No local-network IP yet.</p>
                )}
              </div>
              <Separator />
              <div className="grid gap-2">
                <span className="text-sm font-medium">Token</span>
                <div className="flex gap-2">
                  <Input readOnly value={share?.token ?? ""} />
                  <Button variant="outline" onClick={() => share && void copy(share.token, "Token")}>
                    Copy
                  </Button>
                  <Button
                    variant="secondary"
                    onClick={() =>
                      void regenerateToken()
                        .then((next) => {
                          setShare(next);
                          toast.success("New token. Previous clients no longer work.");
                        })
                        .catch((error: unknown) => toast.error(String(error)))
                    }
                  >
                    Regenerate
                  </Button>
                </div>
              </div>
              <div className="grid gap-2">
                <span className="text-sm font-medium">MCP configuration</span>
                <Textarea readOnly value={mcpConfig} className="min-h-40 font-mono text-xs" />
                <Button variant="outline" disabled={!mcpConfig} onClick={() => void copy(mcpConfig, "MCP")}>
                  Copy configuration
                </Button>
              </div>
            </CardContent>
          </Card>
        </TabsContent>
        <TabsContent value="control" className="flex min-h-0 flex-1 flex-col gap-3">
          <div className="flex flex-wrap items-center gap-2">
            <Button variant={mode === "local" ? "default" : "outline"} onClick={() => setMode("local")}>
              This machine
            </Button>
            <Button variant={mode === "remote" ? "default" : "outline"} onClick={() => setMode("remote")}>
              Remote
            </Button>
            <span className="text-xs text-muted-foreground">{llm.model}</span>
          </div>
          <ModelPicker
            saved={llm}
            providers={providers}
            apiKey={apiKey}
            onApiKey={setApiKey}
            onSave={(draft) =>
              void saveLlm(draft.provider, draft.baseUrl, draft.model, draft.apiKey)
                .then(async () => {
                  setApiKey("");
                  setLlm(await llmSettings());
                  toast.success("Model saved");
                })
                .catch((error: unknown) => toast.error(String(error)))
            }
          />
          {mode === "remote" ? (
            <div className="grid gap-2 md:grid-cols-[1fr_1fr]">
              <Input
                aria-label="Remote URL"
                placeholder="http://192.168.1.20:8765"
                value={remoteUrl}
                onChange={(event) => setRemoteUrl(event.target.value)}
              />
              <Input
                aria-label="Remote token"
                placeholder="token"
                value={remoteToken}
                onChange={(event) => setRemoteToken(event.target.value)}
              />
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">
              While this machine is being controlled, the card stays at the bottom left. {STOP_SHORTCUT} stops and opens the app again.
            </p>
          )}
          <div className={mode === "remote" ? "grid min-h-0 flex-1 gap-3 md:grid-cols-[0.9fr_1.1fr]" : "flex min-h-0 flex-1 flex-col"}>
            {mode === "remote" ? (
              <div className="flex min-h-64 items-center justify-center overflow-hidden rounded-xl bg-muted p-2 ring-1 ring-foreground/10">
                {frame ? (
                  <CaptureView src={frame} alt="Latest capture from the remote machine" />
                ) : (
                  <p className="px-4 text-sm text-muted-foreground">No capture yet. Check the URL and the token.</p>
                )}
              </div>
            ) : null}
            <AgentChat
              lines={lines}
              draft={draft}
              running={running}
              onDraft={setDraft}
              onSend={() => void send()}
              onStop={() => void requestAbort()}
            />
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}

function StatusRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="text-muted-foreground">{label}</span>
      <span className="truncate text-right">{value}</span>
    </div>
  );
}
