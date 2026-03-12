import { Button, buttonVariants } from "@/components/ui/button";
import { slackAuthStartUrl } from "@/lib/api";
import { authQueries } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { ArrowRight, LockKeyhole } from "lucide-react";

export function SignInPage() {
	const userQuery = useQuery(authQueries.me());
	const isLoading = userQuery.isPending;

	return (
		<main className="relative flex min-h-dvh items-center justify-center overflow-hidden bg-[#030406] px-6">
			<div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_50%,rgba(34,197,94,0.08),transparent_28%),linear-gradient(to_bottom,rgba(255,255,255,0.03)_1px,transparent_1px),linear-gradient(to_right,rgba(255,255,255,0.02)_1px,transparent_1px)] [background-size:auto,100%_96px,96px_100%]" />
			<div className="relative text-center">
				<p className="text-[0.9rem] uppercase tracking-[0.38em] text-[#18cc77]">
					Archivist
				</p>
				<h1 className="mt-6 text-6xl font-semibold tracking-tight text-white sm:text-7xl">
					Archivist
				</h1>
				<p className="mt-4 text-2xl text-[#7a7b83]">Slack message archiver</p>

				{isLoading ? (
					<div className="mt-12 inline-flex items-center gap-3 text-[1.05rem] text-[#8c8d94]">
						<span className="size-4 rounded-full bg-[#ff9800] shadow-[0_0_0_6px_rgba(255,152,0,0.12)] animate-pulse" />
						loading
					</div>
				) : userQuery.data?.ok ? (
					<div className="mt-12 flex justify-center">
						<Link
							to="/"
							className={cn(
								buttonVariants({ variant: "secondary", size: "lg" }),
								"border-white/12 bg-white/6 text-white hover:border-[#22c55e]/30 hover:bg-white/10",
							)}
						>
							Open archive
							<ArrowRight className="size-4" />
						</Link>
					</div>
				) : (
					<div className="mt-12 space-y-4">
						<div className="flex justify-center">
							<a
								href={slackAuthStartUrl()}
								className={cn(
									buttonVariants({ variant: "default", size: "lg" }),
									"bg-[#18cc77] text-black hover:bg-[#2ae38a]",
								)}
							>
								<LockKeyhole className="size-4" />
								Continue with Slack
							</a>
						</div>
						<p className="text-sm text-[#6d6f76]">
							Sign in to unlock search, catch-up, thread detail, and saved
							threads.
						</p>
					</div>
				)}

				{userQuery.isError ? (
					<div className="mt-6">
						<Button
							type="button"
							variant="ghost"
							onClick={() => void userQuery.refetch()}
							className="text-[#9ea0a7] hover:bg-white/6 hover:text-white"
						>
							Retry session check
						</Button>
					</div>
				) : null}
			</div>
		</main>
	);
}
