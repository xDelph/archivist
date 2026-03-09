import { Button, buttonVariants } from "@/components/ui/button";
import { slackAuthStartUrl } from "@/lib/api";
import { currentUserQueryOptions } from "@/lib/auth";
import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { ArrowRight, LockKeyhole } from "lucide-react";

export function SignInPage() {
	const userQuery = useQuery(currentUserQueryOptions());

	return (
		<main className="flex min-h-screen items-center px-4 py-8 sm:px-6">
			<div className="mx-auto grid w-full max-w-5xl gap-5 lg:grid-cols-[1.15fr_0.85fr]">
				<section className="overflow-hidden rounded-[2.5rem] border border-white/10 bg-[linear-gradient(135deg,rgba(18,31,49,0.96),rgba(8,15,26,0.9))] p-6 shadow-[0_30px_120px_rgba(5,12,24,0.42)] sm:p-8">
					<p className="text-[0.68rem] uppercase tracking-[0.34em] text-[var(--accent-soft)]">
						Archivist v2
					</p>
					<h1 className="mt-4 text-4xl font-semibold text-white sm:text-5xl">
						Sign in with Slack and open the public-channel archive.
					</h1>
					<p className="mt-4 max-w-2xl text-sm leading-7 text-slate-300 sm:text-base">
						The frontend now routes through the real `apps/api` auth endpoints.
						After sign-in, the app shell unlocks catch-up, search, thread
						detail, and your account view.
					</p>
					<div className="mt-8 flex flex-wrap gap-3">
						<a
							href={slackAuthStartUrl()}
							className={buttonVariants({ variant: "default" })}
						>
							<LockKeyhole className="mr-2 size-4" />
							Continue with Slack
						</a>
						{userQuery.data?.ok ? (
							<Link
								to="/"
								className={cn(buttonVariants({ variant: "secondary" }))}
							>
								Open app
							</Link>
						) : null}
					</div>
				</section>

				<section className="rounded-[2.2rem] border border-white/10 bg-white/[0.05] p-6 backdrop-blur sm:p-7">
					<p className="text-[0.68rem] uppercase tracking-[0.28em] text-slate-400">
						What you get
					</p>
					<ul className="mt-5 space-y-4 text-sm leading-6 text-slate-200">
						<li className="rounded-[1.3rem] border border-white/10 bg-slate-950/30 p-4">
							A catch-up feed with one-day and one-week windows.
						</li>
						<li className="rounded-[1.3rem] border border-white/10 bg-slate-950/30 p-4">
							Search results that stay attached to the thread they came from.
						</li>
						<li className="rounded-[1.3rem] border border-white/10 bg-slate-950/30 p-4">
							Thread detail with extracted links, files, and progressive
							transcript loading.
						</li>
					</ul>
					<div className="mt-6 rounded-[1.3rem] border border-[var(--accent-soft)]/15 bg-[var(--accent-soft)]/8 p-4 text-sm text-slate-200">
						<p className="font-medium text-white">Current local assumption</p>
						<p className="mt-2 leading-6">
							The Vite app proxies `/api` to `apps/api`, so sign-in and session
							cookies stay on the frontend origin during local development.
						</p>
					</div>
					{userQuery.data?.ok ? (
						<div className="mt-6">
							<Link
								to="/"
								className={cn(buttonVariants({ variant: "secondary" }))}
							>
								Already signed in
								<ArrowRight className="ml-2 size-4" />
							</Link>
						</div>
					) : (
						<div className="mt-6">
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
