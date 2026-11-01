import { ImageResponse } from "next/og";

export const alt = "LiveUse: give your agent a real mouse and keyboard";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default function OpenGraphImage() {
  return new ImageResponse(
    (
      <div
        style={{
          background: "#15181d",
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          padding: "80px",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
          <div style={{ width: 90, height: 8, background: "#3ee0e8" }} />
          <div style={{ fontSize: 32, fontWeight: 600, color: "#e9ecf0" }}>LiveUse</div>
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
          <div style={{ color: "#e9ecf0" }}>Give your agent a real</div>
          <div style={{ color: "#e9ecf0" }}>mouse and keyboard</div>
          <div style={{ marginTop: 32, fontSize: 28, fontWeight: 400, color: "#a9b0bc" }}>
            LiveUse for Mac and LuLu, the USB device
          </div>
        </div>
      </div>
    ),
    { ...size },
  );
}
