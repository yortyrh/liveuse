import { CopyHash } from "@/components/copy-hash";
import { PrimaryLink, TextLink } from "@/components/primary-link";
import { formatSize, getApps, releaseUrl, type Download } from "@/lib/apps";

export async function Downloads() {
  const apps = await getApps();
  if (!apps) return <DownloadsUnavailable />;

  const downloads = [...apps.downloads].sort((a, b) => archOrder[a.arch] - archOrder[b.arch]);

  return (
    <div className="flex flex-col gap-6">
      <p className="text-sm text-muted-foreground">
        Version {apps.version}. <TextLink href={releaseUrl(apps.tag)}>Release notes</TextLink>
      </p>
      <div className="grid gap-6 sm:grid-cols-2">
        {downloads.map((item) => (
          <DownloadCard key={item.id} item={item} version={apps.version} />
        ))}
      </div>
      {apps.signed ? null : (
        <p className="max-w-[680px] text-pretty text-base">
          This build is not signed or notarized. The first time, right click LiveUse and choose
          Open.
        </p>
      )}
    </div>
  );
}

export function DownloadSkeleton() {
  return (
    <div aria-busy="true" aria-live="polite" className="grid gap-6 sm:grid-cols-2">
      <p className="sr-only">Loading the installers.</p>
      <SkeletonCard />
      <SkeletonCard />
    </div>
  );
}

function DownloadsUnavailable() {
  return (
    <div role="alert" className="flex max-w-[680px] flex-col gap-4 rounded-2xl border border-border p-6">
      <h3 className="text-xl font-semibold">The download is unavailable</h3>
      <p className="text-pretty text-base text-muted-foreground">
        The public release list did not respond. Reload this page in a moment.
      </p>
      <PrimaryLink href="/">Reload</PrimaryLink>
    </div>
  );
}

function DownloadCard({ item, version }: { item: Download; version: string }) {
  const label = item.arch === "x64" ? "Download for Intel" : "Download for Apple Silicon";

  return (
    <article className="flex h-full flex-col gap-4 rounded-2xl border border-border bg-card p-6 transition-all duration-700 ease-[cubic-bezier(0.32,0.72,0,1)] hover:-translate-y-1">
      <div className="flex flex-col gap-2">
        <h3 className="text-xl font-semibold">{item.name}</h3>
        <p className="text-sm text-muted-foreground">LiveUse {version}</p>
      </div>
      <p className="font-mono text-sm">{item.file}</p>
      <p className="text-sm text-muted-foreground">{formatSize(item.size)}</p>
      <PrimaryLink href={item.url} className="w-full">
        {label}
      </PrimaryLink>
      <div className="flex flex-col gap-2 rounded-lg bg-muted p-3">
        <p className="text-xs font-semibold">SHA256 checksum</p>
        <p className="font-mono text-xs break-all text-muted-foreground">{item.sha256}</p>
      </div>
      <CopyHash value={item.sha256} />
    </article>
  );
}

function SkeletonCard() {
  return (
    <div className="flex flex-col gap-4 rounded-2xl border border-border p-6">
      <div className="h-6 w-40 animate-pulse rounded-lg bg-muted" />
      <div className="h-4 w-24 animate-pulse rounded-lg bg-muted" />
      <div className="h-4 w-full animate-pulse rounded-lg bg-muted" />
      <div className="h-10 w-full animate-pulse rounded-lg bg-muted" />
      <div className="h-16 w-full animate-pulse rounded-lg bg-muted" />
    </div>
  );
}

const archOrder = { x64: 0, aarch64: 1 } as const;
