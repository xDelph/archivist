import {
	Bookmark,
	Hash,
	Heart,
	Link2,
	MessageSquare,
	Paperclip,
	Sparkles,
	Users,
} from "lucide-react";
import { MockAvatar } from "../components/mock-avatar";
import { MockBrowser } from "../components/mock-browser";
import { TypingReveal } from "../components/typing-reveal";
import { useInView } from "../hooks/use-in-view";

export function ShowcaseDetail() {
	const { ref, visible } = useInView();

	return (
		<section ref={ref} className="px-6 py-20">
			<div className="mx-auto max-w-5xl">
				<div className="grid items-center justify-items-center gap-12 lg:justify-items-stretch lg:grid-cols-[1.4fr_1fr]">
					<div className={visible ? "anim-slide-left" : "opacity-0"}>
						<MockBrowser url="app.arkivist.dev/threads/...">
							<div
								className="p-4"
								style={{
									background:
										"radial-gradient(circle at top center, color-mix(in srgb, var(--color-accent) 6%, transparent), transparent 50%), var(--color-bg-base)",
								}}
							>
								{/* Summary panel */}
								<div
									className="rounded-xl border p-4"
									style={{
										borderColor: "var(--color-border-subtle)",
										background:
											"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
										boxShadow: "var(--shadow-panel)",
									}}
								>
									<div className="flex items-start justify-between">
										<div className="min-w-0 flex-1">
											<p
												className="text-[9px] font-medium tracking-[0.16em] uppercase"
												style={{ color: "var(--color-accent-soft)" }}
											>
												AI Summary
											</p>
											<h3
												className="mt-1.5 text-base font-semibold leading-snug tracking-tight sm:text-lg"
												style={{ color: "var(--color-text-primary)" }}
											>
												Post-mortem: API latency spike March 12
											</h3>
										</div>
										<button
											type="button"
											className="ml-3 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg"
											style={{
												background: "var(--color-accent)",
												color: "oklch(0.15 0.013 152)",
												boxShadow: "var(--shadow-accent)",
											}}
										>
											<Bookmark size={14} fill="currentColor" />
										</button>
									</div>

									{/* Author + channel */}
									<div className="mt-3 flex items-center gap-2">
										<MockAvatar initials="JO" tone={5} />
										<span
											className="text-[12px] font-medium"
											style={{ color: "var(--color-text-bright)" }}
										>
											Jamie Ortiz
										</span>
										<span
											className="channel-tone-3 inline-flex items-center gap-0.5 rounded-lg border px-2 py-0.5 text-[10px] font-medium"
											style={{
												boxShadow: "inset 0 1px 0 rgba(255,255,255,0.04)",
											}}
										>
											<Hash size={8} strokeWidth={2.5} />
											incidents
										</span>
										<span
											className="text-[10px]"
											style={{ color: "var(--color-text-quiet)" }}
										>
											6h ago
										</span>
									</div>

									{/* Chips */}
									<div className="mt-3 flex flex-wrap gap-1.5">
										<span
											className="inline-flex items-center gap-1 rounded-lg border px-2.5 py-1 text-[10px] font-medium tracking-wide"
											style={{
												borderColor:
													"color-mix(in srgb, oklch(0.77 0.17 150) 38%, transparent)",
												background:
													"color-mix(in srgb, oklch(0.77 0.17 150) 14%, var(--color-bg-base))",
												color:
													"color-mix(in srgb, oklch(0.77 0.17 150) 82%, white 18%)",
											}}
										>
											Answered
										</span>
										{[
											"post-mortem",
											"latency",
											"database",
											"connection-pool",
										].map((tag) => (
											<span
												key={tag}
												className="rounded-lg border px-2.5 py-1 text-[10px] font-medium"
												style={{
													borderColor: "var(--color-border-strong)",
													background:
														"color-mix(in srgb, white 5%, transparent)",
													color: "var(--color-text-tertiary)",
												}}
											>
												{tag}
											</span>
										))}
									</div>

									{/* Metrics */}
									<div
										className="mt-3 flex items-center gap-3 text-[10px]"
										style={{ color: "var(--color-text-soft)" }}
									>
										<span className="inline-flex items-center gap-1">
											<MessageSquare size={11} />
											31
										</span>
										<span className="inline-flex items-center gap-1">
											<Heart size={11} />7
										</span>
										<span className="inline-flex items-center gap-1">
											<Users size={11} />
											11
										</span>
									</div>
								</div>

								{/* Tab navigation */}
								<div
									className="mt-3 flex gap-0.5 rounded-xl border p-1"
									style={{
										borderColor: "var(--color-border-subtle)",
										background: "color-mix(in srgb, white 2%, transparent)",
									}}
								>
									{[
										{ icon: Sparkles, label: "Highlights", active: true },
										{ icon: MessageSquare, label: "Timeline", active: false },
										{ icon: Link2, label: "Links", active: false },
										{ icon: Paperclip, label: "Files", active: false },
									].map((tab) => (
										<div
											key={tab.label}
											className="flex flex-1 items-center justify-center gap-1 rounded-lg py-2 text-[10px] font-medium"
											style={{
												background: tab.active
													? "color-mix(in srgb, var(--color-accent) 14%, transparent)"
													: "transparent",
												color: tab.active
													? "var(--color-accent-soft)"
													: "var(--color-text-quiet)",
												border: tab.active
													? "1px solid color-mix(in srgb, var(--color-accent) 20%, transparent)"
													: "1px solid transparent",
											}}
										>
											<tab.icon size={11} />
											{tab.label}
										</div>
									))}
								</div>

								{/* Messages */}
								<div className="mt-3 space-y-2">
									<MockMessage
										initials="JO"
										tone={5}
										author="Jamie Ortiz"
										time="10:32 AM"
										content={
											<TypingReveal
												text="Root cause identified: the connection pool was exhausted due to a missing timeout on the aggregation worker. The worker was holding connections for 2+ minutes during peak backfill runs."
												speed={18}
												delay={600}
												visible={visible}
											/>
										}
										reactions={["thumbsup-4", "eyes-2"]}
									/>
									<MockMessage
										initials="SC"
										tone={1}
										author="Sarah Chen"
										time="10:45 AM"
										content="Good find. We should add a circuit breaker for the worker pool. I can set up a PR for the connection timeout fix today."
										reactions={["heart-3"]}
										isReply
									/>
									<MockMessage
										initials="MP"
										tone={4}
										author="Maya Patel"
										time="11:02 AM"
										content="Added monitoring for pool utilization in Grafana. Dashboard link: grafana.internal/d/pool-health. Alert threshold set at 80% capacity."
										reactions={["rocket-5"]}
										isReply
									/>
								</div>
							</div>
						</MockBrowser>
					</div>

					<div className={visible ? "anim-slide-right d-200" : "opacity-0"}>
						<p
							className="text-sm font-medium tracking-wide uppercase"
							style={{ color: "var(--color-accent-soft)" }}
						>
							Thread detail
						</p>
						<h2
							className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl"
							style={{ color: "var(--color-text-primary)" }}
						>
							The full picture, at a glance
						</h2>
						<p
							className="mt-4 text-lg leading-relaxed"
							style={{ color: "var(--color-text-secondary)" }}
						>
							AI-powered summaries distill long threads into key takeaways,
							topic tags, and status. Browse highlights, full timelines,
							extracted links, and attached files — all in one view.
						</p>
					</div>
				</div>
			</div>
		</section>
	);
}

