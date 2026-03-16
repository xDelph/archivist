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
		<main className="relative flex min-h-dvh items-center justify-center overflow-hidden bg-(--color-bg-deep) px-6">
			<div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_45%,color-mix(in_srgb,var(--color-accent)_14%,transparent),transparent_28%),radial-gradient(circle_at_78%_18%,color-mix(in_srgb,var(--color-signal)_10%,transparent),transparent_18%),linear-gradient(to_bottom,rgba(255,255,255,0.03)_1px,transparent_1px),linear-gradient(to_right,rgba(255,255,255,0.02)_1px,transparent_1px)] [background-size:auto,auto,100%_96px,96px_100%]" />
			<div className="relative max-w-xl text-center">
				<p className="text-[0.82rem] font-medium uppercase tracking-[0.26em] text-(--color-accent)">
					Archivist
				</p>
				<h1 className="mt-5 text-[clamp(2.75rem,8vw,4.8rem)] font-semibold tracking-tight text-white">
					Find the decisions buried in Slack.
				</h1>
				<p className="mt-4 text-[1.05rem] leading-7 text-(--color-text-secondary) sm:text-[1.2rem]">
					Sign in with Slack to search public threads, review AI summaries, and
					return to important conversations without reopening the full backlog.
				</p>

				{isLoading ? (
					<div className="text-copy-soft mt-12 inline-flex items-center gap-3 text-[1.05rem]">
						<span className="size-4 animate-pulse rounded-full bg-(--color-warning) shadow-[0_0_0_6px_rgba(255,152,0,0.12)]" />
						Checking your Slack session
					</div>
				) : userQuery.data?.ok ? (
					<div className="mt-12 flex justify-center">
						<Link
							to="/"
							className={cn(
								buttonVariants({ variant: "secondary", size: "lg" }),
								"border-(--color-border-default) bg-white/6 text-white hover:border-(--color-border-accent) hover:bg-white/10",
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
									"bg-(--color-accent) text-black hover:bg-(--color-accent-strong)",
								)}
							>
								<LockKeyhole className="size-4" />
								Continue with Slack
							</a>
						</div>
						<p className="text-copy-quiet text-sm">
							Sign in to unlock catch-up, search, thread detail, and saved
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
							className="text-(--color-text-secondary) hover:bg-white/6 hover:text-white"
						>
							Retry session check
						</Button>
					</div>
				) : null}
			</div>
		</main>
	);
}
