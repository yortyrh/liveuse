import type { Metadata } from "next";
import type { ReactNode } from "react";
import { Suspense } from "react";
import { AppWindow, Key, Mouse, SealCheck } from "@phosphor-icons/react/ssr";
import { DownloadSkeleton, Downloads } from "@/components/downloads";
import { HeroStage } from "@/components/hero-stage";
import { HardwareSection, PartsStrip } from "@/components/hardware";
import { PrimaryLink } from "@/components/primary-link";
import { Reveal } from "@/components/reveal";
import { SiteShell } from "@/components/site-shell";
import { Tagline } from "@/components/tagline";
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

const benefits = [
  {
    icon: Mouse,
    title: "A pointer the agent can call",
    body: "Mouse and keyboard travel over a local API and MCP while LuLu is plugged in.",
  },
  {
    icon: AppWindow,
    title: "Close the window and keep it",
    body: "LiveUse stays in the menu bar. Quit in that menu ends the app. A Dock click opens the window again.",
  },
  {
    icon: Key,
    title: "Replace the token when you want",
    body: "The Share tab shows the URL, the token, and the MCP configuration. A new token stops the previous clients.",
  },
  {
    icon: SealCheck,
    title: "Check the file before you open it",
    body: "Each installer shows its size and SHA256 checksum. Copy the checksum and compare it.",
  },
];

