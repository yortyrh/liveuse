"use client";

import { Moon, Sun } from "@phosphor-icons/react";
import { useTheme } from "next-themes";
import { motion } from "@/lib/motion";
import { cn } from "@/lib/utils";

export function ThemeToggle() {
  const { resolvedTheme, setTheme } = useTheme();

  return (
    <button
      type="button"
      aria-label="Switch theme"
      onClick={() => setTheme(resolvedTheme === "dark" ? "light" : "dark")}
      className={cn(
        "inline-flex size-10 cursor-pointer items-center justify-center rounded-full text-foreground",
        motion,
        "hover:bg-muted focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-foreground active:scale-[0.98]",
      )}
    >
      <Sun className="hidden dark:block" size={16} aria-hidden="true" />
      <Moon className="dark:hidden" size={16} aria-hidden="true" />
    </button>
  );
}
