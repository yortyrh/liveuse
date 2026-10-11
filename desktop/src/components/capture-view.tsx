import { useEffect, useId, useState } from "react";

export function CaptureView({ src, alt }: { src: string; alt: string }) {
  const [open, setOpen] = useState(false);
  const titleId = useId();

  useEffect(() => {
    if (!open) {
      return;
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  return (
    <>
      <button
        type="button"
        className="grid w-full gap-1 rounded-lg text-left outline-none focus-visible:ring-3 focus-visible:ring-ring/50"
        onClick={() => setOpen(true)}
      >
        <img src={src} alt={alt} className="max-h-96 w-full rounded-md bg-muted object-contain" />
        <span className="text-xs text-muted-foreground">Enlarge</span>
      </button>
      {open ? (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby={titleId}
          className="fixed inset-0 z-50 flex items-center justify-center p-4"
        >
          <button
            type="button"
            aria-label="Close capture"
            className="absolute inset-0 bg-black/70"
            onClick={() => setOpen(false)}
          />
          <div className="relative grid max-h-[92vh] max-w-[92vw] gap-2">
            <p id={titleId} className="text-sm text-white">
              {alt}
            </p>
            <img src={src} alt={alt} className="max-h-[84vh] max-w-[92vw] object-contain" />
            <button
              type="button"
              className="justify-self-end rounded-lg bg-white px-3 py-1.5 text-sm text-black"
              onClick={() => setOpen(false)}
            >
              Close
            </button>
          </div>
        </div>
      ) : null}
    </>
  );
}
