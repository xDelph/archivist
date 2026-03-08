import "@fontsource/space-grotesk/500.css";
import "@fontsource/space-grotesk/700.css";
import "./styles.css";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
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

createRoot(rootElement).render(
	<StrictMode>
		<QueryClientProvider client={queryClient}>
			<RouterProvider router={router} />
		</QueryClientProvider>
	</StrictMode>,
);
