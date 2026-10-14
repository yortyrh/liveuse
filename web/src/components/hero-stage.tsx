import Image from "next/image";
import { XiaoBoard } from "@/components/xiao-board";

export function HeroStage() {
  return (
    <div className="relative mx-auto flex w-full max-w-xl items-center">
      <div className="relative z-10 w-[42%] -rotate-6 rounded-2xl bg-white p-3 shadow-xl">
        <XiaoBoard className="h-auto w-full" />
      </div>
      <div className="relative z-0 -ml-8 w-[72%] rotate-3 rounded-2xl border border-border bg-white shadow-xl">
        <Image
          src="/help/control-opus.png"
          alt="LiveUse Control tab with Anthropic and Claude Opus 5.5 selected."
          width={1960}
          height={470}
          priority
          className="h-auto w-full rounded-2xl"
        />
      </div>
    </div>
  );
}
