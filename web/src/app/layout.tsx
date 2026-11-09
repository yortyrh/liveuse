import type { Metadata } from "next";
import { Archivo, Martian_Mono } from "next/font/google";
import { ThemeProvider } from "@/components/theme-provider";
import { cn } from "@/lib/utils";
import "./globals.css";

const archivo = Archivo({ subsets: ["latin"], axes: ["wdth"], variable: "--font-archivo" });
const martian = Martian_Mono({ subsets: ["latin"], axes: ["wdth"], variable: "--font-martian" });

const description =
  "Download LiveUse for Mac. With LuLu plugged in, it publishes a mouse and keyboard API and MCP on your local network.";

// Open Graph and Twitter image URLs resolve against this origin. SITE_URL overrides it; without
// it, production builds use the public site so shared links never point at localhost.
const siteUrl =
  process.env.SITE_URL ??
  (process.env.NODE_ENV === "production" ? "https://liveuse.sitewebr.com" : "http://localhost:3000");

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: {
    default: "LiveUse for Mac",
    template: "%s · LiveUse",
  },
  description,
  robots: { index: true, follow: true },
  openGraph: {
    title: "LiveUse for Mac",
    description,
    type: "website",
    locale: "en_US",
  },
  twitter: {
    card: "summary_large_image",
    title: "LiveUse for Mac",
    description,
  },
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="en"
      className={cn(archivo.variable, martian.variable, "scroll-smooth")}
      suppressHydrationWarning
    >
      <body className="flex min-h-screen flex-col bg-background font-sans text-base text-foreground antialiased">
        <ThemeProvider>{children}</ThemeProvider>
      </body>
    </html>
  );
}
