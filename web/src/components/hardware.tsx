import Image from "next/image";
import type { ReactNode } from "react";
import { ArrowUpRight } from "@phosphor-icons/react/ssr";
import { TextLink } from "@/components/primary-link";
import { Reveal } from "@/components/reveal";
import { formatBytes, getFirmware } from "@/lib/firmware";
import { BOARD_WIKI, CASE_REPO, FIRMWARE_DOCS, RELEASES_REPO } from "@/lib/links";

type Picture = { src: string; w: number; h: number; width: string };

const parts: {
  label: string;
  title: string;
  body: string;
  href: string;
  image: Picture | null;
}[] = [
  {
    label: "01 · App",
    title: "LiveUse for Mac",
    body: "Talks to LuLu over USB and shares it on your network as an API and an MCP.",
    href: "/#download",
    image: null,
  },
  {
    label: "02 · Device",
    title: "LuLu",
    body: "A printed case with a Seeed XIAO SAMD21 inside. 25 × 21.6 × 8.9 mm, USB-C.",
    href: "/#lulu",
    image: { src: "/case/board-in-base.png", w: 900, h: 824, width: "w-[40%]" },
  },
  {
    label: "03 · Firmware",
    title: "What runs inside",
    body: "One cable shows up as a mouse, a keyboard, and a serial port. LiveUse writes it for you.",
    href: "/#firmware",
    image: { src: "/case/cutaway.png", w: 900, h: 556, width: "w-[52%]" },
  },
];

export function PartsStrip() {
  return (
    <ol className="grid gap-4 md:grid-cols-3">
      {parts.map((part, index) => (
        <li key={part.label}>
          <Reveal delay={index * 80} className="h-full">
            <a
              href={part.href}
              className="group flex h-full flex-col overflow-hidden rounded-2xl border border-border bg-card transition-[transform,border-color] duration-500 ease-[cubic-bezier(0.32,0.72,0,1)] hover:-translate-y-1 hover:border-primary/50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground"
            >
              <div className="relative flex aspect-[16/9] items-center justify-center bg-stage">
                {part.image === null ? (
                  <AppGlyph />
                ) : (
                  <Image
                    src={part.image.src}
                    alt=""
                    width={part.image.w}
                    height={part.image.h}
                    sizes="(min-width: 768px) 14rem, 60vw"
                    className={`h-auto ${part.image.width} drop-shadow-[0_12px_18px_rgba(8,20,48,0.28)] transition-transform duration-700 ease-[cubic-bezier(0.32,0.72,0,1)] group-hover:scale-[1.04]`}
                  />
                )}
              </div>
              <div className="flex flex-1 flex-col gap-2 p-5">
                <span className="font-mono text-xs font-medium tracking-wide text-brand uppercase">
                  {part.label}
                </span>
                <h3 className="flex items-center gap-1 text-lg font-semibold">
                  {part.title}
                  <ArrowUpRight
                    size={16}
                    aria-hidden="true"
                    className="text-muted-foreground transition-transform group-hover:-translate-y-0.5 group-hover:translate-x-0.5"
                  />
                </h3>
                <p className="text-pretty text-sm text-muted-foreground">{part.body}</p>
              </div>
            </a>
          </Reveal>
        </li>
      ))}
    </ol>
  );
}

function AppGlyph() {
  return (
    <div className="flex w-[62%] flex-col gap-2 rounded-xl border border-white/10 bg-[#0d1830] p-3 shadow-[0_12px_24px_rgba(8,20,48,0.35)]">
      <div className="flex gap-1.5" aria-hidden="true">
        <span className="size-2 rounded-full bg-white/25" />
        <span className="size-2 rounded-full bg-white/25" />
        <span className="size-2 rounded-full bg-white/25" />
      </div>
      <div className="flex items-center justify-between rounded-md bg-white/5 px-2 py-1.5">
        <span className="font-mono text-[10px] text-white/70">LuLu</span>
        <span className="flex items-center gap-1 font-mono text-[10px] text-emerald-300">
          <span className="size-1.5 rounded-full bg-emerald-400" />
          connected
        </span>
      </div>
      <div className="flex items-center justify-between rounded-md bg-white/5 px-2 py-1.5">
        <span className="font-mono text-[10px] text-white/70">MCP</span>
        <span className="font-mono text-[10px] text-[#8ab8f5]">:8765/mcp</span>
      </div>
    </div>
  );
}

