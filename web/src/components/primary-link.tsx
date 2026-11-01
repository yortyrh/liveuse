import type React from "react";
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";
import { motion } from "@/lib/motion";

const focus =
  "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground";

export function PrimaryLink({
  href,
  children,
  className,
  ref,
}: {
  href: string;
  children: ReactNode;
  className?: string;
  ref?: React.Ref<HTMLAnchorElement>;
}) {
  return (
    <a
      ref={ref}
      href={href}
      className={cn(
        "wide inline-flex w-fit cursor-pointer items-center justify-center gap-2 rounded-md bg-primary px-5 py-3 text-base font-semibold text-primary-foreground",
        motion,
        "hover:bg-primary/85 active:translate-y-px data-[pressed=true]:translate-y-px data-[pressed=true]:bg-primary/85",
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
        "cursor-pointer text-sm font-semibold text-foreground underline decoration-rule decoration-1 underline-offset-4",
        motion,
        "hover:decoration-foreground hover:decoration-2",
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
  "inline-flex w-full cursor-pointer items-center justify-center rounded-md border border-border bg-transparent px-4 py-3 text-base font-semibold text-foreground",
  motion,
  "hover:bg-muted active:translate-y-px",
  focus,
);
