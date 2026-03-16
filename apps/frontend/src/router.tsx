import { AppShell } from "@/components/app-shell";
import { isApiErrorWithStatus } from "@/lib/api";
import { homeTabSchema } from "@/lib/home-tabs";
import {
	authQueries,
	catchUpQueries,
	savedQueries,
	searchSearchSchema,
} from "@/lib/queries";
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
import { z } from "zod";

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
			await context.queryClient.ensureQueryData(authQueries.me());
		} catch (error) {
			if (isApiErrorWithStatus(error, 401)) {
				throw redirect({ to: "/sign-in" });
			}
			throw error;
		}
	},
	component: AppShell,
});

const homeSearchSchema = z.object({
	channel: z.string().optional().catch(undefined),
	tab: homeTabSchema.optional().catch(undefined),
});

const indexRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/",
	component: HomePage,
	validateSearch: homeSearchSchema,
	loaderDeps: ({ search }) => ({ channel: search.channel, tab: search.tab }),
	loader: ({ context }) => {
		void context.queryClient.ensureQueryData(catchUpQueries.summary("7d"));
	},
});

const searchRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/search",
	component: SearchPage,
	validateSearch: searchSearchSchema,
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
	loader: ({ context }) => {
		void context.queryClient.ensureQueryData(savedQueries.list());
	},
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
	defaultPreloadStaleTime: 0,
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