export function HardwareSection() {
  return (
    <section id="lulu" className="flex scroll-mt-24 flex-col gap-10">
      <Reveal>
        <div className="flex max-w-[680px] flex-col gap-4">
          <span className="font-mono text-xs font-medium tracking-wide text-brand uppercase">
            The device
          </span>
          <h2 className="text-3xl font-semibold">Meet LuLu</h2>
          <p className="text-pretty text-lg">
            The pointer is a real USB device on your desk. LuLu is a small printed case with a
            Seeed XIAO SAMD21 inside, running the LiveUse firmware. Your Mac sees a mouse, a
            keyboard, and a serial port.
          </p>
        </div>
      </Reveal>

      <Reveal>
        <div className="grid overflow-hidden rounded-2xl border border-border bg-card lg:grid-cols-[1.15fr_1fr]">
          <div className="relative flex min-h-80 flex-col items-center justify-center gap-6 bg-stage p-8">
            <Image
              src="/case/board-in-base.png"
              alt="LuLu with the lid off: the Seeed XIAO SAMD21 seated inside the case."
              width={900}
              height={824}
              sizes="(min-width: 1024px) 24rem, 70vw"
              className="h-auto w-[62%] max-w-96 drop-shadow-[0_18px_26px_rgba(8,20,48,0.32)]"
            />
            <span className="rounded-full border border-border bg-card/80 px-3 py-1 font-mono text-xs text-muted-foreground backdrop-blur">
              LuLu, lid off · XIAO SAMD21 inside
            </span>
          </div>
          <div className="flex flex-col gap-5 p-6 sm:p-8">
            <div className="flex flex-col gap-2">
              <h3 className="text-xl font-semibold">What is inside</h3>
              <p className="text-pretty text-base text-muted-foreground">
                Four posts in the lid hold the chip board at its corners. The USB-C exits through
                a U opening, so any data cable fits.
              </p>
            </div>
            <Specs
              rows={[
                ["Size", "25.0 × 21.6 × 8.9 mm"],
                ["Inside", "Seeed XIAO SAMD21, ATSAMD21G18A"],
                ["CPU", "Cortex-M0+ 48 MHz, 256 KB flash, 32 KB RAM"],
                ["USB, running", "2886:802F · LiveUse"],
                ["USB, bootloader", "2886:002F · Seeeduino XIAO"],
              ]}
            />
            <div className="grid grid-cols-3 gap-3">
              <Thumb src="/case/assembled.png" w={900} h={653} alt="LuLu closed." />
              <Thumb src="/case/exploded.png" w={900} h={1077} alt="LuLu taken apart: lid, chip board, and base." />
              <Thumb src="/case/cutaway.png" w={900} h={556} alt="Cross-section of LuLu with the chip board inside." />
            </div>
            <p className="text-pretty text-sm text-muted-foreground">
              Use a USB-C cable that carries data. XIAO boards with ESP32-S3, RP2040, or nRF52840
              need a different image and do not work in LuLu today.
            </p>
          </div>
        </div>
      </Reveal>

      <div className="grid gap-6 lg:grid-cols-2">
        <Reveal className="h-full">
          <Panel id="build">
            <div className="flex items-start gap-5">
              <div className="flex size-28 shrink-0 items-center justify-center rounded-xl bg-stage p-2">
                <Image
                  src="/case/exploded.png"
                  alt="LuLu taken apart: lid, XIAO SAMD21, and base."
                  width={900}
                  height={1077}
                  sizes="7rem"
                  className="h-full w-auto drop-shadow-[0_8px_12px_rgba(8,20,48,0.22)]"
                />
              </div>
              <div className="flex flex-col gap-2">
                <h3 className="text-xl font-semibold">Build your LuLu</h3>
                <p className="text-pretty text-base text-muted-foreground">
                  Print the base and the snap-fit lid, drop the XIAO SAMD21 in with the USB-C
                  toward the opening, and press the lid on.
                </p>
              </div>
            </div>
            <Specs
              rows={[
                ["Material", "PLA or PETG, 0.4 mm nozzle"],
                ["Layer", "0.2 mm, 0.12 to 0.16 mm for a tighter fit"],
                ["Supports", "None. Parts come oriented"],
                ["Files", "3MF and STL, base and lid"],
              ]}
            />
            <div className="flex flex-wrap gap-4">
              <TextLink href={CASE_REPO}>Case files</TextLink>
              <TextLink href={BOARD_WIKI}>Seeed XIAO guide</TextLink>
            </div>
          </Panel>
        </Reveal>

        <Reveal className="h-full" delay={80}>
          <Panel id="firmware">
            <div className="flex flex-col gap-2">
              <h3 className="text-xl font-semibold">LuLu firmware</h3>
              <p className="text-pretty text-base text-muted-foreground">
                One cable, three interfaces: a HID mouse, a boot keyboard, and a CDC serial port at
                115200 baud. A single Arduino loop reads one line and runs it. There is no RTOS.
              </p>
            </div>
            <FirmwareFacts />
            <p className="text-pretty text-sm text-muted-foreground">
              The Device tab checks the public manifest and writes new firmware onto LuLu. To flash
              by hand, bossac writes the image at <code className="font-mono">0x2000</code> and keeps
              the bootloader.
            </p>
            <div className="flex flex-wrap gap-4">
              <TextLink href={FIRMWARE_DOCS}>Flashing and commands</TextLink>
              <TextLink href="/help#serial">Serial commands</TextLink>
            </div>
          </Panel>
        </Reveal>
      </div>

      <Reveal>
        <p className="max-w-[680px] text-pretty text-base text-muted-foreground">
          In progress: other chips are being explored for LuLu, including the first steps of a
          smaller device that plugs straight into the computer. They are not available yet.
        </p>
      </Reveal>
    </section>
  );
}

