import type { Metadata } from "next";
import { Inter, JetBrains_Mono, Source_Serif_4 } from "next/font/google";
import { ThemeProvider } from "@/components/theme-provider";
import { cn } from "@/lib/utils";
import "./globals.css";

const inter = Inter({ subsets: ["latin"], variable: "--font-inter" });

const sourceSerif = Source_Serif_4({
  subsets: ["latin"],
  variable: "--font-source-serif-4",
});

const jetbrains = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-jetbrains-mono",
});

export const metadata: Metadata = {
  title: "LiveUse",
  description:
    "App de macOS que, con el XIAO corriendo el sketch, publica en la red local una API y un MCP del mouse y el teclado.",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="es"
      className={cn(inter.variable, sourceSerif.variable, jetbrains.variable)}
      suppressHydrationWarning
    >
      <body className="min-h-full antialiased">
        <ThemeProvider>{children}</ThemeProvider>
      </body>
    </html>
  );
}
