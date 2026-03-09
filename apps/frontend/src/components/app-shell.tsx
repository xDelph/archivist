import { InstallBanner } from "@/components/install-banner";
import { initials } from "@/lib/format";
import { authQueries } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { Bookmark, House, Layers3, Search, UserRound } from "lucide-react";

const navItems = [
	{ to: "/", label: "Home", icon: House },
	{ to: "/search", label: "Search", icon: Search },
	{ to: "/topics", label: "Topics", icon: Layers3 },
	{ to: "/saved", label: "Saved", icon: Bookmark },
	{ to: "/account", label: "Account", icon: UserRound },
] as const;

export function AppShell() {
	const pathname = useRouterState({
		select: (state) => state.location.pathname,
	});
	const userQuery = useQuery(authQueries.me());
	const user = userQuery.data?.user;

	return (
		<div className="relative min-h-dvh pb-[5.5rem]">
			<div className="mx-auto flex min-h-dvh w-full max-w-5xl flex-col px-4 pb-6 pt-5 sm:px-6">
				<header className="sticky top-0 z-20 -mx-4 mb-6 border-b border-(--color-border-subtle) bg-(--color-bg-deep)/85 px-4 py-4 backdrop-blur-lg sm:-mx-0 sm:rounded-(--radius-section) sm:border sm:bg-(--color-bg-base)/75 sm:px-6">
					<div className="flex items-center justify-between gap-4">
						<Link to="/" className="group">
							<p className="text-[0.62rem] font-medium uppercase tracking-[0.36em] text-(--color-accent-soft)">
								Archivist
							</p>
							<h1 className="mt-1.5 text-xl font-semibold text-(--color-text-primary) sm:text-2xl">
								Public-channel catch-up
							</h1>
						</Link>
						<Link
							to="/account"
							className="flex items-center gap-3 rounded-(--radius-pill) border border-(--color-border-subtle) bg-(--color-bg-surface) px-3 py-2 transition-colors hover:border-(--color-border-accent)"
						>
							{user?.avatar_url ? (
								<img
									src={user.avatar_url}
									alt=""
									className="size-9 rounded-full object-cover"
								/>
							) : (
								<span className="flex size-9 items-center justify-center rounded-full bg-(--color-accent-soft)/15 text-sm font-semibold text-(--color-accent-soft)">
									{initials(user?.display_name ?? user?.email)}
								</span>
							)}
							<span className="hidden text-right sm:block">
								<span className="block text-sm font-medium text-(--color-text-primary)">
									{user?.display_name || "Member"}
								</span>
								<span className="block text-xs text-(--color-text-muted)">
									{user?.email || user?.slack_user_id || "Slack"}
								</span>
							</span>
						</Link>
					</div>
				</header>

				<InstallBanner />
				<main className="flex-1">
					<Outlet />
				</main>
			</div>

			<nav className="fixed inset-x-0 bottom-0 z-30 border-t border-(--color-border-subtle) bg-(--color-bg-deep)/92 px-2 pb-[env(safe-area-inset-bottom)] backdrop-blur-lg">
				<div className="mx-auto grid max-w-md grid-cols-5 gap-1 py-2">
					{navItems.map((item) => {
						const Icon = item.icon;
						const isActive =
							item.to === "/"
								? pathname === item.to
								: pathname === item.to || pathname.startsWith(`${item.to}/`);

						return (
							<Link
								key={item.to}
								to={item.to}
								className={cn(
									"flex flex-col items-center gap-1 rounded-(--radius-card) px-1 py-2 text-[0.6rem] font-medium uppercase tracking-[0.18em] transition-colors",
									isActive
										? "bg-(--color-accent-soft)/12 text-(--color-accent-soft)"
										: "text-(--color-text-muted) hover:bg-(--color-bg-surface) hover:text-(--color-text-secondary)",
								)}
							>
								<Icon
									className={cn(
										"size-5",
										isActive &&
											"drop-shadow-[0_0_6px_var(--color-accent-soft)]",
									)}
								/>
								<span>{item.label}</span>
							</Link>
						);
					})}
				</div>
			</nav>
		</div>
	);
}
