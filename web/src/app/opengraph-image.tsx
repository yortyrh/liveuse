import { ImageResponse } from "next/og";

export const alt = "LiveUse, the local pointer for an agent on your Mac";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default function OpenGraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          background: "#ffffff",
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          padding: "80px",
        }}
      >
        <div style={{ fontSize: 32, fontWeight: 600, color: "#171717" }}>LiveUse</div>
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
          <div style={{ color: "#000000" }}>The local pointer</div>
          <div style={{ color: "#666666" }}>for an agent on your Mac</div>
        </div>
      </div>
    ),
    { ...size },
  );
}
