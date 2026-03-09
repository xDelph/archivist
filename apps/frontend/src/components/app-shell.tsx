import { currentUserQueryOptions } from "@/lib/auth";
import { initials } from "@/lib/format";
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
	const location = useRouterState({
		select: (state) => state.location.pathname,
	});
	const userQuery = useQuery(currentUserQueryOptions());
	const user = userQuery.data?.user;

	return (
		<div className="relative min-h-screen pb-24">
			<div className="mx-auto flex min-h-screen w-full max-w-6xl flex-col px-4 pb-10 pt-5 sm:px-6">
				<header className="sticky top-0 z-20 -mx-2 mb-6 border-b border-white/10 bg-[rgba(7,17,26,0.82)] px-2 py-4 backdrop-blur sm:mx-0 sm:rounded-[1.5rem] sm:border sm:bg-[rgba(7,17,26,0.72)]">
					<div className="flex items-center justify-between gap-4">
						<div>
							<p className="text-[0.65rem] uppercase tracking-[0.32em] text-[var(--accent-soft)]">
								Archivist
							</p>
							<h1 className="mt-2 text-2xl font-semibold text-white">
								Public-channel catch-up
							</h1>
						</div>
						<div className="flex items-center gap-3 rounded-full border border-white/10 bg-white/[0.05] px-3 py-2">
							<div className="flex size-10 items-center justify-center rounded-full bg-[var(--accent-soft)]/18 text-sm font-semibold text-[var(--accent-soft)]">
								{initials(user?.display_name ?? user?.email)}
							</div>
							<div className="hidden text-right sm:block">
								<p className="text-sm font-medium text-white">
									{user?.display_name || "Signed-in member"}
								</p>
								<p className="text-xs text-slate-400">
									{user?.email || user?.slack_user_id || "Slack workspace"}
								</p>
							</div>
						</div>
					</div>
				</header>
				<div className="flex-1">
					<Outlet />
				</div>
			</div>
			<nav className="fixed inset-x-0 bottom-0 z-30 border-t border-white/10 bg-[rgba(5,11,18,0.94)] px-2 py-3 backdrop-blur">
				<div className="mx-auto grid max-w-xl grid-cols-5 gap-2">
					{navItems.map((item) => {
						const Icon = item.icon;
						const isActive =
							item.to === "/"
								? location === item.to
								: location === item.to || location.startsWith(`${item.to}/`);

						return (
							<Link
								key={item.to}
								to={item.to}
								className={cn(
									"flex flex-col items-center gap-1 rounded-[1.25rem] px-2 py-2 text-[0.65rem] uppercase tracking-[0.2em] text-slate-400 transition",
									isActive
										? "bg-[var(--accent-soft)]/14 text-[var(--accent-soft)]"
										: "hover:bg-white/[0.04] hover:text-white",
								)}
							>
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
