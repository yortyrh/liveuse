import type { Metadata } from "next";
import { PrimaryLink } from "@/components/primary-link";
import { SiteShell } from "@/components/site-shell";

export const metadata: Metadata = {
  title: "Terms",
  description: "What you get when you download LiveUse, and what the token can do.",
};

export default function TermsPage() {
  return (
    <SiteShell>
      <main
        id="content"
        tabIndex={-1}
        className="mx-auto flex w-full max-w-[680px] flex-col gap-6 px-4 py-12 outline-none sm:px-6"
      >
        <h1 className="display text-5xl">Terms</h1>
        <p className="text-pretty text-base">
          LiveUse is a free macOS download. You receive the installer as it was published. Compare
          the checksum on the download page with the file before you open it.
        </p>
        <p className="text-pretty text-base">
          When a build is not signed, macOS can block the first open. Right click the app and
          choose Open if you trust the file.
        </p>
        <p className="text-pretty text-base">
          The token controls the mouse and the keyboard on the local network. Anyone who has it on
          that network can use them. Issue a new token in the app when you want the old one to
          stop.
        </p>
        <PrimaryLink href="/">Back to LiveUse</PrimaryLink>
      </main>
    </SiteShell>
  );
}
