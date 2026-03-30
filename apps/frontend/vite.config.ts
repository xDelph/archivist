import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const devProxyTarget =
	process.env.VITE_DEV_PROXY_TARGET ?? "http://127.0.0.1:4000";

export default defineConfig({
	plugins: [react(), tailwindcss()],
	server: {
		proxy: {
			"/api": devProxyTarget,
			"/health": devProxyTarget,
		},
		allowedHosts: ["devwithai-local.arkivist.dev"],
	},
	resolve: {
		alias: {
			"@": path.resolve(import.meta.dirname, "./src"),
		},
	},
	test: {
		environment: "node",
		globals: true,
		include: ["src/**/*.test.ts"],
	},
});