async function FirmwareFacts() {
  const firmware = await getFirmware();
  if (!firmware) {
    return (
      <Specs
        rows={[
          ["Image", "Published in liveuse-releases"],
          ["Offset", "0x2000"],
        ]}
        footer={<TextLink href={`${RELEASES_REPO}/releases`}>Firmware releases</TextLink>}
      />
    );
  }
  return (
    <Specs
      rows={[
        ["Version", firmware.version],
        ["Image", `${firmware.board.file} · ${formatBytes(firmware.board.size)}`],
        ["Offset", firmware.board.offset],
        ["SHA256", firmware.board.sha256],
      ]}
      footer={
        <TextLink href={`${RELEASES_REPO}/releases/tag/${firmware.tag}`}>
          Release {firmware.version}
        </TextLink>
      }
    />
  );
}

function Panel({ id, children }: { id?: string; children: ReactNode }) {
  return (
    <article
      id={id}
      className="flex h-full scroll-mt-24 flex-col gap-5 rounded-2xl border border-border bg-card p-6 sm:p-8"
    >
      {children}
    </article>
  );
}

function Specs({ rows, footer }: { rows: [string, string][]; footer?: ReactNode }) {
  return (
    <div className="flex flex-col gap-3">
      <dl className="divide-y divide-border rounded-lg border border-border bg-muted/50 text-sm">
        {rows.map(([term, value]) => (
          <div key={term} className="grid grid-cols-[8.5rem_minmax(0,1fr)] gap-3 px-3 py-2">
            <dt className="text-muted-foreground">{term}</dt>
            <dd className="font-mono text-xs leading-5 [overflow-wrap:anywhere]">{value}</dd>
          </div>
        ))}
      </dl>
      {footer}
    </div>
  );
}

function Thumb({ src, w, h, alt }: { src: string; w: number; h: number; alt: string }) {
  return (
    <div className="flex aspect-square items-center justify-center rounded-lg bg-stage p-2">
      <Image
        src={src}
        alt={alt}
        width={w}
        height={h}
        sizes="7rem"
        className="h-auto max-h-full w-full object-contain"
      />
    </div>
  );
}
