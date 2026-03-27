import type { HomeTab } from "@/lib/home-tabs";

const launchScreenContent = {
	starred: {
		title: "Opening Arkivist",
		description:
			"Loading the threads your team already marked as worth revisiting.",
	},
	fresh: {
		title: "Opening Arkivist",
		description:
			"Loading the most recent conversations before the archive shell appears.",
	},
	steady: {
		title: "Opening Arkivist",
		description:
			"Loading the weekly thread rollup before the main archive view lands.",
	},
} as const;

export function AppLaunchScreen({
	tab,
}: {
	tab: HomeTab;
}) {
	const content = launchScreenContent[tab];

	return (
		<main className="relative min-h-dvh overflow-hidden bg-(--color-bg-deep) px-4 py-4 sm:px-6 sm:py-5">
			<div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_18%_18%,color-mix(in_srgb,var(--color-accent)_18%,transparent),transparent_24%),radial-gradient(circle_at_84%_12%,color-mix(in_srgb,var(--color-signal)_10%,transparent),transparent_18%),linear-gradient(to_bottom,var(--hero-grid-line)_1px,transparent_1px),linear-gradient(to_right,var(--hero-grid-line-soft)_1px,transparent_1px)] [background-size:auto,auto,100%_88px,88px_100%]" />

			<div className="relative mx-auto flex min-h-[calc(100dvh-2rem)] max-w-3xl items-center justify-center">
				<section className="surface-panel surface-panel-soft relative w-full overflow-hidden px-5 py-6 sm:px-7 sm:py-8">
					<div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_left,color-mix(in_srgb,var(--color-accent)_12%,transparent),transparent_40%)]" />
					<div className="relative">
						<h1 className="text-[clamp(2rem,6vw,3.4rem)] font-semibold leading-[1.04] tracking-tight text-(--color-text-primary)">
							{content.title}
						</h1>
						<p className="mt-4 max-w-2xl text-[1rem] leading-7 text-(--color-text-secondary) sm:text-[1.06rem]">
							{content.description}
						</p>

						<div className="surface-frost mt-6 rounded-[1.2rem] border border-(--color-border-subtle) p-4 sm:p-5">
							<div className="flex items-start gap-3">
								<span className="mt-1 block size-3 shrink-0 animate-pulse rounded-full bg-(--color-signal) shadow-[0_0_0_6px_color-mix(in_srgb,var(--color-signal)_12%,transparent)]" />
								<div className="min-w-0">
									<p className="text-[0.95rem] font-medium text-(--color-text-primary)">
										Waiting for first home query
									</p>
									<p className="text-copy-soft mt-1 text-[0.9rem] leading-6">
										This screen stays in front until the first real archive
										response finishes.
									</p>
								</div>
							</div>

							<div className="mt-4 h-2 rounded-full bg-(--surface-ghost-bg)">
								<div className="h-full w-2/5 animate-pulse rounded-full bg-(--color-signal)" />
							</div>
						</div>
					</div>
				</section>
			</div>
		</main>
	);
}
