import type { Metadata } from "next";
import type { ReactNode } from "react";
import { Suspense } from "react";
import { AgentHero } from "@/components/agent-hero";
import { DownloadSkeleton, Downloads } from "@/components/downloads";
import { HardwareSection, SignalPath } from "@/components/hardware";
import { PrimaryLink } from "@/components/primary-link";
import { SiteShell } from "@/components/site-shell";
import { questions } from "@/lib/faq";
import { cn } from "@/lib/utils";

const description =
  "Download LiveUse for Mac. With LuLu plugged in, it publishes a mouse and keyboard API and MCP on your local network.";

export const metadata: Metadata = {
  title: { absolute: "LiveUse for Mac" },
  description,
  robots: { index: true, follow: true },
  openGraph: {
    title: "LiveUse for Mac",
    description,
    type: "website",
    locale: "en_US",
  },
};

const controls = [
  {
    title: "The token decides who can use it",
    body: "The Share tab shows the URL, the token, and the MCP configuration. Issue a new token and the previous clients stop working.",
  },
  {
    title: "You can stop any action",
    body: "Press Control, Option, Command, and Escape together. The current action stops and the LiveUse window opens.",
  },
  {
    title: "It stays on your network",
    body: "The API runs on the Mac with LuLu plugged in and answers on your local network. There is no account and no cloud relay.",
  },
  {
    title: "It keeps running in the menu bar",
    body: "Close the window and LiveUse keeps serving. Quit in the menu bar ends it. A Dock click opens the window again.",
  },
];

const steps = [
  {
    title: "Install LiveUse",
    body: "Open the disk image, drag LiveUse to Applications, and allow screen recording and incoming connections.",
  },
  {
    title: "Plug in LuLu",
    body: "Use a USB-C cable that carries data. The Device tab writes the LiveUse firmware, or updates it.",
  },
  {
    title: "Give your agent the token",
    body: "Copy the URL and the token from the Share tab, or the MCP configuration, into your agent.",
  },
];

export default function Home() {
  const faqLd = {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: questions.map((item) => ({
      "@type": "Question",
      name: item.question,
      acceptedAnswer: { "@type": "Answer", text: item.answer },
    })),
  };

  return (
    <SiteShell>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(faqLd) }}
      />
      <main id="content" tabIndex={-1} className="outline-none">
        <Band className="pt-6 pb-24 sm:pt-10">
          <AgentHero />
        </Band>

        <Band className="border-t border-border py-20">
          <section aria-labelledby="path-title" className="flex flex-col gap-12">
            <div className="flex max-w-[640px] flex-col gap-4">
              <h2 id="path-title" className="display text-4xl sm:text-5xl">
                How a call reaches your cursor
              </h2>
              <p className="text-lg text-pretty">
                Your agent can run anywhere on your network. The pointer it moves is the one on
                the Mac with LuLu plugged in.
              </p>
            </div>
            <SignalPath />
          </section>
        </Band>

        <div className="bg-[#15181d] text-[#e9ecf0] dark:border-y dark:border-border dark:bg-[#181d24]">
          <Band className="py-20">
            <section aria-labelledby="control-title" className="grid gap-12 lg:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)] lg:gap-16">
              <h2 id="control-title" className="display text-4xl sm:text-5xl">
                The agent gets a pointer. Only the token can use it.
              </h2>
              <dl className="grid gap-x-10 gap-y-8 sm:grid-cols-2">
                {controls.map((item) => (
                  <div key={item.title} className="flex flex-col gap-2 border-t border-white/20 pt-4">
                    <dt className="wide text-lg font-semibold">{item.title}</dt>
                    <dd className="text-base text-pretty text-[#a9b0bc]">{item.body}</dd>
                  </div>
                ))}
              </dl>
            </section>
          </Band>
        </div>

        <Band className="py-20">
          <section id="setup" aria-labelledby="setup-title" className="flex scroll-mt-24 flex-col gap-12">
            <h2 id="setup-title" className="display max-w-[640px] text-4xl sm:text-5xl">
              Set up in three steps
            </h2>
            <ol className="grid gap-10 md:grid-cols-3 md:gap-8">
              {steps.map((step, index) => (
                <li key={step.title} className="flex flex-col gap-3">
                  <span aria-hidden="true" className="display text-6xl text-rule">
                    {index + 1}
                  </span>
                  <h3 className="wide text-xl font-semibold">{step.title}</h3>
                  <p className="text-base text-pretty text-muted-foreground">{step.body}</p>
                </li>
              ))}
            </ol>
          </section>
        </Band>

        <Band className="border-t border-border py-20">
          <HardwareSection />
        </Band>

        <Band className="border-t border-border py-20">
          <section id="download" aria-labelledby="download-title" className="flex scroll-mt-24 flex-col gap-10">
            <div className="flex max-w-[640px] flex-col gap-4">
              <h2 id="download-title" className="display text-4xl sm:text-5xl">
                Download
              </h2>
              <p className="text-lg text-pretty">
                One installer for Apple Silicon and one for Intel. To check yours, open the Apple
                menu and choose About This Mac.
              </p>
            </div>
            <Suspense fallback={<DownloadSkeleton />}>
              <Downloads />
            </Suspense>
          </section>
        </Band>

        <Band className="border-t border-border py-20">
          <section
            id="questions"
            aria-labelledby="questions-title"
            className="grid scroll-mt-24 gap-10 lg:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)] lg:gap-16"
          >
            <div className="flex flex-col gap-4 lg:sticky lg:top-28 lg:self-start">
              <h2 id="questions-title" className="display text-4xl sm:text-5xl">
                Questions
              </h2>
              <p className="text-lg text-pretty text-muted-foreground">
                Install, permissions, LuLu, and who can use the token.
              </p>
            </div>
            <div className="border-t border-foreground">
              {questions.map((item) => (
                <details key={item.question} className="group border-b border-border">
                  <summary className="flex cursor-pointer items-center justify-between gap-6 py-5 text-lg font-semibold focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground">
                    {item.question}
                    <span
                      aria-hidden="true"
                      className="faq-mark relative size-4 shrink-0 transition-transform duration-200 before:absolute before:inset-x-0 before:top-1/2 before:h-0.5 before:-translate-y-1/2 before:bg-current after:absolute after:inset-y-0 after:left-1/2 after:w-0.5 after:-translate-x-1/2 after:bg-current"
                    />
                  </summary>
                  <p className="max-w-[60ch] pb-6 text-base text-pretty text-muted-foreground">
                    {item.answer}
                  </p>
                </details>
              ))}
            </div>
          </section>
        </Band>

        <Band className="border-t border-border py-20">
          <section className="flex flex-col items-start gap-6 sm:flex-row sm:items-end sm:justify-between">
            <h2 className="display max-w-[640px] text-4xl sm:text-5xl">
              Put LuLu on your desk and LiveUse on your Mac
            </h2>
            <PrimaryLink href="/#download" className="shrink-0">
              Download for Mac
            </PrimaryLink>
          </section>
        </Band>
      </main>
    </SiteShell>
  );
}

function Band({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("mx-auto w-full max-w-6xl px-5 sm:px-8", className)}>{children}</div>;
}