function MockMessage({
	initials,
	tone,
	author,
	time,
	content,
	reactions,
	isReply,
}: {
	initials: string;
	tone: number;
	author: string;
	time: string;
	content: React.ReactNode;
	reactions: string[];
	isReply?: boolean;
}) {
	return (
		<div
			className="rounded-xl border p-3"
			style={{
				borderColor: "var(--color-border-subtle)",
				background: "color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
				boxShadow:
					"0 8px 24px rgba(0,0,0,0.16), inset 0 1px 0 rgba(255,255,255,0.03)",
			}}
		>
			<div className="flex gap-2.5">
				<MockAvatar initials={initials} tone={tone} />
				<div className="min-w-0 flex-1">
					<div className="flex items-center gap-2">
						<span
							className="text-[12px] font-medium"
							style={{ color: "var(--color-text-bright)" }}
						>
							{author}
						</span>
						<span
							className="text-[10px]"
							style={{ color: "var(--color-text-quiet)" }}
						>
							{isReply && "↩ "}
							{time}
						</span>
					</div>
					<p
						className="mt-1 text-[11px] leading-relaxed"
						style={{ color: "var(--color-text-tertiary)" }}
					>
						{content}
					</p>
					{reactions.length > 0 && (
						<div className="mt-1.5 flex gap-1">
							{reactions.map((r) => {
								const [emoji, count] = r.split("-");
								const emojiMap: Record<string, string> = {
									thumbsup: "\u{1F44D}",
									heart: "\u2764\uFE0F",
									eyes: "\u{1F440}",
									rocket: "\u{1F680}",
									fire: "\u{1F525}",
								};
								return (
									<span
										key={r}
										className="inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-[10px]"
										style={{
											borderColor: "var(--color-border-strong)",
											background: "color-mix(in srgb, white 3%, transparent)",
											color: "var(--color-text-tertiary)",
										}}
									>
										{emojiMap[emoji] ?? emoji} {count}
									</span>
								);
							})}
						</div>
					)}
				</div>
			</div>
		</div>
	);
}
