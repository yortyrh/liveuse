import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import caseImage from "@/assets/case/assembled.png";
import insideImage from "@/assets/case/board-in-base.png";
import appIcon from "../src-tauri/icons/icon.svg";
import { useEffect, useMemo, useRef, useState } from "react";
import { Tabs as TabsPrimitive } from "@base-ui/react/tabs";
import {
  Cable,
  Check,
  Copy,
  ExternalLink,
  MousePointer2,
  RefreshCw,
  Settings2,
  ShieldAlert,
  Share2,
} from "lucide-react";
import { toast } from "sonner";
import { Button, buttonVariants } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { Input } from "@/components/ui/input";
import { ScrollDirection } from "@/components/scroll-direction";
import { Tabs, TabsContent } from "@/components/ui/tabs";
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
  const [checkingFirmware, setCheckingFirmware] = useState(false);
  const [share, setShare] = useState<ShareInfo | null>(null);
  const [urlIndex, setUrlIndex] = useState(0);
  const [llm, setLlm] = useState<LlmSettings>({
    provider: "openai/openai",
    baseUrl: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    reasoning: "",
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
    if (!device.connected && !device.bootloader && !device.foreign) {
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

  async function checkFirmwareUpdates() {
    if (flashing || checkingFirmware) {
      return;
    }
    setCheckingFirmware(true);
    setFlashError(null);
    try {
      const next = await firmwareOffer(true);
      setOffer(next);
      if (next.state === "checkFailed") {
        toast.error(next.message ?? "Couldn't check for firmware updates.");
      }
    } catch (error: unknown) {
      const text = error instanceof Error ? error.message : String(error);
      setOffer({ state: "checkFailed", message: text });
      toast.error(text);
    } finally {
      setCheckingFirmware(false);
    }
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

  const state = deviceState(device, flashing);
  const headline = STATE_COPY[state];
  const idle = !device.connected && !device.bootloader && !device.foreign && !flashing;

  return (
    <Tabs orientation="vertical" defaultValue="device" className="h-dvh gap-0">
      <aside className="flex w-16 shrink-0 flex-col gap-6 border-r border-border bg-card/70 p-2.5 min-[900px]:w-60 min-[900px]:p-4">
        <div className="flex items-center gap-3 min-[900px]:px-1">
          <img src={appIcon} alt="" className="size-11 shrink-0" />
          <span className="hidden text-lg font-semibold tracking-tight min-[900px]:inline">LiveUse</span>
        </div>
        <TabsPrimitive.List aria-label="Sections" className="flex flex-col gap-1">
          <NavTab value="device" icon={<Cable />}>
            Device
          </NavTab>
          <NavTab value="share" icon={<Share2 />}>
            Share
          </NavTab>
          <NavTab value="control" icon={<MousePointer2 />}>
            Control
          </NavTab>
        </TabsPrimitive.List>
        <div className="mt-auto grid gap-3">
          <StatusPill tone={headerTone(device, flashing)} label={headline.pill} />
          <p className="hidden text-xs leading-relaxed text-muted-foreground min-[900px]:block">
            The local service runs while LuLu is connected. Closing the window leaves LiveUse in the menu bar.
          </p>
        </div>
      </aside>
      <main className="flex min-w-0 flex-1 flex-col">
        <TabsContent value="device" className="min-h-0 overflow-y-auto">
          <div className="mx-auto grid w-full max-w-[88rem] gap-5 p-5 min-[1100px]:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)] min-[1100px]:p-8">
            <section
              aria-labelledby="device-headline"
              className="relative flex min-h-[20rem] flex-col items-center justify-center gap-5 overflow-hidden rounded-2xl bg-stage px-6 py-10 text-center ring-1 ring-foreground/10 min-[1100px]:sticky min-[1100px]:top-8 min-[1100px]:h-[calc(100dvh-4rem)] min-[1100px]:self-start"
            >
              <img
                src={device.connected ? insideImage : caseImage}
                alt={device.connected ? "LuLu with the lid off, the XIAO SAMD21 inside" : "LuLu"}
                className={cn(
                  "h-auto w-[clamp(8rem,24vw,19rem)] drop-shadow-[0_18px_22px_rgba(8,20,48,0.32)] transition-[opacity,filter] duration-500",
                  !device.connected && "opacity-60 grayscale",
                )}
              />
              <div
                className="relative h-px w-[clamp(6rem,14vw,12rem)] overflow-hidden bg-foreground/15"
                aria-hidden
              >
                {device.connected ? (
                  <span className="cable-pulse absolute inset-y-0 left-0 w-1/4 bg-primary" />
                ) : null}
              </div>
              <div className="grid max-w-md gap-1.5">
                <h2 id="device-headline" className="text-[1.75rem] leading-tight font-semibold tracking-tight">
                  {headline.title}
                </h2>
                <p className="text-sm text-muted-foreground">{headline.body}</p>
              </div>
              {device.error ? <p className="max-w-md text-sm text-destructive">{device.error}</p> : null}
              <FirmwareOfferBlock
                offer={offer}
                flashing={flashing}
                progress={progress}
                firstSetup={firstSetup}
                flashError={flashError}
                onCheck={() => void checkFirmwareUpdates()}
                onWrite={() => void writeFirmware(false)}
                onReplace={() => setConfirmForeign(true)}
              />
            </section>
            <div className="grid content-start gap-5">
              <Card>
                <CardHeader>
                  <CardTitle>Details</CardTitle>
                </CardHeader>
                <CardContent className="grid gap-1 text-sm">
                  <StatusRow label="Port" value={device.path || "—"} />
                  <StatusRow label="Firmware" value={device.firmware || "—"} />
                  <StatusRow
                    label="Uptime"
                    value={device.uptime_ms != null ? `${Math.round(device.uptime_ms / 1000)} s` : "—"}
                  />
                  <StatusRow
                    label="Service"
                    value={share?.listening ? `listening on port ${share.port}` : "stopped"}
                  />
                  {share?.serverError ? <p className="pt-2 text-destructive">{share.serverError}</p> : null}
                  <div className="flex flex-wrap gap-2 pt-3">
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={flashing || checkingFirmware}
                      onClick={() => void checkFirmwareUpdates()}
                    >
                      <RefreshCw className={cn(checkingFirmware && "animate-spin")} />
                      {checkingFirmware ? "Checking…" : "Check for firmware updates"}
                    </Button>
                  </div>
                </CardContent>
              </Card>
              {idle ? <SetupHints /> : null}
              <ScrollDirection connected={device.connected} />
            </div>
          </div>
          <Dialog open={confirmForeign} onOpenChange={(open) => !open && setConfirmForeign(false)}>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Replace the program on this board?</DialogTitle>
                <DialogDescription>
                  This board is not running LiveUse. Writing replaces whatever is on it. Continue only if this is the board you mean to turn into LuLu. The pointer and keyboard stop until the board answers again.
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
        </TabsContent>

        <TabsContent value="share" className="min-h-0 overflow-y-auto">
          <div className="mx-auto grid w-full max-w-[88rem] gap-5 p-5 min-[1100px]:grid-cols-2 min-[1100px]:p-8">
            <Card>
              <CardHeader>
                <CardTitle className="text-lg">Connect from another computer</CardTitle>
                <CardDescription>
                  Enter the URL and the token in LiveUse on that computer, or in any agent that calls the API.
                </CardDescription>
              </CardHeader>
              <CardContent className="grid gap-5">
                <div className="flex items-start gap-2.5 rounded-lg bg-warning/10 p-3 text-sm text-foreground">
                  <ShieldAlert className="mt-0.5 size-4 shrink-0 text-warning" aria-hidden />
                  <p>Anyone on this network with the token can move the mouse and use the keyboard.</p>
                </div>
                <div className="grid gap-2">
                  <span className="text-sm font-medium" id="share-url-label">
                    API URL
                  </span>
                  {share?.urls.length ? (
                    <div role="radiogroup" aria-labelledby="share-url-label" className="grid gap-1.5">
                      {share.urls.map((url, index) => {
                        const selected = index === urlIndex;
                        return (
                          <div key={url} className="flex gap-1.5">
                            <button
                              type="button"
                              role="radio"
                              aria-checked={selected}
                              onClick={() => setUrlIndex(index)}
                              className={cn(
                                "flex h-9 min-w-0 flex-1 items-center justify-between gap-2 rounded-lg border px-3 font-mono text-xs outline-none transition-colors focus-visible:ring-3 focus-visible:ring-ring/50",
                                selected
                                  ? "border-primary bg-accent text-accent-foreground"
                                  : "border-border hover:bg-muted",
                              )}
                            >
                              <span className="truncate">{url}</span>
                              {selected ? <Check className="size-3.5 shrink-0" aria-hidden /> : null}
                            </button>
                            <Button
                              variant="outline"
                              size="icon"
                              className="h-9 w-9"
                              aria-label={`Copy ${url}`}
                              onClick={() => void copy(url, "URL")}
                            >
                              <Copy />
                            </Button>
                          </div>
                        );
                      })}
                    </div>
                  ) : (
                    <p className="text-sm text-muted-foreground">No local-network address yet. Connect this Mac to a network.</p>
                  )}
                </div>
                <div className="grid gap-2">
                  <label htmlFor="share-token" className="text-sm font-medium">
                    Token
                  </label>
                  <div className="flex gap-1.5">
                    <Input id="share-token" readOnly value={share?.token ?? ""} className="h-9 font-mono text-xs" />
                    <Button
                      variant="outline"
                      size="icon"
                      className="h-9 w-9"
                      aria-label="Copy token"
                      onClick={() => share && void copy(share.token, "Token")}
                    >
                      <Copy />
                    </Button>
                  </div>
                  <Button
                    variant="secondary"
                    className="w-fit"
                    onClick={() =>
                      void regenerateToken()
                        .then((next) => {
                          setShare(next);
                          toast.success("New token. Previous clients no longer work.");
                        })
                        .catch((error: unknown) => toast.error(String(error)))
                    }
                  >
                    <RefreshCw />
                    Generate a new token
                  </Button>
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle className="text-lg">MCP configuration</CardTitle>
                <CardDescription>Paste this into an agent that supports MCP over HTTP.</CardDescription>
              </CardHeader>
              <CardContent className="grid gap-3">
                <pre
                  tabIndex={0}
                  className="max-h-[min(24rem,45vh)] overflow-auto rounded-lg bg-muted p-3 font-mono text-xs leading-relaxed outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
                >
                  {mcpConfig || "Available when this Mac has a local-network address."}
                </pre>
                <Button
                  variant="outline"
                  className="w-fit"
                  disabled={!mcpConfig}
                  onClick={() => void copy(mcpConfig, "MCP configuration")}
                >
                  <Copy />
                  Copy configuration
                </Button>
              </CardContent>
            </Card>
          </div>
        </TabsContent>

        <TabsContent value="control" className="flex min-h-0 flex-col">
          <div className="mx-auto flex min-h-0 w-full max-w-[72rem] flex-1 flex-col gap-3 p-4 min-[1100px]:p-6">
            <div className="flex flex-wrap items-center gap-x-3 gap-y-2">
              <div role="group" aria-label="Where to control" className="inline-flex rounded-lg bg-muted p-0.5">
                {(
                  [
                    ["local", "This Mac"],
                    ["remote", "Remote"],
                  ] as const
                ).map(([value, label]) => (
                  <button
                    key={value}
                    type="button"
                    aria-pressed={mode === value}
                    onClick={() => setMode(value)}
                    className={cn(
                      "h-7 rounded-md px-3 text-sm font-medium outline-none transition-colors focus-visible:ring-3 focus-visible:ring-ring/50",
                      mode === value
                        ? "bg-background text-foreground shadow-sm"
                        : "text-muted-foreground hover:text-foreground",
                    )}
                  >
                    {label}
                  </button>
                ))}
              </div>
              <Button variant="outline" className="ml-auto min-w-0 max-w-full" onClick={() => setModelOpen(true)}>
                <Settings2 />
                <span className="truncate">
                  <span className="text-muted-foreground">{companyLabel}</span>
                  <span className="px-1.5 text-muted-foreground" aria-hidden>
                    ·
                  </span>
                  <span>{llm.model}</span>
                </span>
                <span className="sr-only">Change model</span>
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
                    void saveLlm(next.provider, next.baseUrl, next.model, next.apiKey, next.reasoning)
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
              <div className="flex items-center justify-between gap-3 rounded-lg border border-border bg-card px-3 py-2">
                <div className="flex min-w-0 items-center gap-2">
                  <span className="truncate font-mono text-xs">{remoteSession.url}</span>
                  <StatusPill tone="ok" label="Connected" />
                </div>
                <Button variant="outline" size="sm" onClick={disconnectRemote}>
                  Disconnect
                </Button>
              </div>
            ) : null}
            {mode === "remote" && !remoteSession ? (
              <form
                className="grid content-start gap-3 rounded-xl border border-border bg-card p-4 min-[900px]:max-w-xl"
                onSubmit={(event) => {
                  event.preventDefault();
                  connectRemote();
                }}
              >
                <div>
                  <h2 className="text-base font-semibold">Drive another computer</h2>
                  <p className="text-sm text-muted-foreground">
                    Copy the URL and the token from the Share tab of LiveUse on that computer.
                  </p>
                </div>
                <label className="grid gap-1.5 text-sm font-medium">
                  URL
                  <Input
                    placeholder="http://192.168.1.20:8765"
                    value={remoteUrl}
                    onChange={(event) => setRemoteUrl(event.target.value)}
                  />
                </label>
                <label className="grid gap-1.5 text-sm font-medium">
                  Token
                  <Input
                    value={remoteToken}
                    onChange={(event) => setRemoteToken(event.target.value)}
                  />
                </label>
                <Button type="submit" className="w-fit" disabled={!remoteUrl.trim() || !remoteToken.trim()}>
                  Connect
                </Button>
              </form>
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
          </div>
        </TabsContent>
      </main>
    </Tabs>
  );
}

function NavTab({ value, icon, children }: { value: string; icon: React.ReactNode; children: string }) {
  return (
    <TabsPrimitive.Tab
      value={value}
      title={children}
      aria-label={children}
      className="flex h-10 items-center justify-center gap-3 rounded-lg px-3 text-sm font-medium text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 data-active:bg-accent data-active:text-accent-foreground min-[900px]:justify-start [&_svg]:size-[1.15rem] [&_svg]:shrink-0"
    >
      {icon}
      <span className="hidden min-[900px]:inline">{children}</span>
    </TabsPrimitive.Tab>
  );
}

type DeviceState = "connected" | "updating" | "bootloader" | "foreign" | "none";

function deviceState(device: DeviceStatus, flashing: boolean): DeviceState {
  if (flashing) return "updating";
  if (device.connected) return "connected";
  if (device.bootloader) return "bootloader";
  if (device.foreign) return "foreign";
  return "none";
}

const STATE_COPY: Record<DeviceState, { pill: string; title: string; body: string }> = {
  connected: {
    pill: "LuLu connected",
    title: "LuLu is connected",
    body: "The mouse and keyboard answer over the USB cable.",
  },
  updating: {
    pill: "Updating",
    title: "Updating LuLu",
    body: "Keep the cable plugged in until this finishes.",
  },
  bootloader: {
    pill: "Bootloader",
    title: "LuLu is waiting for firmware",
    body: "Write the LiveUse firmware to start it.",
  },
  foreign: {
    pill: "Other firmware",
    title: "This board runs another program",
    body: "It is not running LiveUse yet.",
  },
  none: {
    pill: "No device",
    title: "Plug in LuLu",
    body: "Use a USB-C cable that carries data.",
  },
};

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
    <div className="grid max-w-md justify-items-center gap-2 text-sm">
      {flashing ? (
        <p>
          {firstSetup ? "First setup. " : ""}
          {phaseLabel(progress?.phase)} {progress?.percent ?? 0}%
        </p>
      ) : null}
      {offer.message && offer.state !== "update" && offer.state !== "published" ? (
        <p>{offer.message}</p>
      ) : null}
      {(offer.state === "update" || offer.state === "published") && offer.available ? (
        <p>Published firmware {offer.available} is available.</p>
      ) : null}
      {offer.state === "published" && offer.available ? (
        <p className="text-muted-foreground">Connect LuLu to update.</p>
      ) : null}
      {flashError ? <p className="text-destructive">{flashError}</p> : null}
      <div className="flex flex-wrap justify-center gap-2">
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
    <div className="flex items-center justify-between gap-4 border-t border-border/60 py-2 first:border-t-0">
      <span className="text-muted-foreground">{label}</span>
      <span className="truncate text-right font-mono text-xs">{value}</span>
    </div>
  );
}

type Tone = "ok" | "busy" | "warn" | "off";

function headerTone(device: DeviceStatus, flashing: boolean): Tone {
  if (flashing) return "busy";
  if (device.connected) return "ok";
  if (device.bootloader || device.foreign) return "warn";
  return "off";
}

const toneDot: Record<Tone, string> = {
  ok: "bg-success",
  busy: "bg-primary animate-pulse",
  warn: "bg-warning",
  off: "bg-muted-foreground/50",
};

function StatusPill({ tone, label }: { tone: Tone; label: string }) {
  return (
    <span
      title={label}
      className="inline-flex shrink-0 items-center justify-center gap-2 rounded-full border border-border bg-card px-2 py-1.5 text-xs font-medium shadow-xs min-[900px]:justify-start min-[900px]:px-3"
    >
      <span className={"size-2 rounded-full " + toneDot[tone]} aria-hidden />
      <span className="sr-only min-[900px]:not-sr-only">{label}</span>
    </span>
  );
}

const hints = [
  {
    title: "LuLu",
    body: "A printed case with a Seeed XIAO SAMD21 inside. Plug it in with a USB-C cable that carries data.",
    link: "Build your LuLu",
    url: "https://github.com/yortyrh/xiao-samd21-case",
  },
  {
    title: "The firmware",
    body: "Plug LuLu in. This tab writes the published LiveUse firmware.",
    link: "Firmware guide",
    url: "https://github.com/yortyrh/liveuse-releases#readme",
  },
  {
    title: "The chip",
    body: "A Seeed XIAO SAMD21. Other XIAO chips do not work in LuLu today.",
    link: "Seeed XIAO guide",
    url: "https://wiki.seeedstudio.com/Seeeduino-XIAO/",
  },
];
function SetupHints() {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Set up LuLu</CardTitle>
      </CardHeader>
      <CardContent className="grid">
        {hints.map((hint) => (
          <div key={hint.title} className="grid gap-1 border-t border-border/60 py-3 first:border-t-0 first:pt-0 text-sm">
            <span className="font-medium">{hint.title}</span>
            <p className="text-muted-foreground">{hint.body}</p>
            <button
              type="button"
              className="inline-flex w-fit cursor-pointer items-center gap-1 rounded text-left text-primary underline-offset-4 outline-none hover:underline focus-visible:ring-3 focus-visible:ring-ring/50"
              onClick={() => void openUrl(hint.url).catch((error: unknown) => toast.error(String(error)))}
            >
              {hint.link}
              <ExternalLink className="size-3" aria-hidden />
            </button>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}
