"use client";

import { useState } from "react";
import { secondaryButtonClass } from "@/components/primary-link";

export function CopyHash({ value }: { value: string }) {
  const [state, setState] = useState<"idle" | "copied" | "error">("idle");

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setState("copied");
      window.setTimeout(() => setState("idle"), 2000);
    } catch {
      setState("error");
    }
  }

  const label =
    state === "copied" ? "Copied" : state === "error" ? "Try again" : "Copy checksum";

  return (
    <div className="flex flex-col gap-2">
      <button type="button" onClick={() => void copy()} className={secondaryButtonClass}>
        {label}
      </button>
      {state === "error" ? (
        <p role="alert" className="text-pretty text-sm text-destructive">
          Could not copy the checksum.
        </p>
      ) : null}
      <p role="status" className="sr-only">
        {state === "copied" ? "Checksum copied." : ""}
      </p>
    </div>
  );
}
