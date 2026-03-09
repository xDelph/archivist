import { AppShell } from "@/components/app-shell";
import { currentUserQueryOptions, isUnauthorizedError } from "@/lib/auth";
import { AccountPage } from "@/routes/account-page";
import { HomePage } from "@/routes/home-page";
import { SavedPage } from "@/routes/saved-page";
import { SearchPage } from "@/routes/search-page";
import { SignInPage } from "@/routes/sign-in-page";
import { ThreadPage } from "@/routes/thread-page";
import { TopicsPage } from "@/routes/topics-page";
import type { QueryClient } from "@tanstack/react-query";
import {
	Outlet,
	createRootRouteWithContext,
	createRoute,
	createRouter,
	redirect,
} from "@tanstack/react-router";

interface RouterContext {
	queryClient: QueryClient;
}

const rootRoute = createRootRouteWithContext<RouterContext>()({
	component: RootLayout,
});

const signInRoute = createRoute({
	getParentRoute: () => rootRoute,
	path: "/sign-in",
	component: SignInPage,
});

const appRoute = createRoute({
	getParentRoute: () => rootRoute,
	id: "app",
	beforeLoad: async ({ context }) => {
		try {
			await context.queryClient.ensureQueryData(currentUserQueryOptions());
		} catch (error) {
			if (isUnauthorizedError(error)) {
				throw redirect({ to: "/sign-in" });
			}

			throw error;
		}
	},
	component: AppShell,
});

const indexRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/",
	component: HomePage,
});

const searchRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/search",
	component: SearchPage,
});

const topicsRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/topics",
	component: TopicsPage,
});

const savedRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/saved",
	component: SavedPage,
});

const accountRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/account",
	component: AccountPage,
});

const threadRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/threads/$threadId",
	component: ThreadPage,
});

const routeTree = rootRoute.addChildren([
	signInRoute,
	appRoute.addChildren([
		indexRoute,
		searchRoute,
		topicsRoute,
		savedRoute,
		accountRoute,
		threadRoute,
	]),
]);

export const router = createRouter({
	routeTree,
	defaultPreload: "intent",
	context: {
		queryClient: undefined as never,
	},
});

declare module "@tanstack/react-router" {
	interface Register {
		router: typeof router;
	}
}

function RootLayout() {
	return <Outlet />;
}
