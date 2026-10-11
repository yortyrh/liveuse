import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

type OperationEvent = { text?: string | null };

export function Overlay() {
  const [text, setText] = useState("Waiting");

  useEffect(() => {
    const unlisten = listen<OperationEvent>("operation", (event) => {
      setText(event.payload.text?.trim() || "Waiting");
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  return (
    <div className="flex h-screen flex-col justify-center gap-1 bg-background px-3 text-foreground">
      <div className="text-xs text-muted-foreground">LiveUse</div>
      <div className="truncate text-sm font-medium">{text}</div>
      <div className="text-xs text-muted-foreground">Ctrl+Option+Cmd+Esc to stop and open</div>
    </div>
  );
}
