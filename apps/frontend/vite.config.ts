import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
	plugins: [react(), tailwindcss()],
	server: {
		proxy: {
			"/api": "http://127.0.0.1:4000",
			"/health": "http://127.0.0.1:4000",
		},
	},
	resolve: {
		alias: {
			"@": path.resolve(__dirname, "./src"),
		},
	},
	test: {
		environment: "node",
		globals: true,
		include: ["src/**/*.test.ts"],
	},
});
