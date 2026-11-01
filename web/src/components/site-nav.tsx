"use client";

import { useCallback, useEffect, useId, useRef, useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { Logo } from "@/components/logo";
import { ThemeToggle } from "@/components/theme-toggle";
import { motion } from "@/lib/motion";
import { cn } from "@/lib/utils";

const links = [
  { href: "/#download", id: "download", label: "Download" },
  { href: "/#setup", id: "setup", label: "Setup" },
  { href: "/#lulu", id: "lulu", label: "LuLu" },
  { href: "/#questions", id: "questions", label: "Questions" },
  { href: "/help", id: "help", label: "Help" },
];

const delays = ["delay-100", "delay-150", "delay-200", "delay-300", "delay-500"];

export function SiteNav() {
  const pathname = usePathname();
  const menuId = useId();
  const buttonRef = useRef<HTMLButtonElement>(null);
  const firstLinkRef = useRef<HTMLAnchorElement>(null);
  const closeTimer = useRef<number | null>(null);
  const [open, setOpen] = useState(false);
  const [shown, setShown] = useState(false);
  const [active, setActive] = useState("");

  const closeMenu = useCallback(() => {
    setShown(false);
    buttonRef.current?.focus();
    if (closeTimer.current !== null) window.clearTimeout(closeTimer.current);
    closeTimer.current = window.setTimeout(() => setOpen(false), 700);
  }, []);

  useEffect(() => {
    if (pathname !== "/") return;
    const nodes = links
      .map((link) => document.getElementById(link.id))
      .filter((node): node is HTMLElement => node !== null);
    if (nodes.length === 0) return;

    const ratios = new Map<string, number>();
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          ratios.set(entry.target.id, entry.isIntersecting ? entry.intersectionRatio : 0);
        }
        let best = "";
        let bestRatio = 0;
        for (const link of links) {
          const ratio = ratios.get(link.id) ?? 0;
          if (ratio > bestRatio) {
            best = link.id;
            bestRatio = ratio;
          }
        }
        setActive(best);
      },
      { rootMargin: "-30% 0px -55% 0px", threshold: [0.15, 0.4, 0.7] },
    );
    for (const node of nodes) observer.observe(node);
    return () => observer.disconnect();
  }, [pathname]);

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeMenu();
    };
    const previous = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    window.addEventListener("keydown", onKey);
    return () => {
      document.body.style.overflow = previous;
      window.removeEventListener("keydown", onKey);
    };
  }, [open, closeMenu]);

  useEffect(() => {
    if (shown) firstLinkRef.current?.focus();
  }, [shown]);

  useEffect(() => {
    const query = window.matchMedia("(min-width: 768px)");
    const onChange = () => {
      if (!query.matches) return;
      setShown(false);
      setOpen(false);
    };
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  useEffect(() => {
    return () => {
      if (closeTimer.current !== null) window.clearTimeout(closeTimer.current);
    };
  }, []);

  function openMenu() {
    if (closeTimer.current !== null) window.clearTimeout(closeTimer.current);
    setOpen(true);
    setShown(false);
    requestAnimationFrame(() => {
      requestAnimationFrame(() => setShown(true));
    });
  }

  function marked(id: string, href: string) {
    if (href === "/help") return pathname === "/help";
    return active === id;
  }

  return (
    <>
      <div className="pointer-events-none fixed inset-x-0 top-0 z-50 flex justify-center">
        <nav
          aria-label="Main"
          className="pointer-events-auto mt-6 flex w-max items-center gap-2 rounded-full border border-border bg-card/85 px-2 py-2 shadow-sm backdrop-blur-xl"
        >
          <Link
            href="/"
            aria-current={pathname === "/" ? "page" : undefined}
            className={cn(itemClass(false), "inline-flex items-center gap-2")}
          >
            <Logo className="size-6" />
            LiveUse
          </Link>
          <div className="hidden items-center gap-2 md:flex">
            {links.map((link) => (
              <a
                key={link.id}
                href={link.href}
                aria-current={marked(link.id, link.href) ? (link.href === "/help" ? "page" : "location") : undefined}
                className={itemClass(marked(link.id, link.href))}
              >
                {link.label}
              </a>
            ))}
          </div>
          <ThemeToggle />
          <button
            ref={buttonRef}
            type="button"
            className={cn(
              "relative size-10 cursor-pointer rounded-full text-foreground md:hidden",
              motion,
              "hover:bg-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground active:scale-[0.98]",
            )}
            aria-expanded={open}
            aria-controls={menuId}
            aria-label={open ? "Close menu" : "Open menu"}
            onClick={() => (open ? closeMenu() : openMenu())}
          >
            <span
              className={cn(
                "absolute right-3 left-3 h-0.5 bg-current",
                motion,
                open ? "top-1/2 -translate-y-1/2 rotate-45" : "top-3",
              )}
            />
            <span
              className={cn(
                "absolute right-3 left-3 h-0.5 bg-current",
                motion,
                open ? "top-1/2 -translate-y-1/2 -rotate-45" : "bottom-3",
              )}
            />
          </button>
        </nav>
      </div>
      {open ? (
        <div
          id={menuId}
          className={cn(
            "fixed inset-0 z-40 flex flex-col items-center justify-center gap-8 bg-background/85 backdrop-blur-3xl",
            motion,
            shown ? "opacity-100" : "opacity-0",
          )}
        >
          {links.map((link, index) => (
            <a
              key={link.id}
              ref={index === 0 ? firstLinkRef : undefined}
              href={link.href}
              onClick={closeMenu}
              aria-current={marked(link.id, link.href) ? (link.href === "/help" ? "page" : "location") : undefined}
              className={cn(
                "text-4xl font-semibold text-foreground",
                motion,
                delays[index],
                shown ? "translate-y-0 opacity-100" : "translate-y-12 opacity-0",
                "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground",
              )}
            >
              {link.label}
            </a>
          ))}
        </div>
      ) : null}
    </>
  );
}

function itemClass(current: boolean) {
  return cn(
    "cursor-pointer rounded-full px-3 py-2 text-sm font-semibold",
    motion,
    "hover:bg-muted hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground active:scale-[0.98]",
    current ? "bg-muted text-foreground" : "text-muted-foreground",
  );
}
