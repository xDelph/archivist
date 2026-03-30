import { AppLaunchScreen } from "@/components/app-launch-screen";
import { EmptyState } from "@/components/empty-state";
import { IdentityAvatar } from "@/components/identity-avatar";
import { InstallBanner } from "@/components/install-banner";
import { OfflineSaveNotification } from "@/components/offline-save-notification";
import { ThemeControls } from "@/components/theme-controls";
import { resolveHomeEntryTarget } from "@/lib/app-entry-loading";
import { useAppWarmup } from "@/lib/app-warmup";
import { canReadPathOffline } from "@/lib/offline-reading";
import { authQueries, catchUpQueries, starredQueries } from "@/lib/queries";
import { useNetworkStatus } from "@/lib/use-network-status";
import { cn } from "@/lib/utils";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { Bookmark, CloudOff, House, Search } from "lucide-react";
import { startTransition, useEffect, useState } from "react";

const navItems = [
	{ to: "/", label: "Catch up", icon: House },
	{ to: "/search", label: "Search", icon: Search },
	{ to: "/saved", label: "Saved", icon: Bookmark },
] as const;

export function AppShell() {
	const pathname = useRouterState({
		select: (state) => state.location.pathname,
	});
	const search = useRouterState({
		select: (state) => state.location.searchStr,
	});
	const { isOnline } = useNetworkStatus();
	const userQuery = useQuery(authQueries.me());
	const user = userQuery.data?.user;
	const isOfflineRestricted = !isOnline && !canReadPathOffline(pathname);
	const homeEntryTarget = resolveHomeEntryTarget(pathname, search);
	const [isInitialArchiveViewReady, setIsInitialArchiveViewReady] = useState(
		() => homeEntryTarget === null,
	);
	const homeEntryStarredQuery = useQuery({
		...starredQueries.list({ channelId: homeEntryTarget?.channelId }),
		enabled: !isInitialArchiveViewReady && homeEntryTarget?.tab === "starred",
	});
	const homeEntryCatchUpQuery = useInfiniteQuery({
		...catchUpQueries.feed({
			window: homeEntryTarget?.window ?? "24h",
			channelId: homeEntryTarget?.channelId,
			sort: homeEntryTarget?.sort,
		}),
		enabled:
			!isInitialArchiveViewReady &&
			homeEntryTarget !== null &&
			homeEntryTarget.tab !== "starred",
	});
	useAppWarmup({ isOnline, pathname, search });

	const isHomeEntryPending =
		homeEntryTarget === null
			? false
			: homeEntryTarget.tab === "starred"
				? homeEntryStarredQuery.isPending
				: homeEntryCatchUpQuery.isPending;

	useEffect(() => {
		if (isInitialArchiveViewReady) {
			return;
		}

		if (homeEntryTarget === null || !isHomeEntryPending) {
			startTransition(() => {
				setIsInitialArchiveViewReady(true);
			});
		}
	}, [homeEntryTarget, isHomeEntryPending, isInitialArchiveViewReady]);

	if (
		!isInitialArchiveViewReady &&
		homeEntryTarget !== null &&
		isHomeEntryPending
	) {
		return <AppLaunchScreen tab={homeEntryTarget.tab} />;
	}

	return (
		<div className="min-h-dvh bg-(--color-bg-deep) pb-20 text-(--color-text-primary)">
			<header className="sticky top-0 z-30 border-b border-(--color-border-subtle) bg-(--color-header-bg) backdrop-blur-xl">
				<div className="mx-auto flex max-w-[1680px] items-center gap-3 px-3 py-2 sm:px-4">
					<Link to="/" className="flex items-center gap-2">
						<img
							src="/icons/proposal-e-spines-tight.svg"
							alt=""
							aria-hidden="true"
							className="size-10 rounded-xl shadow-[var(--shadow-accent)]"
						/>
						<div>
							<p className="text-[1.15rem] font-semibold leading-none tracking-tight text-(--color-text-bright) sm:text-[1.28rem]">
								Arkivist
							</p>
						</div>
					</Link>

					<nav className="hidden items-center rounded-xl border border-(--color-border-subtle) bg-(--surface-ghost-bg) p-1 sm:flex">
						{navItems.map((item) => {
							const isActive = isNavItemActive(pathname, item.to);
							const isDisabled = !isOnline && item.to !== "/saved";
							const className = cn(
								"rounded-lg border border-transparent px-3.5 py-2 text-[0.84rem] font-medium transition-[background-color,border-color,color,box-shadow]",
								isActive
									? "border-(--color-border-accent) bg-(--color-bg-surface) text-(--color-text-primary) shadow-[var(--shadow-panel-soft)]"
									: "text-(--color-text-secondary)",
								isDisabled
									? "cursor-not-allowed opacity-45"
									: "hover-surface-subtle",
							);

							return isDisabled ? (
								<span key={item.to} aria-disabled="true" className={className}>
									{item.label}
								</span>
							) : (
								<Link key={item.to} to={item.to} className={className}>
									{item.label}
								</Link>
							);
						})}
					</nav>

					<div className="ml-auto flex items-center gap-2 sm:gap-2.5">
						<ThemeControls />
						<Link
							to="/account"
							className="button-ghost flex size-11 items-center justify-center rounded-full p-1 transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent-soft)/40 sm:size-10"
						>
							<IdentityAvatar
								author={
									user
										? {
												slack_user_id: user.slack_user_id,
												display_name: user.display_name,
												avatar_url: user.avatar_url,
											}
										: null
								}
								fallback={user?.email || "Arkivist"}
								size="sm"
							/>
						</Link>
					</div>
				</div>
			</header>

			<div className="mx-auto flex w-full max-w-[1680px] flex-col px-3 py-3 sm:px-4 sm:py-4">
				<InstallBanner />
				<OfflineSaveNotification />
				{!isOnline ? (
					<div className="mb-3 rounded-[1rem] border border-(--color-border-accent) bg-(--color-accent)/10 px-3 py-2.5 text-[0.82rem] text-(--color-text-secondary)">
						<div className="flex items-start gap-2">
							<CloudOff className="mt-0.5 size-4 shrink-0 text-(--color-accent-soft)" />
							<p>
								Offline reading mode is active. Arkivist keeps Saved and
								downloaded thread detail available until the network comes back.
							</p>
						</div>
					</div>
				) : null}
				<main className="flex-1">
					{isOfflineRestricted ? <OfflineRestrictedState /> : <Outlet />}
				</main>
			</div>

			<nav className="fixed inset-x-0 bottom-0 z-30 border-t border-(--color-border-subtle) bg-(--color-mobile-nav-bg) px-2 pb-[env(safe-area-inset-bottom)] backdrop-blur-xl sm:hidden">
				<div className="mx-auto grid max-w-md grid-cols-3 gap-1 py-1.5">
					{navItems.map((item) => {
						const Icon = item.icon;
						const isActive = isNavItemActive(pathname, item.to);
						const isDisabled = !isOnline && item.to !== "/saved";
						const className = cn(
							"flex min-h-12 flex-col items-center justify-center gap-1 rounded-xl px-1 py-2 text-[0.74rem] font-medium transition-[background-color,color] active:translate-y-px",
							isActive
								? "bg-(--color-accent)/14 text-(--color-accent-strong)"
								: "text-(--color-text-muted)",
							isDisabled
								? "cursor-not-allowed opacity-45"
								: "hover:bg-(--surface-ghost-hover-bg) hover:text-(--color-text-primary)",
						);

						return isDisabled ? (
							<span key={item.to} aria-disabled="true" className={className}>
								<Icon className="size-4.5" />
								<span>{item.label}</span>
							</span>
						) : (
							<Link key={item.to} to={item.to} className={className}>
								<Icon className="size-4.5" />
								<span>{item.label}</span>
							</Link>
						);
					})}
				</div>
			</nav>
		</div>
	);
}

function isNavItemActive(pathname: string, to: string) {
	return to === "/"
		? pathname === to
		: pathname === to || pathname.startsWith(`${to}/`);
}

function OfflineRestrictedState() {
	return (
		<EmptyState
			title="Offline reading is limited to saved threads"
			description="Catch-up and search will come back automatically once the network is available again. Open Saved to continue reading."
			icon={<CloudOff className="size-5" />}
		/>
	);
}
