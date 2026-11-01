"use client";

import Image from "next/image";
import { useEffect, useRef, useState } from "react";
import { PrimaryLink, TextLink } from "@/components/primary-link";

type Point = { x: number; y: number };
type Line = { call: string; args?: string; result?: string };

// One short session, in the same `computer` tool calls an agent sends to LiveUse.
// It ends on the Download button and then leaves the page alone.
export function AgentHero() {
  const stageRef = useRef<HTMLDivElement>(null);
  const headlineRef = useRef<HTMLSpanElement>(null);
  const ctaRef = useRef<HTMLAnchorElement>(null);
  const [cursor, setCursor] = useState<Point | null>(null);
  const [lines, setLines] = useState<Line[]>([]);
  const [clicking, setClicking] = useState(false);
  const [pressed, setPressed] = useState(false);
  const done = useRef(false);

  useEffect(() => {
    const stage = stageRef.current;
    const headline = headlineRef.current;
    const cta = ctaRef.current;
    if (!stage || !headline || !cta) return;

    const local = (node: HTMLElement, fx: number, fy: number): Point => {
      const box = stage.getBoundingClientRect();
      const rect = node.getBoundingClientRect();
      return {
        x: Math.round(rect.left - box.left + rect.width * fx),
        y: Math.round(rect.top - box.top + rect.height * fy),
      };
    };
    const screen = (point: Point): string => {
      const box = stage.getBoundingClientRect();
      return `[${Math.round(point.x + box.left)}, ${Math.round(point.y + box.top + window.scrollY)}]`;
    };
    const target = () => local(cta, 0.72, 0.62);

    const finalLines = (): Line[] => {
      const word = local(headline, 0.92, 0.55);
      return [
        { call: "screenshot", result: "1 image" },
        { call: "mouse_move", args: screen(word) },
        { call: "mouse_move", args: screen(target()) },
        { call: "left_click", result: "ok" },
      ];
    };

    const onResize = () => {
      if (done.current) setCursor(target());
    };
    window.addEventListener("resize", onResize);

    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      done.current = true;
      setCursor(target());
      setLines(finalLines());
      return () => window.removeEventListener("resize", onResize);
    }

    const timers: number[] = [];
    const at = (ms: number, run: () => void) => timers.push(window.setTimeout(run, ms));
    const plan = finalLines();
    const start = local(stage, 0.86, 0.9);
    const word = local(headline, 0.92, 0.55);

    setCursor(start);
    at(500, () => setLines(plan.slice(0, 1)));
    at(1300, () => {
      setLines(plan.slice(0, 2));
      setCursor(word);
    });
    at(2600, () => {
      setLines(plan.slice(0, 3));
      setCursor(target());
    });
    at(3600, () => {
      setClicking(true);
      setPressed(true);
    });
    at(3780, () => setPressed(false));
    at(3900, () => {
      setLines(plan);
      done.current = true;
    });
    at(4400, () => setClicking(false));

    return () => {
      timers.forEach((id) => window.clearTimeout(id));
      window.removeEventListener("resize", onResize);
    };
  }, []);

  return (
    <div ref={stageRef} className="relative">
      <div className="grid items-start gap-12 lg:grid-cols-[minmax(0,1.15fr)_minmax(0,0.85fr)] lg:gap-16">
        <div className="flex min-w-0 flex-col gap-7 pt-4 lg:pt-10">
          <h1 className="display text-[2.75rem] text-balance sm:text-6xl lg:text-[4.25rem]">
            Give your agent a real mouse and <span ref={headlineRef}>keyboard</span>
          </h1>
          <p className="max-w-[34rem] text-lg text-pretty text-muted-foreground">
            LuLu is a small USB device your Mac sees as a mouse and a keyboard. LiveUse, the Mac
            app, shares both on your local network as an API and an MCP, behind a token you
            control.
          </p>
          <div className="flex flex-wrap items-center gap-x-6 gap-y-4">
            <PrimaryLink
              ref={ctaRef}
              href="/#download"
              className={pressed ? "translate-y-px bg-primary/85" : undefined}
            >
              Download for Mac
            </PrimaryLink>
            <TextLink href="/#setup" className="text-base">
              See how it works
            </TextLink>
          </div>
          <p className="text-sm text-muted-foreground">
            Free, no account. Each installer lists its SHA256 checksum. Linux and Windows are in
            progress.
          </p>
        </div>

        <figure className="relative flex min-w-0 flex-col">
          <div className="mm-paper relative flex aspect-[5/4] items-center justify-center overflow-hidden rounded-t-lg border border-b-0 border-border">
            <Image
              src="/case/assembled.png"
              alt="LuLu, closed: a blue printed case with the USB-C port at the front."
              width={900}
              height={653}
              preload
              fetchPriority="high"
              sizes="(min-width: 1024px) 22rem, 70vw"
              className="h-auto w-[64%] drop-shadow-[0_22px_22px_rgba(16,28,60,0.28)]"
            />
            <Dimension />
          </div>
          <div
            className="rounded-b-lg bg-[#15181d] px-4 py-4 font-mono text-[0.75rem] leading-6 text-[#c9ced6] sm:px-5"
            aria-label="An agent session: a screenshot, two pointer moves, and a click on Download for Mac."
            role="img"
          >
            <div className="mb-2 flex items-center justify-between gap-3 text-[#8a929e]">
              <span>lulu.local:8765/mcp</span>
              <span className="flex items-center gap-2 text-pointer">
                <span className="size-1.5 rounded-full bg-pointer" />
                LuLu connected
              </span>
            </div>
            <ol className="min-h-24">
              {lines.map((line, index) => (
                <li key={index} className="log-line flex gap-3 whitespace-nowrap">
                  <span className="text-[#5f6773]">computer</span>
                  <span className="text-white">{line.call}</span>
                  {line.args ? <span className="text-pointer">{line.args}</span> : null}
                  {line.result ? <span className="text-spark">{line.result}</span> : null}
                </li>
              ))}
            </ol>
          </div>
          <figcaption className="mt-3 text-sm text-muted-foreground">
            LuLu, and the calls an agent sends to LiveUse to reach the button on this page.
          </figcaption>
        </figure>
      </div>

      {cursor ? (
        <div
          aria-hidden="true"
          className="agent-cursor pointer-events-none absolute top-0 left-0 z-20"
          style={{ transform: `translate(${cursor.x}px, ${cursor.y}px)` }}
        >
          <span className={clicking ? "agent-click relative block" : "relative block"}>
            <svg width="26" height="34" viewBox="24 16 56 80" className="-translate-x-[2px] -translate-y-[2px]">
              <path
                d="M26 18 L26 84 L44 66 L58 94 L68 89 L54 61 L78 61 Z"
                fill="#3EE0E8"
                stroke="#071316"
                strokeWidth="4"
                strokeLinejoin="round"
              />
            </svg>
          </span>
        </div>
      ) : null}
    </div>
  );
}

function Dimension() {
  return (
    <div
      aria-hidden="true"
      className="absolute inset-x-[18%] bottom-[9%] flex items-center gap-2 font-mono text-[0.6875rem] text-brand"
    >
      <span className="h-3 w-px bg-brand" />
      <span className="h-px flex-1 bg-brand" />
      <span className="px-1">25.0 × 21.6 × 8.9 mm</span>
      <span className="h-px flex-1 bg-brand" />
      <span className="h-3 w-px bg-brand" />
    </div>
  );
}
