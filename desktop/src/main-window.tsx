import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button, buttonVariants } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import { ScrollDirection } from "@/components/scroll-direction";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";
import { AgentChat } from "@/components/agent-chat";
import { ModelPicker } from "@/components/model-picker";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { runAgent, type TranscriptLine } from "@/lib/agent";
import {
  deviceStatus,
  firmwareOffer,
  flashFirmware,
  llmProviders,
  llmSettings,
  regenerateToken,
  requestAbort,
  saveLlm,
  shareInfo,
  type DeviceStatus,
  type FirmwareOffer,
  type FlashProgress,
  type LlmProvider,
  type LlmSettings,
  type ShareInfo,
  type Target,
} from "@/lib/backend";

export function MainWindow() {
  const [device, setDevice] = useState<DeviceStatus>({ connected: false });
  const [offer, setOffer] = useState<FirmwareOffer>({ state: "none" });
  const [flashing, setFlashing] = useState(false);
  const [progress, setProgress] = useState<FlashProgress | null>(null);
  const [confirmForeign, setConfirmForeign] = useState(false);
  const [firstSetup, setFirstSetup] = useState(false);
  const [flashError, setFlashError] = useState<string | null>(null);
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
  const [providersNote, setProvidersNote] = useState("Loading providers…");
  const [apiKey, setApiKey] = useState("");
  const [modelOpen, setModelOpen] = useState(false);
  const [canSaveModel, setCanSaveModel] = useState(false);
  const [mode, setMode] = useState<"local" | "remote">("local");
  const [remoteUrl, setRemoteUrl] = useState("");
  const [remoteToken, setRemoteToken] = useState("");
  const [remoteSession, setRemoteSession] = useState<{ url: string; token: string } | null>(null);
  const [lines, setLines] = useState<TranscriptLine[]>([]);
  const [draft, setDraft] = useState("");
  const [running, setRunning] = useState(false);
  const cancelled = useMemo(() => ({ current: false }), []);
  const runId = useRef(0);

  useEffect(() => {
    const refresh = () => {
      void deviceStatus().then(setDevice).catch(() => undefined);
      void shareInfo().then(setShare).catch(() => undefined);
    };
    refresh();
    void llmSettings().then(setLlm).catch(() => undefined);
    void llmProviders()
      .then((items) => {
        setProviders(items);
        setProvidersNote(items.length === 0 ? "No providers." : "");
      })
      .catch(() => setProvidersNote("Couldn't load providers."));
    const timer = window.setInterval(refresh, 2000);
    const unlisten = listen<DeviceStatus>("device-status", (event) => {
      setDevice(event.payload);
    });
    const abortListen = listen("agent-abort", () => {
      cancelled.current = true;
    });
    const progressListen = listen<FlashProgress>("flash-progress", (event) => {
      if (event.payload.phase === "first-setup") {
        setFirstSetup(true);
      }
      setProgress(event.payload);
    });
    return () => {
      window.clearInterval(timer);
      void unlisten.then((stop) => stop());
      void abortListen.then((stop) => stop());
      void progressListen.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    if (flashing) {
      return;
    }
    void firmwareOffer()
      .then((next) => {
        setOffer(next);
      })
      .catch((error: unknown) => {
        const text = error instanceof Error ? error.message : String(error);
        setOffer({ state: "checkFailed", message: text });
      });
  }, [device.connected, device.firmware, device.bootloader, device.foreign, device.path, flashing]);

  const target: Target = useMemo(() => {
    if (mode === "remote" && remoteSession) {
      return { kind: "remote", url: remoteSession.url, token: remoteSession.token };
    }
    return { kind: "local" };
  }, [mode, remoteSession]);

  const companyLabel = providers.find((item) => item.id === llm.provider)?.label ?? llm.provider;
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

  async function writeFirmware(confirmed: boolean) {
    if (flashing) {
      return;
    }
    setConfirmForeign(false);
    setFirstSetup(false);
    setFlashing(true);
    setFlashError(null);
    setProgress({ phase: "prepare", percent: 0 });
    try {
      const report = await flashFirmware(confirmed);
      if (report.outcome === "confirm") {
        setConfirmForeign(true);
        return;
      }
      if (report.outcome === "written") {
        toast.success(report.firstSetup ? "First setup: firmware written" : "Firmware written");
      } else if (report.message) {
        toast.message(report.message);
      }
      setFlashError(null);
    } catch (error) {
      const text = error instanceof Error ? error.message : String(error);
      if (text === "payment_required") {
        setFlashError("Installing the first firmware will require a purchase.");
      } else {
        toast.error(text);
      }
    } finally {
      setFlashing(false);
      setProgress(null);
      void firmwareOffer(true)
        .then(setOffer)
        .catch(() => undefined);
    }
  }

  async function copy(value: string, label: string) {
    await navigator.clipboard.writeText(value);
    toast.success(`${label} copied`);
  }

  function connectRemote() {
    const url = remoteUrl.trim();
    const token = remoteToken.trim();
    if (!url || !token) {
      return;
    }
    setRemoteSession({ url, token });
  }

  function clearChat() {
    runId.current += 1;
    cancelled.current = true;
    if (running) {
      void requestAbort();
    }
    setRunning(false);
    setLines([]);
    setDraft("");
  }

  function disconnectRemote() {
    runId.current += 1;
    if (running) {
      cancelled.current = true;
      void requestAbort();
    }
    setRunning(false);
    setRemoteSession(null);
    setLines([]);
    setDraft("");
  }

  async function send() {
    const prompt = draft.trim();
    if (!prompt || running) {
      return;
    }
    if (mode === "remote" && !remoteSession) {
      return;
    }
    if (llm.needsKey && !llm.hasApiKey) {
      toast.error("Save the API key on this Mac");
      return;
    }
    const id = ++runId.current;
    setDraft("");
    setRunning(true);
    cancelled.current = false;
    try {
      await runAgent({
        target,
        history: lines,
        prompt,
        cancelled,
        onUpdate: (next) => {
          if (runId.current === id) {
            setLines(next);
          }
        },
      });
    } catch (error) {
      if (runId.current !== id) {
        return;
      }
      const text = error instanceof Error ? error.message : String(error);
      toast.error(text);
      setLines((current) => [
        ...current,
        { id: crypto.randomUUID(), role: "assistant", text },
      ]);
    } finally {
      if (runId.current === id) {
        setRunning(false);
      }
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
          {flashing
            ? "Updating"
            : device.connected
              ? "XIAO connected"
              : device.bootloader
                ? "Bootloader"
                : device.foreign
                  ? "Other firmware"
                  : "No device"}
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
              <StatusRow
                label="Status"
                value={
                  device.connected
                    ? "Connected"
                    : device.bootloader
                      ? "Bootloader"
                      : device.foreign
                        ? "Other firmware"
                        : "Disconnected"
                }
              />
              <StatusRow label="Port" value={device.path || "—"} />
              <StatusRow label="Firmware" value={device.firmware || "—"} />
              <StatusRow
                label="Uptime"
                value={device.uptime_ms != null ? `${Math.round(device.uptime_ms / 1000)} s` : "—"}
              />
              <StatusRow label="Service" value={share?.listening ? `listening on port ${share.port}` : "stopped"} />
              {device.error ? <p className="text-destructive">{device.error}</p> : null}
              {share?.serverError ? <p className="text-destructive">{share.serverError}</p> : null}
              <FirmwareOfferBlock
                offer={offer}
                flashing={flashing}
                progress={progress}
                firstSetup={firstSetup}
                flashError={flashError}
                onCheck={() =>
                  void firmwareOffer(true)
                    .then((next) => {
                      setFlashError(null);
                      setOffer(next);
                    })
                    .catch((error: unknown) => {
                      const text = error instanceof Error ? error.message : String(error);
                      setOffer({ state: "checkFailed", message: text });
                    })
                }
                onWrite={() => void writeFirmware(false)}
                onReplace={() => setConfirmForeign(true)}
              />
            </CardContent>
          </Card>
          <Dialog open={confirmForeign} onOpenChange={(open) => !open && setConfirmForeign(false)}>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Replace the program on this board?</DialogTitle>
                <DialogDescription>
                  This board is not running LiveUse. Writing replaces whatever is on it. Continue only if this is the XIAO you mean to overwrite. The pointer and keyboard stop until the board answers again.
                </DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <DialogClose className={buttonVariants({ variant: "outline" })}>Cancel</DialogClose>
                <Button onClick={() => void writeFirmware(true)} disabled={flashing}>
                  Replace with LiveUse
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
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
            <p className="min-w-0 text-sm">
              <span className="text-muted-foreground">{companyLabel}</span>
              <span className="px-1.5 text-muted-foreground" aria-hidden>
                ·
              </span>
              <span className="font-medium">{llm.model}</span>
            </p>
            <Button variant="outline" onClick={() => setModelOpen(true)}>
              Change
            </Button>
          </div>
          <Dialog open={modelOpen} onOpenChange={setModelOpen}>
            <DialogContent initialFocus={false}>
              <DialogHeader>
                <DialogTitle>Change model</DialogTitle>
                <DialogDescription>The company and model used for this chat.</DialogDescription>
              </DialogHeader>
              <ModelPicker
                saved={llm}
                providers={providers}
                providersNote={providersNote}
                apiKey={apiKey}
                onApiKey={setApiKey}
                onCanSave={setCanSaveModel}
                onSave={(next) =>
                  void saveLlm(next.provider, next.baseUrl, next.model, next.apiKey)
                    .then(async () => {
                      setApiKey("");
                      setLlm(await llmSettings());
                      setModelOpen(false);
                      toast.success("Model saved");
                    })
                    .catch((error: unknown) => toast.error(String(error)))
                }
              />
              <DialogFooter className="border-t border-border pt-4">
                <DialogClose className={buttonVariants({ variant: "outline" })}>Cancel</DialogClose>
                <Button type="submit" form="llm-model-form" disabled={!canSaveModel}>
                  Save
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
          {mode === "remote" && remoteSession ? (
            <div className="flex items-center justify-between gap-3">
              <div className="flex min-w-0 items-center gap-2">
                <span className="truncate text-sm font-medium">{remoteSession.url}</span>
                <Badge>Connected</Badge>
              </div>
              <Button variant="outline" onClick={disconnectRemote}>
                Disconnect
              </Button>
            </div>
          ) : null}
          {mode === "remote" && !remoteSession ? (
            <div className="grid gap-2 md:grid-cols-[1fr_1fr_auto]">
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
              <Button
                disabled={!remoteUrl.trim() || !remoteToken.trim()}
                onClick={connectRemote}
              >
                Connect
              </Button>
            </div>
          ) : (
            <div className="flex min-h-0 flex-1 flex-col">
              <AgentChat
                lines={lines}
                draft={draft}
                running={running}
                onDraft={setDraft}
                onSend={() => void send()}
                onStop={() => void requestAbort()}
                onClear={clearChat}
              />
            </div>
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}

function FirmwareOfferBlock({
  offer,
  flashing,
  progress,
  firstSetup,
  flashError,
  onCheck,
  onWrite,
  onReplace,
}: {
  offer: FirmwareOffer;
  flashing: boolean;
  progress: FlashProgress | null;
  firstSetup: boolean;
  flashError: string | null;
  onCheck: () => void;
  onWrite: () => void;
  onReplace: () => void;
}) {
  if (offer.state === "none" && !flashing && !flashError) {
    return null;
  }
  return (
    <div className="grid gap-2 border-t border-border pt-3">
      {flashing ? (
        <p>
          {firstSetup ? "First setup. " : ""}
          {phaseLabel(progress?.phase)} {progress?.percent ?? 0}%
        </p>
      ) : null}
      {offer.message ? <p>{offer.message}</p> : null}
      {offer.state === "update" && offer.available ? (
        <p>Published firmware {offer.available} is available.</p>
      ) : null}
      {flashError ? <p className="text-destructive">{flashError}</p> : null}
      <div className="flex flex-wrap gap-2">
        {offer.state === "update" ? (
          <Button disabled={flashing} onClick={onWrite}>
            Update to {offer.available}
          </Button>
        ) : null}
        {offer.state === "bootloader" ? (
          <Button disabled={flashing} onClick={onWrite}>
            Write firmware
          </Button>
        ) : null}
        {offer.state === "foreign" ? (
          <Button variant="secondary" disabled={flashing} onClick={onReplace}>
            Replace with LiveUse
          </Button>
        ) : null}
        {offer.state === "checkFailed" ? (
          <Button variant="outline" disabled={flashing} onClick={onCheck}>
            Check again
          </Button>
        ) : null}
      </div>
    </div>
  );
}

function phaseLabel(phase: string | undefined) {
  switch (phase) {
    case "first-setup":
      return "First setup";
    case "bootloader":
      return "Starting the bootloader";
    case "erase":
      return "Erasing";
    case "write":
      return "Writing";
    case "verify":
      return "Verifying";
    case "reset":
      return "Restarting";
    default:
      return "Preparing";
  }
}

function StatusRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="text-muted-foreground">{label}</span>
      <span className="truncate text-right">{value}</span>
    </div>
  );
}
