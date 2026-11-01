import type { Metadata } from "next";
import type { ReactNode } from "react";
import Image from "next/image";
import { SiteShell } from "@/components/site-shell";
import { CASE_PRINT, CASE_REPO, FIRMWARE_DOCS } from "@/lib/links";

export const metadata: Metadata = {
  title: "Help",
  description:
    "Use LiveUse on the Mac with LuLu, or drive that pointer from another computer on the same network.",
};

export default function HelpPage() {
  return (
    <SiteShell>
      <main
        id="content"
        tabIndex={-1}
        className="mx-auto flex w-full max-w-6xl flex-col gap-16 px-5 py-12 outline-none sm:px-8"
      >
        <header className="flex max-w-[680px] flex-col gap-4">
          <h1 className="display text-5xl text-balance">Use the pointer</h1>
          <p className="text-pretty text-lg">
            On the Mac with LuLu, LiveUse talks to it over USB. On any other
            computer on that network, the same pointer is an API and an MCP, gated by the token.
          </p>
        </header>

        <section className="flex flex-col gap-8">
          <div className="flex max-w-[680px] flex-col gap-4">
            <h2 className="display text-3xl sm:text-4xl">On this Mac</h2>
            <p className="text-pretty text-lg">
              Plug in LuLu with the firmware running. The Device tab should say Connected, and
              the service should be listening. The bootloader does not start the service. When a
              newer firmware is published, this tab writes it onto LuLu.
            </p>
          </div>
          <Shot
            src="/help/device.png"
            alt="LiveUse Device tab with LuLu connected and a firmware update ready to install."
            caption="LuLu is connected, and a published firmware update is ready to write."
            width={1960}
            height={1520}
          />
          <p className="max-w-[680px] text-pretty text-base">
            Open Control and leave This machine selected. Write what you want done. The agent looks
            at the screen and uses the mouse and the keyboard. Closing the window leaves LiveUse
            in the menu bar.
          </p>
          <Shot
            src="/help/control-opus.png"
            alt="LiveUse Control tab with Anthropic and Claude Opus 5.5 selected."
            caption="This machine, with Anthropic and Claude Opus 5.5."
            width={1960}
            height={470}
          />
        </section>

        <section className="flex flex-col gap-8">
          <div className="flex max-w-[680px] flex-col gap-4">
            <h2 className="display text-3xl sm:text-4xl">From another computer</h2>
            <p className="text-pretty text-lg">
              The Share tab on the Mac with LuLu shows the API URL, the token, and the MCP
              configuration. Anyone with that token on the local network can move the cursor and
              the keyboard. Issue a new token there when you want the old one to stop.
            </p>
          </div>
          <Shot
            src="/help/share.png"
            alt="LiveUse Share tab with an example URL, an example token, and an MCP configuration."
            caption="Copy these from the Mac with LuLu. The values on your machine will differ."
            width={1960}
            height={1520}
          />
          <div className="flex max-w-[680px] flex-col gap-4">
            <h3 className="text-xl font-semibold">MCP</h3>
            <p className="text-pretty text-base">
              Point the agent at the MCP URL. The header is a bearer token. A second Mac, or a
              cluster of them, is the same shape with one server per machine.
            </p>
          </div>
          <Sample>{mcpExample}</Sample>
          <div className="flex max-w-[680px] flex-col gap-4">
            <h3 className="text-xl font-semibold">HTTP</h3>
            <p className="text-pretty text-base">
              The same token calls the API. A screenshot comes back as a PNG. A click moves the
              pointer first, then presses.
            </p>
          </div>
          <Sample>{sizeExample}</Sample>
          <Sample>{clickExample}</Sample>
          <Sample>{shotExample}</Sample>
          <p className="max-w-[680px] text-pretty text-base">
            Useful calls are screenshot, click, move_to, write, and scroll. Coordinates start at
            the top left of the primary display.
          </p>
          <div className="flex max-w-[680px] flex-col gap-4">
            <h3 className="text-xl font-semibold">Remote from LiveUse itself</h3>
            <p className="text-pretty text-base">
              On the other computer, open Control, choose Remote, and paste the URL and the token
              from the first Mac. Connect. The chat on this computer then drives the pointer over
              there.
            </p>
          </div>
          <Shot
            src="/help/remote-form.png"
            alt="LiveUse Control tab set to Remote, with fields for the other computer URL and token."
            caption="Remote is how one LiveUse window drives another Mac on the network."
            width={1960}
            height={820}
          />
        </section>

        <section className="flex max-w-[680px] flex-col gap-6">
          <h2 className="display text-3xl sm:text-4xl">A cluster</h2>
          <p className="text-pretty text-lg">
            Each Mac that should be driven needs its own LuLu, its own LiveUse, and its own token.
            Give every server a name in the agent config. The agent then chooses which pointer to
            use.
          </p>
          <Sample>{clusterExample}</Sample>
          <p className="text-pretty text-base">
            Desk and bench in that example are two computers on one network. The tokens are not
            interchangeable. Regenerate one and only that machine stops accepting the old token.
          </p>
        </section>

        <section className="grid items-start gap-8 md:grid-cols-[minmax(0,280px)_minmax(0,1fr)] md:gap-12">
          <div className="flex w-full max-w-[280px] items-center justify-center rounded-2xl bg-stage p-6">
            <Image
              src="/case/board-in-base.png"
              alt="LuLu with the lid off: the Seeed XIAO SAMD21 seated in the base."
              width={900}
              height={824}
              sizes="280px"
              className="h-auto w-full drop-shadow-[0_12px_18px_rgba(8,20,48,0.25)]"
            />
          </div>
          <div className="flex max-w-[680px] flex-col gap-4">
            <h2 className="display text-3xl sm:text-4xl">LuLu</h2>
            <p className="text-pretty text-lg">
              LuLu is a printed case with a Seeed XIAO SAMD21 inside. One USB-C cable shows up as
              a HID mouse, a boot keyboard, and a serial port. A single Arduino loop reads one line and
              runs it. There is no RTOS.
            </p>
            <p className="text-pretty text-base">
              With the firmware running, LuLu reports USB id <code className="font-mono">2886:802F</code>{" "}
              and the product name LiveUse. In the bootloader it reports{" "}
              <code className="font-mono">2886:002F</code>, Seeeduino XIAO. To enter the bootloader by
              hand, short the two RST pads twice in a row.
            </p>
            <p className="text-pretty text-base">
              The case prints in two parts without supports. The files and the print settings are
              in{" "}
              <a className="font-semibold underline underline-offset-4" href={CASE_REPO}>
                xiao-samd21-case
              </a>
              , and the model is ready to slice on{" "}
              <a className="font-semibold underline underline-offset-4" href={CASE_PRINT}>
                Creality Cloud
              </a>
              .
            </p>
            <p className="text-pretty text-base">
              This is a work in progress. Other boards are being explored, and the first steps are
              underway for a smaller device that plugs straight into the computer. The desktop app
              is macOS for now. Linux and Windows are in progress.
            </p>
          </div>
        </section>

        <section id="serial" className="flex scroll-mt-24 flex-col gap-8">
          <div className="flex max-w-[680px] flex-col gap-4">
            <h2 className="display text-3xl sm:text-4xl">The serial port</h2>
            <p className="text-pretty text-lg">
              LiveUse speaks to LuLu in plain lines over its serial port, at 115200 baud. You
              can do the same from a terminal or your own program while LiveUse is closed. The port
              cannot be shared.
            </p>
            <p className="text-pretty text-base">
              One line is one command, ending in LF or CRLF. The command word is case-insensitive.
              Mouse and keyboard commands finish with a line that ends in{" "}
              <code className="font-mono">done</code>. A line longer than 95 characters answers{" "}
              <code className="font-mono">ERR line too long</code>.
            </p>
          </div>
          <div className="overflow-x-auto rounded-2xl border border-border bg-card">
            <table className="w-full min-w-[560px] text-left text-sm">
              <thead className="bg-muted/60 text-muted-foreground">
                <tr>
                  <th scope="col" className="px-4 py-3 font-semibold">Command</th>
                  <th scope="col" className="px-4 py-3 font-semibold">What it does</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {serialCommands.map(([command, action]) => (
                  <tr key={command}>
                    <td className="px-4 py-3 align-top font-mono text-xs whitespace-nowrap">{command}</td>
                    <td className="px-4 py-3 text-pretty">{action}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <Sample>{serialExample}</Sample>
          <p className="max-w-[680px] text-pretty text-base">
            Key names, error replies, manual flashing with bossac at{" "}
            <code className="font-mono">0x2000</code>, and example scripts in Python and Web
            Serial are in the{" "}
            <a className="font-semibold underline underline-offset-4" href={FIRMWARE_DOCS}>
              firmware guide
            </a>
            .
          </p>
        </section>

        <section className="flex max-w-[680px] flex-col gap-6">
          <h2 className="display text-3xl sm:text-4xl">Models</h2>
          <p className="text-pretty text-lg">
            LiveUse does not ship a model. The pictures above use Anthropic, Claude Opus 5.5.
            Change, on the Control tab, saves whichever company and model look at the screen and
            call the pointer.
          </p>
          <ul className="flex flex-col gap-4">
            <li className="text-pretty text-base">
              <span className="font-semibold">Claude Opus 5.5 and Claude Fable 5.1.</span> From
              Anthropic. Sonnet does this well too.
            </li>
            <li className="text-pretty text-base">
              <span className="font-semibold">Holo, Qwen, and UI-TARS.</span> Built to use a
              computer, so they fit this job.
            </li>
            <li className="text-pretty text-base">
              <span className="font-semibold">GPT-6 Astra and GPT-5.4.</span> From OpenAI.
            </li>
            <li className="text-pretty text-base">
              <span className="font-semibold">Gemini 3.1 Pro.</span> From Google.
            </li>
          </ul>
          <p className="text-pretty text-base">
            Other models that can read a screenshot and call tools also work. Press Control,
            Option, Command, and Escape together to abort the current action and open the window.
            Stop in the chat does the same thing.
          </p>
        </section>
      </main>
    </SiteShell>
  );
}

function Shot({
  src,
  alt,
  caption,
  width,
  height,
}: {
  src: string;
  alt: string;
  caption: string;
  width: number;
  height: number;
}) {
  return (
    <figure className="flex flex-col gap-3">
      <Image
        src={src}
        alt={alt}
        width={width}
        height={height}
        sizes="(min-width: 1152px) 69rem, 100vw"
        className="h-auto w-full rounded-2xl border border-border"
      />
      <figcaption className="text-pretty text-sm text-muted-foreground">{caption}</figcaption>
    </figure>
  );
}

function Sample({ children }: { children: ReactNode }) {
  return (
    <pre className="max-w-full overflow-x-auto rounded-lg bg-muted p-4 font-mono text-sm">
      <code>{children}</code>
    </pre>
  );
}

const mcpExample = `{
  "mcpServers": {
    "liveuse": {
      "url": "http://192.168.1.20:8765/mcp",
      "headers": {
        "Authorization": "Bearer exampletoken"
      }
    }
  }
}`;

const clusterExample = `{
  "mcpServers": {
    "desk": {
      "url": "http://192.168.1.20:8765/mcp",
      "headers": { "Authorization": "Bearer exampletoken" }
    },
    "bench": {
      "url": "http://192.168.1.21:8765/mcp",
      "headers": { "Authorization": "Bearer anothertoken" }
    }
  }
}`;

const sizeExample = `curl -H "Authorization: Bearer exampletoken" \\
  http://192.168.1.20:8765/v1/size`;

const clickExample = `curl -H "Authorization: Bearer exampletoken" \\
  -H "Content-Type: application/json" \\
  -d '{"x":120,"y":80}' \\
  http://192.168.1.20:8765/v1/click`;

const shotExample = `curl -H "Authorization: Bearer exampletoken" \\
  -o screen.png \\
  http://192.168.1.20:8765/v1/screenshot`;

const serialCommands: [string, string][] = [
  ["HELP or ?", "List the commands."],
  ["STATUS", "Uptime and version: OK uptime_ms=<ms> fw=<version>."],
  ["ECHO <text>", "Send the text back. A line that is not a command is echoed too."],
  ["MOVE <dx> <dy>", "Glide the pointer in small steps. Positive dx is right, positive dy is down. Each axis is clamped to ±2000."],
  ["CLICK left|right|double", "Click. double is two left clicks."],
  ["MDOWN / MUP left|right|middle", "Hold or release a button. MOVE while held drags."],
  ["SCROLL <steps>", "Vertical wheel. Positive is up, negative is down."],
  ["HSCROLL <steps>", "Horizontal wheel. Positive is right, negative is left."],
  ["TYPE <ascii>", "Type printable ASCII. Prints nothing on success."],
  ["KEY <chord>", "Tap one key with optional modifiers, like cmd+shift+a."],
  ["KDOWN / KUP <key>", "Hold or release a key. Up to 6 keys at once."],
  ["HOTKEY <k1+k2+…>", "Press the steps in order, then release them in reverse."],
];

const serialExample = `screen /dev/cu.usbmodem1101 115200

STATUS
OK uptime_ms=18420 fw=0.5.10
MOVE 80 0
MOVE 80 0
MOVE done`;
