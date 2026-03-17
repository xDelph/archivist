import "@fontsource/space-grotesk/500.css";
import "@fontsource/space-grotesk/700.css";
import "./styles.css";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { registerAppServiceWorker } from "./lib/pwa";
import { ThemeProvider } from "./lib/theme";
import { router } from "./router";

const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			retry: false,
			staleTime: 60_000,
		},
	},
});

const rootElement = document.getElementById("root");

if (!rootElement) {
	throw new Error("Missing #root element");
}

void registerAppServiceWorker();

createRoot(rootElement).render(
	<StrictMode>
		<ThemeProvider>
			<QueryClientProvider client={queryClient}>
				<RouterProvider router={router} context={{ queryClient }} />
			</QueryClientProvider>
		</ThemeProvider>
	</StrictMode>,
);
