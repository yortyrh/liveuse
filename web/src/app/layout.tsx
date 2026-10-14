import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import { ThemeProvider } from "@/components/theme-provider";
import { cn } from "@/lib/utils";
import "./globals.css";
import "./ground.css";

const geist = Geist({ subsets: ["latin"], variable: "--font-geist" });
const geistMono = Geist_Mono({ subsets: ["latin"], variable: "--font-geist-mono" });

const description =
  "Download LiveUse for Mac. With a Seeed XIAO running the sketch, it publishes a mouse and keyboard API and MCP on your local network.";

export const metadata: Metadata = {
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
      className={cn(geist.variable, geistMono.variable, "scroll-smooth")}
      suppressHydrationWarning
    >
      <body className="page-ground flex min-h-screen flex-col font-sans text-base text-foreground antialiased">
        <ThemeProvider>{children}</ThemeProvider>
      </body>
    </html>
  );
}
