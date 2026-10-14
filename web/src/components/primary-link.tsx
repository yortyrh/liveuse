import type { ReactNode } from "react";
import { cn } from "@/lib/utils";
import { motion } from "@/lib/motion";

const focus =
  "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground";

export function PrimaryLink({
  href,
  children,
  className,
}: {
  href: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <a
      href={href}
      className={cn(
        "inline-flex w-fit cursor-pointer items-center justify-center rounded-lg bg-primary px-3 py-2 text-base font-semibold text-primary-foreground",
        motion,
        "hover:bg-primary/80 active:scale-[0.98]",
        focus,
        className,
      )}
    >
      {children}
    </a>
  );
}

export function TextLink({
  href,
  children,
  className,
  current = false,
}: {
  href: string;
  children: ReactNode;
  className?: string;
  current?: boolean;
}) {
  return (
    <a
      href={href}
      aria-current={current ? "page" : undefined}
      className={cn(
        "cursor-pointer text-sm font-semibold text-foreground underline decoration-transparent underline-offset-4",
        motion,
        "hover:decoration-foreground active:scale-[0.98]",
        focus,
        current && "decoration-foreground",
        className,
      )}
    >
      {children}
    </a>
  );
}

export const secondaryButtonClass = cn(
  "inline-flex w-full cursor-pointer items-center justify-center rounded-lg border border-border bg-background px-3 py-2 text-sm font-semibold text-foreground",
  motion,
  "hover:bg-muted active:scale-[0.98]",
  focus,
);
