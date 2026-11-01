"use client";

import { usePathname } from "next/navigation";
import { Logo } from "@/components/logo";
import { TextLink } from "@/components/primary-link";
import { CASE_PRINT, CASE_REPO, FIRMWARE_DOCS, RELEASES_REPO } from "@/lib/links";

const projects = [
  { href: `${RELEASES_REPO}/releases`, label: "App and firmware releases" },
  { href: FIRMWARE_DOCS, label: "Firmware guide" },
  { href: CASE_REPO, label: "LuLu case files" },
  { href: CASE_PRINT, label: "LuLu on Creality Cloud" },
];

const links = [
  { href: "/privacy", label: "Privacy" },
  { href: "/terms", label: "Terms" },
];

export function SiteFooter() {
  const pathname = usePathname();

  return (
    <footer className="border-t border-border bg-stage">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-6 px-5 py-12 sm:flex-row sm:items-start sm:justify-between sm:px-8">
        <div className="flex flex-col gap-2">
          <p className="wide inline-flex items-center gap-2 text-base font-semibold">
            <Logo className="size-6" />
            LiveUse
          </p>
          <p className="max-w-xs text-sm text-muted-foreground">
            A Mac app and LuLu, the USB device: its firmware and its printed case.
          </p>
        </div>
        <nav aria-label="Projects" className="flex flex-col gap-3">
          {projects.map((link) => (
            <TextLink key={link.href} href={link.href}>
              {link.label}
            </TextLink>
          ))}
        </nav>
        <nav aria-label="Footer" className="flex flex-col gap-3">
          {links.map((link) => (
            <TextLink key={link.href} href={link.href} current={pathname === link.href}>
              {link.label}
            </TextLink>
          ))}
        </nav>
      </div>
    </footer>
  );
}
