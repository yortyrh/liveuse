import Image from "next/image";

export function XiaoBoard({ className }: { className?: string }) {
  return (
    <Image
      src="/xiao-samd21-angle.png"
      alt="Seeed XIAO SAMD21"
      width={531}
      height={437}
      sizes="280px"
      className={className}
    />
  );
}
