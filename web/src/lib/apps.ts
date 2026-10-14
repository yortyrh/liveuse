import { cacheLife } from "next/cache";

const APPS_URL =
  "https://raw.githubusercontent.com/yortyrh/liveuse-releases/main/apps.json";
const DOWNLOAD_PREFIX =
  "https://github.com/yortyrh/liveuse-releases/releases/download/";
const VERSION_RE = /^\d+\.\d+\.\d+$/;
const SHA256_RE = /^[a-f0-9]{64}$/;

export type Download = {
  id: string;
  os: "macos";
  arch: "x64" | "aarch64";
  name: string;
  file: string;
  url: string;
  sha256: string;
  size: number;
};

export type AppsManifest = {
  schema: 1;
  version: string;
  tag: string;
  signed: boolean;
  downloads: Download[];
};

function isDownload(value: unknown, tag: string): value is Download {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const item = value as Record<string, unknown>;
  const file = item.file;
  return (
    typeof item.id === "string" &&
    item.id.length > 0 &&
    item.os === "macos" &&
    (item.arch === "x64" || item.arch === "aarch64") &&
    typeof item.name === "string" &&
    item.name.trim().length > 0 &&
    typeof file === "string" &&
    file.length > 0 &&
    !file.includes("/") &&
    item.url === `${DOWNLOAD_PREFIX}${tag}/${file}` &&
    typeof item.sha256 === "string" &&
    SHA256_RE.test(item.sha256) &&
    typeof item.size === "number" &&
    Number.isInteger(item.size) &&
    item.size > 0
  );
}

function parseApps(value: unknown): AppsManifest | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const body = value as Record<string, unknown>;
  const version = body.version;
  const tag = body.tag;
  if (body.schema !== 1 || typeof version !== "string" || !VERSION_RE.test(version)) {
    return null;
  }
  if (tag !== `app-v${version}` || typeof body.signed !== "boolean") {
    return null;
  }
  if (!Array.isArray(body.downloads) || body.downloads.length === 0) {
    return null;
  }
  const downloads: Download[] = [];
  for (const item of body.downloads) {
    if (!isDownload(item, tag)) {
      return null;
    }
    downloads.push(item);
  }
  return {
    schema: 1,
    version,
    tag,
    signed: body.signed,
    downloads,
  };
}

export async function getApps(): Promise<AppsManifest | null> {
  "use cache";
  cacheLife("hours");

  try {
    const response = await fetch(APPS_URL);
    if (!response.ok) {
      return null;
    }
    return parseApps(await response.json());
  } catch {
    return null;
  }
}

export function formatSize(size: number): string {
  const megabytes = size / (1024 * 1024);
  const formatted = new Intl.NumberFormat("en", {
    maximumFractionDigits: 1,
    minimumFractionDigits: megabytes < 10 ? 1 : 0,
  }).format(megabytes);
  return `${formatted} MB`;
}

export function releaseUrl(tag: string): string {
  return `https://github.com/yortyrh/liveuse-releases/releases/tag/${tag}`;
}
