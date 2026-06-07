import { AppShell } from "@/components/app-shell";
import { isApiErrorWithStatus } from "@/lib/api";
import { readCachedCurrentUser } from "@/lib/auth-cache";
import { homeTabSchema, normalizeHomeTab } from "@/lib/home-tabs";
import { canReadPathOffline } from "@/lib/offline-reading";
import {
	authQueries,
	catchUpQueries,
	savedQueries,
	searchSearchSchema,
	starredQueries,
} from "@/lib/queries";
import {
	normalizeThreadListSort,
	threadListSortSchema,
} from "@/lib/thread-list-sort";
import { AccountPage } from "@/routes/account-page";
import { AdminUsersPage } from "@/routes/admin-users-page";
import { HomePage } from "@/routes/home-page";
import { SavedPage } from "@/routes/saved-page";
import { SearchPage } from "@/routes/search-page";
import { SignInPage } from "@/routes/sign-in-page";
import { ThreadPage } from "@/routes/thread-page";
import { TopicsPage } from "@/routes/topics-page";
import type {
	EnsureQueryDataOptions,
	QueryClient,
} from "@tanstack/react-query";
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
	beforeLoad: async ({ context, location }) => {
		try {
			await context.queryClient.ensureQueryData(authQueries.me());
		} catch (error) {
			if (isApiErrorWithStatus(error, 401)) {
				throw redirect({ to: "/sign-in" });
			}

			if (
				canReadPathOffline(location.pathname) &&
				readCachedCurrentUser() !== null
			) {
				return;
			}

			throw error;
		}
	},
	component: AppShell,
});

const homeSearchSchema = z.object({
	channel: z.string().optional().catch(undefined),
	tab: homeTabSchema.optional().catch(undefined),
	sort: threadListSortSchema.optional().catch(undefined),
});

const savedSearchSchema = z.object({
	sort: threadListSortSchema.optional().catch(undefined),
});

const indexRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/",
	component: HomePage,
	validateSearch: homeSearchSchema,
	loaderDeps: ({ search }) => ({
		channel: search.channel,
		tab: search.tab,
		sort: search.sort,
	}),
	loader: ({ context, deps }) => {
		preloadRouteData(context.queryClient, catchUpQueries.summary("7d"));
		preloadActiveHomeTab(context.queryClient, deps);
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
	validateSearch: savedSearchSchema,
	loader: ({ context }) => {
		preloadRouteData(context.queryClient, savedQueries.list());
	},
});

const accountRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/account",
	component: AccountPage,
});

const adminUsersRoute = createRoute({
	getParentRoute: () => appRoute,
	path: "/admin/users",
	component: AdminUsersPage,
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
		adminUsersRoute,
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

function preloadRouteData<
	TQueryFnData = unknown,
	TError = unknown,
	TData = TQueryFnData,
	TQueryKey extends readonly unknown[] = readonly unknown[],
>(
	queryClient: QueryClient,
	query: EnsureQueryDataOptions<TQueryFnData, TError, TData, TQueryKey>,
) {
	void queryClient.ensureQueryData(query).catch(() => undefined);
}

function preloadActiveHomeTab(
	queryClient: QueryClient,
	{
		channel,
		tab,
		sort,
	}: {
		channel?: string;
		tab?: string;
		sort?: string;
	},
) {
	const activeTab = normalizeHomeTab(tab);
	const activeSort = normalizeThreadListSort(sort);
	if (activeTab === "starred") {
		preloadRouteData(queryClient, starredQueries.list({ channelId: channel }));
		return;
	}

	const options =
		activeTab === "fresh"
			? catchUpQueries.feed({
					window: "24h",
					channelId: channel,
					sort: activeSort,
				})
			: catchUpQueries.feed({
					window: "7d",
					channelId: channel,
					sort: activeSort,
				});
	void queryClient.prefetchInfiniteQuery(options).catch(() => undefined);
}
