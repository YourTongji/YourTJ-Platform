import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  base: process.env.VITE_BASE_PATH ?? "/",
  plugins: [react(), tailwindcss()],
  build: {
    // The CodeMirror core is a lazy editor-only chunk; keep the warning focused on larger regressions.
    chunkSizeWarningLimit: 650,
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
          if (moduleId.includes("/ali-oss/")) {
            return "oss-vendor";
          }
          if (moduleId.includes("/@uiw/react-codemirror/")) {
            return "codemirror-react-vendor";
          }
          if (
            moduleId.includes("/@codemirror/lang-markdown/")
            || moduleId.includes("/@lezer/markdown/")
          ) {
            return "codemirror-language-vendor";
          }
          if (moduleId.includes("/@codemirror/") || moduleId.includes("/@lezer/")) {
            return "codemirror-core-vendor";
          }
          if (
            moduleId.includes("/react-markdown/")
            || moduleId.includes("/remark-")
            || moduleId.includes("/rehype-")
            || moduleId.includes("/unified/")
          ) {
            return "markdown-renderer-vendor";
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
