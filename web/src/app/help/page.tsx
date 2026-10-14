import type { Metadata } from "next";
import type { ReactNode } from "react";
import Image from "next/image";
import { SiteShell } from "@/components/site-shell";
import { XiaoBoard } from "@/components/xiao-board";

export const metadata: Metadata = {
  title: "Help",
  description:
    "Use LiveUse on the Mac with the board, or drive that pointer from another computer on the same network.",
};

export default function HelpPage() {
  return (
    <SiteShell>
      <main
        id="content"
        tabIndex={-1}
        className="mx-auto flex w-full max-w-6xl flex-col gap-16 px-4 py-12 outline-none sm:px-6"
      >
        <header className="flex max-w-[680px] flex-col gap-4">
          <h1 className="text-4xl font-semibold text-balance">Use the pointer</h1>
          <p className="text-pretty text-lg">
            On the Mac with the Seeed XIAO, LiveUse talks to the board over USB. On any other
            computer on that network, the same pointer is an API and an MCP, gated by the token.
          </p>
        </header>

        <section className="flex flex-col gap-8">
          <div className="flex max-w-[680px] flex-col gap-4">
            <h2 className="text-3xl font-semibold">On this Mac</h2>
            <p className="text-pretty text-lg">
              Plug in the board with the sketch running. The Device tab should say Connected, and
              the service should be listening. The bootloader does not start the service. When a
              newer firmware is published, this tab writes it onto the board.
            </p>
          </div>
          <Shot
            src="/help/device.png"
            alt="LiveUse Device tab with the XIAO connected and a firmware update ready to install."
            caption="The board is connected, and a published firmware update is ready to write."
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
            <h2 className="text-3xl font-semibold">From another computer</h2>
            <p className="text-pretty text-lg">
              The Share tab on the Mac with the board shows the API URL, the token, and the MCP
              configuration. Anyone with that token on the local network can move the cursor and
              the keyboard. Issue a new token there when you want the old one to stop.
            </p>
          </div>
          <Shot
            src="/help/share.png"
            alt="LiveUse Share tab with an example URL, an example token, and an MCP configuration."
            caption="Copy these from the Mac with the board. The values on your machine will differ."
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
          <h2 className="text-3xl font-semibold">A cluster</h2>
          <p className="text-pretty text-lg">
            Each Mac that should be driven needs its own board, its own LiveUse, and its own token.
            Give every server a name in the agent config. The agent then chooses which pointer to
            use.
          </p>
          <Sample>{clusterExample}</Sample>
          <p className="text-pretty text-base">
            Desk and bench in that example are two computers on one network. The tokens are not
            interchangeable. Regenerate one and only that machine stops accepting the old token.
          </p>
        </section>

        <section className="grid items-center gap-12 lg:grid-cols-[minmax(0,280px)_minmax(0,1fr)]">
          <div className="rounded-2xl bg-white p-6 shadow-md">
            <XiaoBoard className="h-auto w-full" />
          </div>
          <div className="flex max-w-[680px] flex-col gap-4">
            <h2 className="text-3xl font-semibold">The board</h2>
            <p className="text-pretty text-lg">
              Today the sketch runs on a Seeed XIAO SAMD21. It uses FreeRTOS.
            </p>
            <p className="text-pretty text-base">
              This is a work in progress. Other boards are being explored, and the first steps are
              underway for a smaller board that plugs straight into the computer. The desktop app
              is macOS for now. Linux and Windows are in progress.
            </p>
          </div>
        </section>

        <section className="flex max-w-[680px] flex-col gap-6">
          <h2 className="text-3xl font-semibold">Models</h2>
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
