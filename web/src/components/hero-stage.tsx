import Image from "next/image";

export function HeroStage() {
  return (
    <div className="relative mx-auto w-full overflow-hidden rounded-2xl border border-border bg-white shadow-xl">
      <Image
        src="/help/device-hero.png"
        alt="LiveUse Device tab with the XIAO connected and a firmware update ready to install."
        width={1960}
        height={1032}
        preload
        fetchPriority="high"
        sizes="(min-width: 1024px) 33rem, 100vw"
        className="h-auto w-full"
      />
      <Image
        src="/xiao-samd21-angle.png"
        alt="Seeed XIAO SAMD21"
        width={531}
        height={437}
        sizes="(min-width: 1024px) 7rem, 21vw"
        className="pointer-events-none absolute top-[54%] left-[64.5%] w-[21%] -translate-x-1/2 -translate-y-1/2 drop-shadow-[2px_8px_6px_rgba(0,0,0,0.16)]"
      />
    </div>
  );
}
