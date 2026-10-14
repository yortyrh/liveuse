export function XiaoBoard({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 360 220" className={className} role="img" aria-label="Seeed XIAO SAMD21">
      <rect x="48" y="36" width="292" height="148" rx="16" fill="#1d3557" />
      <rect x="64" y="52" width="168" height="116" rx="8" fill="#16304f" />
      <rect x="86" y="78" width="92" height="64" rx="6" fill="#0f2744" />
      <circle cx="214" cy="92" r="5" fill="#3ee0e8" />
      <rect x="18" y="78" width="36" height="64" rx="8" fill="#c5ccd6" />
      <rect x="28" y="90" width="16" height="40" rx="8" fill="#8b95a3" />
      {Array.from({ length: 7 }, (_, index) => (
        <rect key={`top-${index}`} x={78 + index * 28} y="28" width="16" height="18" rx="2" fill="#d6b25e" />
      ))}
      {Array.from({ length: 7 }, (_, index) => (
        <rect key={`bottom-${index}`} x={78 + index * 28} y="174" width="16" height="18" rx="2" fill="#d6b25e" />
      ))}
    </svg>
  );
}
