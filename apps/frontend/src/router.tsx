import { HomePage } from "@/routes/home-page";
import {
	Outlet,
	createRootRoute,
	createRoute,
	createRouter,
} from "@tanstack/react-router";

const rootRoute = createRootRoute({
	component: RootLayout,
});

const indexRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/",
	component: HomePage,
});

const routeTree = rootRoute.addChildren([indexRoute]);

export const router = createRouter({
	routeTree,
	defaultPreload: "intent",
});

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
}

function RootLayout() {
	return <Outlet />;
}
