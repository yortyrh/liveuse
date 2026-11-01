import { CopyHash } from "@/components/copy-hash";
import { PrimaryLink, TextLink } from "@/components/primary-link";
import { RELEASES_REPO } from "@/lib/links";
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
      <div className="grid gap-12 md:grid-cols-2">
        {downloads.map((item) => (
          <DownloadCard key={item.id} item={item} version={apps.version} />
        ))}
      </div>
      {apps.signed ? null : (
        <p className="max-w-[640px] border-l-2 border-spark pl-4 text-base text-pretty">
          This build is not signed or notarized. The first time, right click LiveUse and choose
          Open.
        </p>
      )}
    </div>
  );
}

export function DownloadSkeleton() {
  return (
    <div aria-busy="true" aria-live="polite" className="grid gap-12 md:grid-cols-2">
      <p className="sr-only">Loading the installers.</p>
      <SkeletonCard />
      <SkeletonCard />
    </div>
  );
}

function DownloadsUnavailable() {
  return (
    <div role="alert" className="flex max-w-[640px] flex-col gap-4 border-t-2 border-destructive pt-5">
      <h3 className="wide text-xl font-semibold">The installer list did not load</h3>
      <p className="text-pretty text-base text-muted-foreground">
        GitHub did not answer for the release list. Reload this page, or open the releases on GitHub.
      </p>
      <div className="flex flex-wrap items-center gap-6">
        <PrimaryLink href="/">Reload</PrimaryLink>
        <TextLink href={`${RELEASES_REPO}/releases`}>Open the releases on GitHub</TextLink>
      </div>
    </div>
  );
}

function DownloadCard({ item, version }: { item: Download; version: string }) {
  const label = item.arch === "x64" ? "Download for Intel" : "Download for Apple Silicon";
  const chip = item.arch === "x64" ? "About This Mac says Processor" : "About This Mac says Chip";

  return (
    <article className="flex h-full flex-col gap-5 border-t-2 border-foreground pt-5">
      <div className="flex flex-col gap-1">
        <h3 className="wide text-2xl font-semibold">{item.name}</h3>
        <p className="text-sm text-muted-foreground">{chip}</p>
      </div>
      <dl className="divide-y divide-border border-y border-border text-sm">
        <div className="grid grid-cols-[5rem_minmax(0,1fr)] gap-3 py-2">
          <dt className="text-muted-foreground">File</dt>
          <dd className="font-mono text-xs leading-5 [overflow-wrap:anywhere]">{item.file}</dd>
        </div>
        <div className="grid grid-cols-[5rem_minmax(0,1fr)] gap-3 py-2">
          <dt className="text-muted-foreground">Size</dt>
          <dd>
            {formatSize(item.size)}, LiveUse {version}
          </dd>
        </div>
        <div className="grid grid-cols-[5rem_minmax(0,1fr)] gap-3 py-2">
          <dt className="text-muted-foreground">SHA256</dt>
          <dd className="font-mono text-xs leading-5 break-all text-muted-foreground">
            {item.sha256}
          </dd>
        </div>
      </dl>
      <div className="mt-auto grid gap-3 sm:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
        <PrimaryLink href={item.url} className="w-full">
          {label}
        </PrimaryLink>
        <CopyHash value={item.sha256} />
      </div>
    </article>
  );
}

function SkeletonCard() {
  return (
    <div className="flex flex-col gap-4 border-t-2 border-border pt-5">
      <div className="h-7 w-48 animate-pulse rounded bg-muted" />
      <div className="h-4 w-40 animate-pulse rounded bg-muted" />
      <div className="h-28 w-full animate-pulse rounded bg-muted" />
      <div className="h-12 w-full animate-pulse rounded bg-muted" />
    </div>
  );
}

const archOrder = { aarch64: 0, x64: 1 } as const;
