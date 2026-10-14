import Image from "next/image";

export function XiaoBoard({ className }: { className?: string }) {
  return (
    <Image
      src="/xiao-samd21.png"
      alt="Seeed XIAO SAMD21"
      width={398}
      height={488}
      className={className}
    />
  );
}
