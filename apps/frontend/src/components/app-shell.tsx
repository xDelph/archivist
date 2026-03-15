import { IdentityAvatar } from "@/components/identity-avatar";
import { InstallBanner } from "@/components/install-banner";
import { authQueries } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { Archive, Bookmark, House, Search } from "lucide-react";

const navItems = [
	{ to: "/", label: "Catch up", icon: House },
	{ to: "/search", label: "Search", icon: Search },
	{ to: "/saved", label: "Saved", icon: Bookmark },
] as const;

export function AppShell() {
	const pathname = useRouterState({
		select: (state) => state.location.pathname,
	});
	const userQuery = useQuery(authQueries.me());
	const user = userQuery.data?.user;

	return (
		<div className="min-h-dvh bg-(--color-bg-deep) pb-20 text-white">
			<header className="sticky top-0 z-30 border-b border-white/6 bg-(--color-bg-deep)/94 backdrop-blur-xl">
				<div className="mx-auto flex max-w-[1680px] items-center gap-2 px-2.5 py-1 sm:px-3">
					<Link to="/" className="flex items-center gap-2">
						<span className="flex size-9 items-center justify-center rounded-lg bg-(--color-accent) text-black shadow-[0_8px_18px_rgba(24,204,119,0.18)]">
							<Archive className="size-3.5" />
						</span>
						<div>
							<p className="text-[1.2rem] font-semibold leading-none tracking-tight text-white sm:text-[1.3rem]">
								Archivist
							</p>
						</div>
					</Link>

					<nav className="hidden items-center rounded-xl border border-(--color-border-subtle) bg-white/[0.035] p-0.5 lg:flex">
						{navItems.map((item) => {
							const isActive = isNavItemActive(pathname, item.to);

							return (
								<Link
									key={item.to}
									to={item.to}
									className={cn(
										"rounded-md px-3 py-1.25 text-[0.8rem] text-(--color-text-secondary) transition-colors",
										isActive
											? "bg-black text-white shadow-[0_8px_20px_rgba(0,0,0,0.24)]"
											: "hover:bg-white/[0.04] hover:text-white",
									)}
								>
									{item.label}
								</Link>
							);
						})}
					</nav>

					<Link
						to="/account"
						className="ml-auto flex items-center justify-center rounded-full border border-(--color-border-strong) bg-white/[0.035] p-1 transition-colors hover:border-white/16 hover:bg-white/[0.06]"
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
							fallback={user?.email || "Archivist"}
							size="sm"
						/>
					</Link>
				</div>
			</header>

			<div className="mx-auto flex w-full max-w-[1680px] flex-col px-2.5 py-2.5 sm:px-3">
				<InstallBanner />
				<main className="flex-1">
					<Outlet />
				</main>
			</div>

			<nav className="fixed inset-x-0 bottom-0 z-30 border-t border-(--color-border-subtle) bg-black/92 px-2 pb-[env(safe-area-inset-bottom)] backdrop-blur-xl lg:hidden">
				<div className="mx-auto grid max-w-md grid-cols-3 gap-1 py-1.5">
					{navItems.map((item) => {
						const Icon = item.icon;
						const isActive = isNavItemActive(pathname, item.to);

						return (
							<Link
								key={item.to}
								to={item.to}
								className={cn(
									"flex flex-col items-center gap-0.5 rounded-xl px-1 py-1.5 text-[0.62rem] uppercase tracking-[0.16em] transition-colors",
									isActive
										? "bg-(--color-accent)/14 text-(--color-accent-strong)"
										: "text-(--color-text-muted) hover:bg-white/6 hover:text-white",
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

function isNavItemActive(pathname: string, to: string) {
	return to === "/"
		? pathname === to
		: pathname === to || pathname.startsWith(`${to}/`);
}
