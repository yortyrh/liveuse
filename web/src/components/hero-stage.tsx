import Image from "next/image";

export function HeroStage() {
  return (
    <div className="relative mx-auto w-full rounded-3xl bg-stage p-4 pb-16 sm:p-6 sm:pb-20">
      <div className="relative overflow-hidden rounded-2xl border border-border bg-white shadow-xl">
        <Image
          src="/help/device-hero.png"
          alt="LiveUse Device tab with LuLu connected and a firmware update ready to install."
          width={1960}
          height={1032}
          preload
          fetchPriority="high"
          sizes="(min-width: 1024px) 33rem, 100vw"
          className="h-auto w-full"
        />
      </div>
      <figure className="absolute bottom-3 left-6 flex items-end gap-3 sm:left-10">
        <Image
          src="/case/assembled.png"
          alt="LuLu, closed."
          width={900}
          height={653}
          sizes="(min-width: 1024px) 8rem, 28vw"
          className="h-auto w-28 drop-shadow-[0_14px_16px_rgba(8,20,48,0.35)] sm:w-32"
        />
        <figcaption className="mb-2 rounded-full border border-border bg-card/90 px-3 py-1 font-mono text-xs text-muted-foreground backdrop-blur">
          LuLu
        </figcaption>
      </figure>
    </div>
  );
}
