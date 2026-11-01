import { ImageResponse } from "next/og";

export const alt = "LiveUse, the local pointer for an agent on your Mac";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default function OpenGraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          background: "linear-gradient(135deg, #0e1730 0%, #1b2b55 100%)",
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          padding: "80px",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
          <div style={{ width: 90, height: 8, background: "#4f8ff0" }} />
          <div style={{ fontSize: 32, fontWeight: 600, color: "#8ab8f5" }}>LiveUse</div>
        </div>
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            fontSize: 72,
            fontWeight: 600,
            lineHeight: 1.1,
            letterSpacing: "-1px",
          }}
        >
          <div style={{ color: "#ffffff" }}>The local pointer</div>
          <div style={{ color: "#4f8ff0" }}>for an agent on your Mac</div>
          <div style={{ marginTop: 32, fontSize: 28, fontWeight: 400, color: "#9fb0cf" }}>
            macOS app · LuLu, the USB device · firmware
          </div>
        </div>
      </div>
    ),
    { ...size },
  );
}
