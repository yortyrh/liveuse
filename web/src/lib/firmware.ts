import { cacheLife } from "next/cache";

const MANIFEST_URL =
  "https://raw.githubusercontent.com/yortyrh/liveuse-releases/main/manifest.json";
const DOWNLOAD_PREFIX =
  "https://github.com/yortyrh/liveuse-releases/releases/download/";
const VERSION_RE = /^\d+\.\d+\.\d+$/;
const SHA256_RE = /^[a-f0-9]{64}$/;

export type FirmwareImage = {
  id: string;
  name: string;
  file: string;
  url: string;
  sha256: string;
  size: number;
  offset: string;
};

export type FirmwareManifest = {
  version: string;
  tag: string;
  board: FirmwareImage;
};

function parseManifest(value: unknown): FirmwareManifest | null {
  if (typeof value !== "object" || value === null) return null;
  const body = value as Record<string, unknown>;
  const version = body.version;
  const tag = body.tag;
  if (body.schema !== 1 || typeof version !== "string" || !VERSION_RE.test(version)) return null;
  if (tag !== `firmware-v${version}` || !Array.isArray(body.boards)) return null;

  for (const entry of body.boards) {
    if (typeof entry !== "object" || entry === null) continue;
    const item = entry as Record<string, unknown>;
    const flash = item.flash as Record<string, unknown> | undefined;
    const file = item.file;
    if (
      item.id === "seeed_xiao" &&
      typeof item.name === "string" &&
      typeof file === "string" &&
      !file.includes("/") &&
      item.url === `${DOWNLOAD_PREFIX}${tag}/${file}` &&
      typeof item.sha256 === "string" &&
      SHA256_RE.test(item.sha256) &&
      typeof item.size === "number" &&
      Number.isInteger(item.size) &&
      item.size > 0 &&
      typeof flash?.offset === "string"
    ) {
      return {
        version,
        tag,
        board: {
          id: item.id,
          name: item.name,
          file,
          url: item.url,
          sha256: item.sha256,
          size: item.size,
          offset: flash.offset,
        },
      };
    }
  }
  return null;
}

export async function getFirmware(): Promise<FirmwareManifest | null> {
  "use cache";
  cacheLife("hours");

  try {
    const response = await fetch(MANIFEST_URL);
    if (!response.ok) return null;
    return parseManifest(await response.json());
  } catch {
    return null;
  }
}

export function formatBytes(size: number): string {
  return `${new Intl.NumberFormat("en", { maximumFractionDigits: 1 }).format(size / 1024)} KB`;
}
