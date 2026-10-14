export function Logo({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 128 128" className={className} aria-hidden="true">
      <g transform="translate(64 64) scale(0.8) translate(-64 -64)">
        <rect width="128" height="128" rx="28" fill="#111111" />
        <g transform="translate(64 64) scale(0.64) translate(-52 -67)">
          <path
            d="M26 18 L26 84 L44 66 L58 94 L68 89 L54 61 L78 61 Z"
            fill="#3EE0E8"
            stroke="#071316"
            strokeWidth="4"
            strokeLinejoin="round"
          />
          <g transform="translate(52 98)">
            <polygon
              points="0,-18 3.2,-3.2 18,0 3.2,3.2 0,18 -3.2,3.2 -18,0 -3.2,-3.2"
              fill="#F5B429"
            />
            <polygon
              points="0,-8 1.6,-1.6 8,0 1.6,1.6 0,8 -1.6,1.6 -8,0 -1.6,-1.6"
              fill="#FFF4CC"
            />
          </g>
        </g>
      </g>
    </svg>
  );
}
