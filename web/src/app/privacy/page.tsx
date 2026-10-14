import type { Metadata } from "next";
import { PrimaryLink } from "@/components/primary-link";
import { SiteShell } from "@/components/site-shell";

export const metadata: Metadata = {
  title: "Privacy",
  description: "What the LiveUse download site stores, and what it requests from GitHub.",
};

export default function PrivacyPage() {
  return (
    <SiteShell>
      <main
        id="content"
        tabIndex={-1}
        className="mx-auto flex w-full max-w-[680px] flex-col gap-6 px-4 py-12 outline-none sm:px-6"
      >
        <h1 className="text-4xl font-semibold">Privacy</h1>
        <p className="text-pretty text-base">
          This site does not ask you to create an account, and it does not run analytics. Your
          theme choice is stored in this browser.
        </p>
        <p className="text-pretty text-base">
          The page reads a public release list from GitHub and links each installer there. The
          download itself happens on GitHub.
        </p>
        <PrimaryLink href="/">Back to LiveUse</PrimaryLink>
      </main>
    </SiteShell>
  );
}
