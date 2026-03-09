import { Button, buttonVariants } from "@/components/ui/button";
import { slackAuthStartUrl } from "@/lib/api";
import { authQueries } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { ArrowRight, LockKeyhole } from "lucide-react";

export function SignInPage() {
	const userQuery = useQuery(authQueries.me());

	return (
		<main className="flex min-h-dvh items-center px-4 py-8 sm:px-6">
			<div className="mx-auto grid w-full max-w-4xl gap-5 lg:grid-cols-[1.15fr_0.85fr]">
				<section className="overflow-hidden rounded-(--radius-section) border border-(--color-border-subtle) bg-[linear-gradient(135deg,var(--color-bg-surface),var(--color-bg-base))] p-6 shadow-[0_24px_80px_oklch(0.05_0.02_220/0.5)] sm:p-8">
					<p className="text-[0.62rem] font-medium uppercase tracking-[0.36em] text-(--color-accent-soft)">
						Archivist v2
					</p>
					<h1 className="mt-4 text-3xl font-semibold text-(--color-text-primary) sm:text-4xl">
						Sign in with Slack and open the public-channel archive.
					</h1>
					<p className="mt-4 max-w-xl text-sm leading-relaxed text-(--color-text-secondary) sm:text-base">
						After sign-in, the app unlocks catch-up, search, thread detail,
						saved items, and your account view.
					</p>
					<div className="mt-8 flex flex-wrap gap-3">
						<a
							href={slackAuthStartUrl()}
							className={buttonVariants({ variant: "default" })}
						>
							<LockKeyhole className="size-4" />
							Continue with Slack
						</a>
						{userQuery.data?.ok && (
							<Link
								to="/"
								className={cn(buttonVariants({ variant: "secondary" }))}
							>
								Open app
							</Link>
						)}
					</div>
				</section>

				<section className="rounded-(--radius-section) border border-(--color-border-subtle) bg-(--color-bg-surface)/60 p-6 backdrop-blur-sm sm:p-7">
					<p className="text-[0.62rem] font-medium uppercase tracking-[0.28em] text-(--color-text-muted)">
						What you get
					</p>
					<ul className="mt-5 space-y-3 text-sm leading-relaxed text-(--color-text-primary)">
						<li className="rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 p-4">
							A catch-up feed with one-day and one-week windows.
						</li>
						<li className="rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 p-4">
							Search results that stay attached to the thread they came from.
						</li>
						<li className="rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 p-4">
							Thread detail with extracted links, files, and progressive
							transcript loading.
						</li>
					</ul>
					<div className="mt-5 rounded-(--radius-card) border border-(--color-accent-soft)/15 bg-(--color-accent-soft)/8 p-4 text-sm text-(--color-text-primary)">
						<p className="font-medium">Local development</p>
						<p className="mt-2 leading-relaxed text-(--color-text-secondary)">
							The Vite app proxies `/api` to `apps/api`, so sign-in and session
							cookies stay on the frontend origin during local development.
						</p>
					</div>
					{userQuery.data?.ok ? (
						<div className="mt-5">
							<Link
								to="/"
								className={cn(buttonVariants({ variant: "secondary" }))}
							>
								Already signed in
								<ArrowRight className="size-4" />
							</Link>
						</div>
					) : (
						<div className="mt-5">
							<Button type="button" variant="ghost" disabled>
								Waiting for Slack session
							</Button>
						</div>
					)}
				</section>
			</div>
		</main>
	);
}
