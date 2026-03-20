import { GitBranch, Lock, Server } from "lucide-react";
import { useInView } from "../hooks/use-in-view";

export function OpenSource() {
	const { ref, visible } = useInView();

	return (
		<section
			ref={ref}
			id="open-source"
			className={`px-6 py-20 ${visible ? "anim-fade-up" : "opacity-0"}`}
		>
			<div className="mx-auto max-w-5xl">
				<div
					className="overflow-hidden rounded-3xl border"
					style={{
						borderColor: "var(--color-border-subtle)",
						background:
							"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
						boxShadow: "var(--shadow-panel)",
					}}
				>
					<div className="grid items-center gap-8 p-8 md:grid-cols-2 md:p-12">
						<div>
							<p
								className="text-sm font-medium tracking-wide uppercase"
								style={{
									color: "var(--color-accent-soft)",
								}}
							>
								Open source
							</p>
							<h2
								className="mt-3 text-3xl font-bold tracking-tight"
								style={{
									color: "var(--color-text-primary)",
								}}
							>
								Your data, your infrastructure
							</h2>
							<p
								className="mt-4 text-base leading-relaxed"
								style={{
									color: "var(--color-text-secondary)",
								}}
							>
								Arkivist is AGPL-licensed. Self-host it on your own
								infrastructure, audit every line, and own your data completely.
								No vendor lock-in, no third-party data access.
							</p>
						</div>

						<div className="flex flex-col gap-4">
							<div className="flex items-start gap-3">
								<GitBranch
									size={20}
									style={{
										color: "var(--color-accent-soft)",
										marginTop: 2,
										flexShrink: 0,
									}}
								/>
								<div>
									<h3
										className="text-sm font-semibold"
										style={{
											color: "var(--color-text-bright)",
										}}
									>
										AGPL licensed
									</h3>
									<p
										className="mt-0.5 text-sm"
										style={{
											color: "var(--color-text-tertiary)",
										}}
									>
										Fork it, modify it, contribute back. Full source available
										on GitHub.
									</p>
								</div>
							</div>

							<div className="flex items-start gap-3">
								<Server
									size={20}
									style={{
										color: "var(--color-accent-soft)",
										marginTop: 2,
										flexShrink: 0,
									}}
								/>
								<div>
									<h3
										className="text-sm font-semibold"
										style={{
											color: "var(--color-text-bright)",
										}}
									>
										Self-hostable
									</h3>
									<p
										className="mt-0.5 text-sm"
										style={{
											color: "var(--color-text-tertiary)",
										}}
									>
										Rust backend + React frontend. Deploy anywhere — Vercel,
										Docker, bare metal.
									</p>
								</div>
							</div>

							<div className="flex items-start gap-3">
								<Lock
									size={20}
									style={{
										color: "var(--color-accent-soft)",
										marginTop: 2,
										flexShrink: 0,
									}}
								/>
								<div>
									<h3
										className="text-sm font-semibold"
										style={{
											color: "var(--color-text-bright)",
										}}
									>
										Commercial license available
									</h3>
									<p
										className="mt-0.5 text-sm"
										style={{
											color: "var(--color-text-tertiary)",
										}}
									>
										Need to run it without AGPL obligations? Commercial
										licensing is available.
									</p>
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</section>
	);
}
