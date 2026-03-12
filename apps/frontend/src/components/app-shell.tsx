import { IdentityAvatar } from "@/components/identity-avatar";
import { InstallBanner } from "@/components/install-banner";
import { authQueries } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { Link, Outlet, useRouterState } from "@tanstack/react-router";
import { Archive, Bookmark, House, Search, UserRound } from "lucide-react";

const navItems = [
	{ to: "/", label: "Catch up", icon: House },
	{ to: "/search", label: "Search", icon: Search },
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
		<div className="min-h-dvh bg-[#050607] pb-20 text-white">
			<header className="sticky top-0 z-30 border-b border-white/6 bg-black/85 backdrop-blur-xl">
				<div className="mx-auto flex max-w-[1500px] items-center gap-4 px-4 py-4 sm:px-6">
					<Link to="/" className="flex items-center gap-4">
						<span className="flex size-12 items-center justify-center rounded-2xl bg-[#18cc77] text-black shadow-[0_8px_24px_rgba(24,204,119,0.25)]">
							<Archive className="size-6" />
						</span>
						<div>
							<p className="text-[1.7rem] font-semibold leading-none tracking-tight text-white">
								Archivist
							</p>
						</div>
					</Link>

					<nav className="hidden items-center gap-2 lg:flex">
						{navItems.map((item) => {
							const isActive =
								item.to === "/"
									? pathname === item.to
									: pathname === item.to || pathname.startsWith(`${item.to}/`);

							return (
								<Link
									key={item.to}
									to={item.to}
									className={cn(
										"rounded-2xl border px-5 py-3 text-lg text-[#c0c1c6] transition-colors",
										isActive
											? "border-white/12 bg-black text-white"
											: "border-transparent bg-white/4 hover:border-white/10 hover:bg-white/8 hover:text-white",
									)}
								>
									{item.label}
								</Link>
							);
						})}
					</nav>

					<Link
						to="/account"
						className="ml-auto flex items-center gap-3 rounded-full border border-white/10 bg-white/4 px-2 py-1.5 transition-colors hover:border-white/16 hover:bg-white/8"
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
						<div className="hidden pr-3 text-right sm:block">
							<p className="text-lg font-medium leading-tight text-white">
								{user?.display_name || "Archivist"}
							</p>
							<p className="text-sm text-[#868790]">
								{user?.email || user?.slack_user_id || "Slack"}
							</p>
						</div>
					</Link>
				</div>
			</header>

			<div className="mx-auto flex w-full max-w-[1500px] flex-col px-4 py-6 sm:px-6">
				<InstallBanner />
				<main className="flex-1">
					<Outlet />
				</main>
			</div>

			<nav className="fixed inset-x-0 bottom-0 z-30 border-t border-white/8 bg-black/92 px-2 pb-[env(safe-area-inset-bottom)] backdrop-blur-xl lg:hidden">
				<div className="mx-auto grid max-w-md grid-cols-4 gap-1 py-2">
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
									"flex flex-col items-center gap-1 rounded-xl px-1 py-2 text-[0.68rem] uppercase tracking-[0.18em] transition-colors",
									isActive
										? "bg-[#18cc77]/14 text-[#3be18b]"
										: "text-[#7a7c84] hover:bg-white/6 hover:text-white",
								)}
							>
								<Icon className="size-5" />
								<span>{item.label}</span>
							</Link>
						);
					})}
				</div>
			</nav>
		</div>
	);
}
