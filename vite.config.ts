import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: { host: "127.0.0.1", port: 1420, strictPort: true },
  clearScreen: false,
  test: {
    include: ["src/test/**/*.test.{ts,tsx}"],
    environment: "jsdom",
    setupFiles: "./src/test/setup.ts",
    restoreMocks: true,
    clearMocks: true,
    maxWorkers: 2,
  },
});
