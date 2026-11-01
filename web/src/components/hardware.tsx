import Image from "next/image";
import { Fragment, type ReactNode } from "react";
import { TextLink } from "@/components/primary-link";
import { formatBytes, getFirmware } from "@/lib/firmware";
import { BOARD_WIKI, CASE_PRINT, CASE_REPO, FIRMWARE_DOCS, RELEASES_REPO } from "@/lib/links";

const path = [
  {
    name: "Your agent",
    where: "Any computer on your network",
    body: "Sends mouse and keyboard calls with the token, through the API or the MCP.",
  },
  {
    name: "LiveUse",
    where: "The Mac with LuLu plugged in",
    body: "Checks the token and passes each call to LuLu. It stays in the menu bar.",
  },
  {
    name: "LuLu",
    where: "On the desk, in a USB-C port",
    body: "Moves the real cursor and types, as a USB mouse and keyboard.",
  },
];

const hops = ["HTTP on port 8765, with the token", "USB-C, serial commands"];

export function SignalPath() {
  return (
    <ol className="grid lg:grid-cols-[1fr_auto_1fr_auto_1fr]">
      {path.map((node, index) => (
        <Fragment key={node.name}>
          <li className="flex flex-col gap-2 border-l-2 border-foreground py-1 pl-5 lg:border-t-2 lg:border-l-0 lg:pt-5 lg:pl-0">
            <span className="text-sm text-muted-foreground">{node.where}</span>
            <h3 className="wide text-2xl font-semibold">{node.name}</h3>
            <p className="max-w-[22rem] text-base text-pretty text-muted-foreground">{node.body}</p>
          </li>
          {index < hops.length ? (
            <li className="flex items-center gap-3 py-5 lg:w-44 lg:flex-col lg:items-stretch lg:gap-3 lg:py-0">
              <span
                aria-hidden="true"
                className="relative h-10 w-0.5 bg-pointer-ink after:absolute after:-bottom-1 after:-left-[3px] after:border-x-4 after:border-t-[6px] after:border-x-transparent after:border-t-pointer-ink lg:h-0.5 lg:w-full lg:after:top-[-3px] lg:after:right-[-1px] lg:after:bottom-auto lg:after:left-auto lg:after:border-x-0 lg:after:border-y-4 lg:after:border-l-[6px] lg:after:border-y-transparent lg:after:border-l-pointer-ink"
              />
              <span className="font-mono text-[0.6875rem] leading-5 text-pointer-ink lg:px-3 lg:text-center">
                <span className="sr-only">Then over </span>
                {hops[index]}
              </span>
            </li>
          ) : null}
        </Fragment>
      ))}
    </ol>
  );
}

