"use client";

import { usePathname } from "next/navigation";
import { TextLink } from "@/components/primary-link";

const links = [
  { href: "/privacy", label: "Privacy" },
  { href: "/terms", label: "Terms" },
];

export function SiteFooter() {
  const pathname = usePathname();

  return (
    <footer className="bg-muted">
      <div className="mx-auto flex w-full max-w-6xl flex-col gap-4 px-4 py-8 sm:flex-row sm:items-center sm:justify-between sm:px-6">
        <p className="text-sm font-semibold">LiveUse</p>
        <nav aria-label="Footer" className="flex flex-wrap gap-4">
          {links.map((link) => (
            <TextLink key={link.href} href={link.href} current={pathname === link.href}>
              {link.label}
            </TextLink>
          ))}
          <TextLink href="https://github.com/yortyrh/liveuse-releases">Public releases</TextLink>
        </nav>
      </div>
    </footer>
  );
}