const steps = [
  {
    title: "Install the app",
    body: "Open the disk image, drag LiveUse to Applications, and allow screen recording plus incoming connections.",
  },
  {
    title: "Plug in LuLu",
    body: "Connect LuLu with a data cable. The Device tab writes the LiveUse firmware, or updates it. The bootloader does not start the service.",
  },
  {
    title: "Share the token",
    body: "Copy the URL and the token from the Share tab. Closing the window leaves the service in the menu bar.",
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
        <Band className="pt-8 pb-12">
          <div className="grid items-center gap-12 lg:grid-cols-2">
            <Reveal className="min-w-0">
              <div className="flex max-w-[680px] flex-col gap-6">
                <span className="w-fit rounded-full border border-border bg-card px-3 py-1 font-mono text-xs text-muted-foreground">
                  macOS app · LuLu, the USB device · firmware
                </span>
                <h1 className="flex flex-col gap-2 text-4xl font-semibold sm:text-5xl">
                  <span className="hero-heading text-balance">The local pointer</span>
                  <span className="text-balance text-foreground">
                    with <span className="text-brand">LiveUse</span>
                  </span>
                </h1>
                <p className="text-pretty text-lg">
                  LiveUse is the macOS app for LuLu, a small USB device that is a mouse and a
                  keyboard. With LuLu plugged in, it publishes an API and an MCP for both, then
                  stays in the menu bar when you close the window.
                </p>
                <PrimaryLink href="/#download">Download for Mac</PrimaryLink>
                <p className="text-sm font-semibold text-muted-foreground">
                  Free on macOS, with a public checksum. Linux and Windows are in progress.
                </p>
              </div>
            </Reveal>
            <Reveal className="min-w-0" delay={120}>
              <HeroStage />
            </Reveal>
          </div>
        </Band>

        <Band className="pb-12">
          <section aria-label="Three parts" className="flex flex-col gap-6">
            <Reveal>
              <p className="max-w-[680px] text-pretty text-base text-muted-foreground">
                Three parts work together. The app runs on your Mac. LuLu is a printed case with
                a Seeed XIAO SAMD21 inside. The firmware runs on that chip.
              </p>
            </Reveal>
            <PartsStrip />
          </section>
        </Band>

        <Band className="py-12">
          <Reveal>
            <section className="flex max-w-[680px] flex-col gap-4">
              <h2 className="text-3xl font-semibold">The pointer stays on your Mac</h2>
              <p className="text-pretty text-lg">
                An agent on another computer cannot move your cursor by itself. LiveUse runs on
                that Mac, with LuLu on the desk, and publishes the mouse and the keyboard
                on the local network.
              </p>
            </section>
          </Reveal>
        </Band>

        <Band className="py-12">
          <section className="flex flex-col gap-8">
            <Reveal>
              <h2 className="max-w-[680px] text-3xl font-semibold">What you get</h2>
            </Reveal>
            <div className="grid gap-6 md:grid-cols-2">
              {benefits.map((benefit, index) => {
                const Icon = benefit.icon;
                return (
                  <Reveal key={benefit.title} delay={index * 80}>
                    <article className="flex h-full flex-col gap-4 rounded-2xl border border-border bg-card p-6">
                      <span className="flex size-10 items-center justify-center rounded-xl bg-primary/10 text-primary">
                        <Icon size={22} aria-hidden="true" />
                      </span>
                      <h3 className="text-xl font-semibold">{benefit.title}</h3>
                      <p className="text-pretty text-base text-muted-foreground">{benefit.body}</p>
                    </article>
                  </Reveal>
                );
              })}
            </div>
          </section>
        </Band>

        <Band className="py-24">
          <Tagline />
        </Band>

        <Band className="py-12">
          <section id="setup" className="flex scroll-mt-24 flex-col gap-8">
            <Reveal>
              <h2 className="max-w-[680px] text-3xl font-semibold">How it works</h2>
            </Reveal>
            <ol className="grid gap-8 md:grid-cols-3">
              {steps.map((step, index) => (
                <li key={step.title}>
                  <Reveal delay={index * 80}>
                    <div className="flex flex-col gap-4 border-t-2 border-brand/40 pt-4">
                      <span className="font-mono text-sm font-medium text-brand">0{index + 1}</span>
                      <h3 className="text-xl font-semibold">{step.title}</h3>
                      <p className="text-pretty text-base">{step.body}</p>
                    </div>
                  </Reveal>
                </li>
              ))}
            </ol>
          </section>
        </Band>

        <Band className="py-12">
          <HardwareSection />
        </Band>

        <Band className="py-12">
          <section id="download" className="flex scroll-mt-24 flex-col gap-8">
            <Reveal>
              <div className="flex max-w-[680px] flex-col gap-4">
                <h2 className="text-3xl font-semibold">Download</h2>
                <p className="text-pretty text-lg">
                  One installer for Intel and one for Apple Silicon. Open the Apple menu and
                  choose About This Mac. If it says Chip, choose Apple Silicon. If it says
                  Processor, choose Intel.
                </p>
              </div>
            </Reveal>
            <Suspense fallback={<DownloadSkeleton />}>
              <Downloads />
            </Suspense>
          </section>
        </Band>

        <Band className="py-12">
          <section id="questions" className="flex scroll-mt-24 flex-col gap-8">
            <Reveal>
              <div className="flex max-w-[680px] flex-col gap-4">
                <h2 className="text-3xl font-semibold">Questions</h2>
                <p className="text-pretty text-lg">
                  Install, permissions, LuLu, and who can use the token.
                </p>
              </div>
            </Reveal>
            <div className="grid gap-8 md:grid-cols-2">
              {questions.map((item, index) => (
                <Reveal key={item.question} delay={index * 40}>
                  <article className="flex flex-col gap-3">
                    <h3 className="text-xl font-semibold">{item.question}</h3>
                    <p className="text-pretty text-base text-muted-foreground">{item.answer}</p>
                  </article>
                </Reveal>
              ))}
            </div>
          </section>
        </Band>

        <Band className="pt-12 pb-24">
          <Reveal>
            <section className="flex max-w-[680px] flex-col gap-6">
              <h2 className="text-4xl font-semibold text-balance">Download LiveUse for your Mac</h2>
              <p className="text-pretty text-lg">
                Free, with a public checksum on each file. No account.
              </p>
              <PrimaryLink href="/#download">Download for Mac</PrimaryLink>
            </section>
          </Reveal>
        </Band>
      </main>
    </SiteShell>
  );
}

function Band({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("mx-auto w-full max-w-6xl px-4 sm:px-6", className)}>{children}</div>;
}
