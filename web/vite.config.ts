import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  base: process.env.VITE_BASE_PATH ?? "/",
  plugins: [react(), tailwindcss()],
  build: {
    rollupOptions: {
      output: {
        manualChunks(moduleId) {
          if (!moduleId.includes("node_modules")) {
            return undefined;
          }
          if (moduleId.includes("/react/") || moduleId.includes("/react-dom/") || moduleId.includes("/react-router/")) {
            return "react-vendor";
          }
          if (moduleId.includes("/@tanstack/")) {
            return "query-vendor";
          }
          if (
            moduleId.includes("/@radix-ui/")
            || moduleId.includes("/cmdk/")
            || moduleId.includes("/lucide-react/")
            || moduleId.includes("/sonner/")
          ) {
            return "ui-vendor";
          }
          if (moduleId.includes("/@noble/")) {
            return "crypto-vendor";
          }
          return undefined;
        },
      },
    },
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    proxy: {
      "/api": {
        target: "http://localhost:8080",
        changeOrigin: true,
      },
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    restoreMocks: true,
    css: true,
  },
});
