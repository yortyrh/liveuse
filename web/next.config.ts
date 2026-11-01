import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  output: "standalone",
  cacheComponents: true,
  partialPrefetching: true,
  experimental: {
    agentUpgrade: "security",
  },
  images: {
    // Sources are at most 1960px wide, so 3840 duplicates the 2048 candidate.
    deviceSizes: [640, 750, 828, 1080, 1200, 1920, 2048],
  },
  turbopack: {
    rules: {
      "*.css": {
        loaders: ["@tailwindcss/turbopack"],
        as: "*.css",
      },
    },
  },
};

export default nextConfig;
