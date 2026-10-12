import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardAction, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { saveScrollDirection, scrollDirection, scrollProbe, scrollProbeNudge } from "@/lib/backend";
import { invertedFromProbe } from "@/lib/scroll-direction";

type Phase = "idle" | "probe" | "nudge" | "away" | "manual";

export function ScrollDirection({ connected }: { connected: boolean }) {
  const probeRef = useRef<HTMLDivElement>(null);
  const phaseRef = useRef<Phase>("idle");
  const wheelRef = useRef(0);
  const decidedRef = useRef(false);
  const pointerRef = useRef(false);
  const watchingPointerRef = useRef(false);
  const timerRef = useRef<number | null>(null);
  const commitRef = useRef<(inverted: boolean) => void>(() => undefined);
  const [phase, setPhase] = useState<Phase>("idle");
  const [inverted, setInverted] = useState<boolean | null>(null);
  const [shift, setShift] = useState(0);

  function setPhaseNow(next: Phase) {
    phaseRef.current = next;
    setPhase(next);
  }

  function clearTimer() {
    if (timerRef.current != null) {
      window.clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }

  function resetWheel() {
    wheelRef.current = 0;
    setShift(0);
  }

  commitRef.current = (next) => {
    clearTimer();
    void saveScrollDirection(next)
      .then(() => {
        setInverted(next);
        setPhaseNow("idle");
        toast.success(
          next
            ? "Wheel compensated. Positive scrolls up again."
            : "Not compensated. Positive already scrolls up.",
        );
      })
      .catch((error: unknown) => {
        decidedRef.current = false;
        toast.error(error instanceof Error ? error.message : String(error));
      });
  };

  useEffect(() => {
    void scrollDirection()
      .then((settings) => setInverted(settings.inverted))
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    const el = probeRef.current;
    if (!el) {
      return;
    }
    const consider = () => {
      if (phaseRef.current === "idle" || decidedRef.current) {
        return;
      }
      const result = invertedFromProbe(0, wheelRef.current);
      if (result === null) {
        return;
      }
      decidedRef.current = true;
      commitRef.current(result);
    };
    const onWheel = (event: WheelEvent) => {
      if (phaseRef.current === "idle" || decidedRef.current) {
        return;
      }
      event.preventDefault();
      if (Math.abs(event.deltaX) > Math.abs(event.deltaY)) {
        return;
      }
      wheelRef.current += event.deltaY;
      setShift(Math.max(-18, Math.min(18, wheelRef.current)));
      window.setTimeout(consider, 50);
    };
    const onPointerMove = () => {
      if (watchingPointerRef.current) {
        pointerRef.current = true;
      }
    };
    el.addEventListener("wheel", onWheel, { passive: false });
    el.addEventListener("pointermove", onPointerMove);
    return () => {
      el.removeEventListener("wheel", onWheel);
      el.removeEventListener("pointermove", onPointerMove);
    };
  }, []);

  async function start() {
    clearTimer();
    decidedRef.current = false;
    resetWheel();
    if (!connected) {
      setPhaseNow("manual");
      return;
    }
    setPhaseNow("probe");
    try {
      await scrollProbe();
    } catch (error: unknown) {
      setPhaseNow("manual");
      toast.error(error instanceof Error ? error.message : String(error));
      return;
    }
    if (decidedRef.current) {
      return;
    }
    timerRef.current = window.setTimeout(() => {
      void confirmPointer();
    }, 1000);
  }

  async function confirmPointer() {
    const phase = phaseRef.current;
    if (decidedRef.current || phase !== "probe") {
      return;
    }
    setPhaseNow("nudge");
    pointerRef.current = false;
    watchingPointerRef.current = true;
    try {
      await scrollProbeNudge();
      await new Promise((resolve) => window.setTimeout(resolve, 400));
    } catch (error: unknown) {
      watchingPointerRef.current = false;
      if (phaseRef.current === "nudge") {
        setPhaseNow("manual");
      }
      toast.error(error instanceof Error ? error.message : String(error));
      return;
    }
    watchingPointerRef.current = false;
    if (decidedRef.current || phaseRef.current !== "nudge") {
      return;
    }
    setPhaseNow(pointerRef.current ? "manual" : "away");
  }

  function cancel() {
    clearTimer();
    decidedRef.current = true;
    watchingPointerRef.current = false;
    setPhaseNow("idle");
    resetWheel();
  }

  const badge =
    phase === "probe" || phase === "nudge"
      ? "Checking"
      : phase === "away"
        ? "Off the button"
        : phase === "manual"
          ? "Your turn"
          : inverted === null
            ? "Not checked"
            : inverted
              ? "Compensated"
              : "Normal";

  const hint =
    phase === "probe"
      ? "The pointer stays on the button. The wheel is being read there."
      : phase === "nudge"
        ? "The scroll did not arrive. The mouse moves a little to see if it is still over the button."
        : phase === "away"
          ? "Leave the mouse over the check button and press Check again."
          : phase === "manual"
            ? "The mouse is over the button, but the wheel was not seen. Roll the wheel forward: the top edge moves away from you."
            : inverted === true
              ? "The wheel moved the content down. The agent compensates so positive still scrolls up."
              : inverted === false
                ? "The wheel moved the content up. No compensation is needed."
                : connected
                  ? "Press Check and do not move the mouse. The test uses that same button."
                  : "Press Check and, without moving the mouse, roll the wheel over the button.";

  const label =
    phase === "probe" || phase === "nudge"
      ? "Checking"
      : phase === "manual"
        ? "Roll the wheel here"
        : inverted === null || phase === "away"
          ? "Check"
          : "Again";

  return (
    <Card>
      <CardHeader>
        <CardTitle>Wheel direction</CardTitle>
        <CardDescription>
          Positive has to scroll up on screen. If the system inverts the wheel, it is measured on the button and the XIAO compensates, including horizontally.
        </CardDescription>
        <CardAction>
          <Badge variant={phase === "away" || inverted === null ? "secondary" : "default"}>{badge}</Badge>
        </CardAction>
      </CardHeader>
      <CardContent className="grid gap-3">
        <p className="text-sm text-muted-foreground" aria-live="polite">
          {hint}
        </p>
        <div ref={probeRef} className="w-fit">
          <Button
            className="relative h-14 min-w-44 overflow-hidden"
            variant={phase === "manual" ? "outline" : "default"}
            onClick={() => {
              if (phase === "probe" || phase === "nudge") {
                return;
              }
              if (phase === "manual") {
                cancel();
                return;
              }
              void start();
            }}
          >
            <span
              aria-hidden
              className="pointer-events-none absolute inset-x-3 top-1/2 h-px bg-current opacity-40"
              style={{ transform: `translateY(${shift}px)` }}
            />
            <span className="relative">{label}</span>
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
