"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";

export function CopyHash({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2000);
    } catch {
      setCopied(false);
    }
  }

  return (
    <Button type="button" variant="outline" size="sm" onClick={copy}>
      {copied ? "Copiado" : "Copiar"}
    </Button>
  );
}
