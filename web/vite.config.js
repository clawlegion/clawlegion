import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
export default defineConfig({
    plugins: [react()],
    server: {
        port: 5173,
        host: "0.0.0.0",
    },
    test: {
        environment: "jsdom",
        setupFiles: "./src/test/setup.ts",
        globals: true,
        exclude: ["tests/e2e/**", "node_modules/**"],
    },
});
