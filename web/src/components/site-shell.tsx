import type { ReactNode } from "react";
import { SiteFooter } from "@/components/site-footer";
import { SiteNav } from "@/components/site-nav";

export function SiteShell({ children }: { children: ReactNode }) {
  return (
    <>
      <a
        href="#content"
        className="sr-only focus:not-sr-only focus:fixed focus:top-4 focus:left-4 focus:z-50 focus:inline-flex focus:rounded-lg focus:bg-primary focus:px-3 focus:py-2 focus:text-base focus:font-semibold focus:text-primary-foreground focus:outline-2 focus:outline-offset-2 focus:outline-foreground"
      >
        Skip to content
      </a>
      <SiteNav />
      <div className="flex flex-1 flex-col pt-16">{children}</div>
      <SiteFooter />
    </>
  );
}
