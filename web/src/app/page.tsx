import { CopyHash } from "@/components/copy-hash";
import { ThemeToggle } from "@/components/theme-toggle";
import { Badge } from "@/components/ui/badge";
import { buttonVariants } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { formatSize, getApps, releaseUrl, type Download } from "@/lib/apps";

export default async function Home() {
  const apps = await getApps();

  return (
    <>
      <header className="border-b">
        <div className="mx-auto flex h-14 w-full max-w-3xl items-center justify-between px-6">
          <p className="font-heading text-sm font-semibold tracking-tight">LiveUse</p>
          <ThemeToggle />
        </div>
      </header>
      <main className="mx-auto flex w-full max-w-3xl flex-col gap-12 px-6 py-14">
        <section className="flex flex-col gap-4">
          <h1>LiveUse</h1>
          <p className="max-w-2xl text-lg">
            LiveUse es la app de macOS para el Seeed XIAO. Con el sketch corriendo, publica en la
            red local una API y un MCP del mouse y del teclado. Cerrar la ventana la deja en la
            barra de menú. Quit, en ese menú, cierra la app.
          </p>
          <p className="max-w-2xl text-muted-foreground">
            Quien tenga el token en esa red puede mover el cursor y el teclado.
          </p>
        </section>

        <Separator />

        <section id="descargar" className="flex flex-col gap-6">
          <div className="flex flex-col gap-3">
            <h2>Descargar</h2>
            <p>
              Hay un instalador para Mac Intel y otro para Apple Silicon. En el menú Apple, abre
              Acerca de este Mac. Si dice Chip, baja Apple Silicon. Si dice Procesador, baja Intel.
            </p>
          </div>
          {apps ? <DownloadList apps={apps} /> : <DownloadsUnavailable />}
        </section>

        <Separator />

        <section className="flex flex-col gap-4">
          <h2>Instalación</h2>
          <ol className="flex list-decimal flex-col gap-2 pl-5">
            <li>Abre el DMG.</li>
            <li>Arrastra LiveUse a la carpeta Aplicaciones.</li>
            {apps?.signed === false ? (
              <li>
                La primera vez, clic derecho sobre LiveUse y elige Abrir. Esta versión no está
                firmada ni notarizada, y macOS puede bloquearla.
              </li>
            ) : null}
          </ol>
        </section>

        <Separator />

        <section className="flex flex-col gap-4">
          <h2>Qué pide macOS</h2>
          <ul className="flex list-disc flex-col gap-2 pl-5">
            <li>macOS.</li>
            <li>Permiso de grabación de pantalla, para las capturas del agente.</li>
            <li>Permiso de conexiones entrantes, para la API en la red local.</li>
            <li>
              Un Seeed XIAO SAMD21 con el sketch corriendo. El bootloader no inicia el servicio.
            </li>
          </ul>
        </section>
      </main>
      <footer className="border-t">
        <div className="mx-auto flex w-full max-w-3xl px-6 py-6 text-sm text-muted-foreground">
          <a href="https://github.com/yortyrh/liveuse-releases">Versiones públicas</a>
        </div>
      </footer>
    </>
  );
}

function DownloadsUnavailable() {
  return (
    <p role="status" className="text-muted-foreground">
      La descarga no está disponible. El manifiesto público no respondió.
    </p>
  );
}

function DownloadList({
  apps,
}: {
  apps: {
    version: string;
    tag: string;
    downloads: Download[];
  };
}) {
  return (
    <div className="flex flex-col gap-4">
      <p className="text-sm text-muted-foreground">
        Versión {apps.version}.{" "}
        <a href={releaseUrl(apps.tag)} className="underline-offset-4 hover:underline">
          Notas de la versión
        </a>
      </p>
      <div className="grid gap-4 sm:grid-cols-2">
        {apps.downloads.map((item) => (
          <DownloadCard key={item.id} item={item} version={apps.version} />
        ))}
      </div>
    </div>
  );
}

function DownloadCard({ item, version }: { item: Download; version: string }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{item.name}</CardTitle>
        <CardDescription>LiveUse {version}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <Badge variant="outline">{item.file}</Badge>
        <p className="text-sm text-muted-foreground">{formatSize(item.size)}</p>
        <p className="break-all font-mono text-xs text-muted-foreground">{item.sha256}</p>
        <CopyHash value={item.sha256} />
      </CardContent>
      <CardFooter>
        <a href={item.url} className={buttonVariants({ size: "lg" })}>
          Descargar
        </a>
      </CardFooter>
    </Card>
  );
}