export function HardwareSection() {
  return (
    <section id="lulu" className="flex scroll-mt-24 flex-col gap-12">
      <div className="flex max-w-[640px] flex-col gap-4">
        <h2 className="display text-4xl sm:text-5xl">Meet LuLu</h2>
        <p className="text-lg text-pretty">
          A printed case with a Seeed XIAO SAMD21 inside, running the LiveUse firmware. Your Mac
          sees a mouse, a keyboard, and a serial port. You can print the case yourself.
        </p>
      </div>

      <div className="grid overflow-hidden rounded-lg border border-foreground/70 lg:grid-cols-[minmax(0,1.3fr)_minmax(0,1fr)]">
        <div className="mm-paper grid min-h-96 grid-cols-[1.1fr_1fr] items-center gap-4 p-6 sm:p-10">
          <View
            src="/case/exploded.png"
            w={900}
            h={1077}
            alt="LuLu taken apart: the snap-fit lid, the XIAO SAMD21, and the base."
            label="Exploded"
            className="w-full max-w-72"
          />
          <div className="flex flex-col gap-8">
            <View
              src="/case/board-in-base.png"
              w={900}
              h={824}
              alt="LuLu with the lid off: the XIAO SAMD21 seated in the base."
              label="Lid off"
              className="w-[82%]"
            />
            <View
              src="/case/cutaway.png"
              w={900}
              h={556}
              alt="Cross-section of LuLu with the board held by the lid posts."
              label="Section"
              className="w-full"
            />
          </div>
        </div>
        <div className="flex flex-col border-t border-foreground/70 bg-card lg:border-t-0 lg:border-l">
          <TitleBlock
            rows={[
              ["Part", "LuLu"],
              ["Size", "25.0 × 21.6 × 8.9 mm"],
              ["Inside", "Seeed XIAO SAMD21, ATSAMD21G18A"],
              ["CPU", "Cortex-M0+ 48 MHz, 256 KB flash, 32 KB RAM"],
              ["USB, running", "2886:802F LiveUse"],
              ["USB, bootloader", "2886:002F Seeeduino XIAO"],
              ["Cable", "USB-C that carries data"],
            ]}
          />
          <p className="mt-auto border-t border-border p-5 text-sm text-pretty text-muted-foreground">
            Four posts in the lid hold the board at its corners. The USB-C port exits through a U
            opening, so any data cable fits. XIAO boards with ESP32-S3, RP2040, or nRF52840 need
            a different image and do not work in LuLu today.
          </p>
        </div>
      </div>

      <div className="grid gap-12 lg:grid-cols-2 lg:gap-0 lg:divide-x lg:divide-border">
        <article id="build" className="flex scroll-mt-24 flex-col gap-5 lg:pr-12">
          <h3 className="wide text-2xl font-semibold">Print the case</h3>
          <p className="text-base text-pretty text-muted-foreground">
            Print the base and the snap-fit lid, drop the XIAO SAMD21 in with the USB-C toward the
            opening, and press the lid on.
          </p>
          <Specs
            rows={[
              ["Material", "PLA or PETG, 0.4 mm nozzle"],
              ["Layer", "0.2 mm, or 0.12 to 0.16 mm for a tighter fit"],
              ["Supports", "None. Parts come oriented"],
              ["Files", "3MF and STL, base and lid"],
            ]}
          />
          <div className="flex flex-wrap gap-x-6 gap-y-3">
            <TextLink href={CASE_PRINT}>Print it from Creality Cloud</TextLink>
            <TextLink href={CASE_REPO}>Case source on GitHub</TextLink>
            <TextLink href={BOARD_WIKI}>Seeed XIAO guide</TextLink>
          </div>
        </article>

        <article id="firmware" className="flex scroll-mt-24 flex-col gap-5 lg:pl-12">
          <h3 className="wide text-2xl font-semibold">The firmware</h3>
          <p className="text-base text-pretty text-muted-foreground">
            One cable, three interfaces: a HID mouse, a boot keyboard, and a CDC serial port at
            115200 baud. The Device tab in LiveUse writes it onto LuLu and keeps it updated.
          </p>
          <FirmwareFacts />
          <p className="text-sm text-pretty text-muted-foreground">
            To flash by hand, bossac writes the image at <code className="font-mono">0x2000</code>{" "}
            and keeps the bootloader.
          </p>
          <div className="flex flex-wrap gap-x-6 gap-y-3">
            <TextLink href={FIRMWARE_DOCS}>Flashing and commands</TextLink>
            <TextLink href="/help#serial">Serial commands</TextLink>
          </div>
        </article>
      </div>

      <p className="max-w-[640px] text-base text-pretty text-muted-foreground">
        Other chips are being explored for LuLu, including a smaller device that plugs straight
        into the computer. They are not available yet.
      </p>
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
        ["Image", `${firmware.board.file}, ${formatBytes(firmware.board.size)}`],
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

function TitleBlock({ rows }: { rows: [string, string][] }) {
  return (
    <dl className="divide-y divide-border">
      {rows.map(([term, value]) => (
        <div key={term} className="grid grid-cols-[8rem_minmax(0,1fr)] gap-3 px-5 py-3">
          <dt className="text-sm text-muted-foreground">{term}</dt>
          <dd className="text-sm font-medium [overflow-wrap:anywhere]">{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function Specs({ rows, footer }: { rows: [string, string][]; footer?: ReactNode }) {
  return (
    <div className="flex flex-col gap-3">
      <dl className="divide-y divide-border border-y border-border text-sm">
        {rows.map(([term, value]) => (
          <div key={term} className="grid grid-cols-[7rem_minmax(0,1fr)] gap-3 py-2">
            <dt className="text-muted-foreground">{term}</dt>
            <dd className="font-mono text-xs leading-5 [overflow-wrap:anywhere]">{value}</dd>
          </div>
        ))}
      </dl>
      {footer}
    </div>
  );
}

function View({
  src,
  w,
  h,
  alt,
  label,
  className,
}: {
  src: string;
  w: number;
  h: number;
  alt: string;
  label: string;
  className?: string;
}) {
  return (
    <figure className="flex flex-col items-center gap-2">
      <Image
        src={src}
        alt={alt}
        width={w}
        height={h}
        sizes="(min-width: 1024px) 18rem, 45vw"
        className={`h-auto drop-shadow-[0_14px_18px_rgba(16,28,60,0.22)] ${className ?? ""}`}
      />
      <figcaption className="font-mono text-[0.6875rem] text-brand">{label}</figcaption>
    </figure>
  );
}
