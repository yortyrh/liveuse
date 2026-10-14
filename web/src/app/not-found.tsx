import { PrimaryLink } from "@/components/primary-link";
import { SiteShell } from "@/components/site-shell";

export default function NotFound() {
  return (
    <SiteShell>
      <main
        id="content"
        tabIndex={-1}
        className="mx-auto flex w-full max-w-[680px] flex-1 flex-col gap-6 px-4 py-16 outline-none sm:px-6"
      >
        <h1 className="text-4xl font-semibold text-balance">This page does not exist</h1>
        <p className="text-pretty text-lg">The address does not match a page on this site.</p>
        <PrimaryLink href="/">Back to LiveUse</PrimaryLink>
      </main>
    </SiteShell>
  );
}
