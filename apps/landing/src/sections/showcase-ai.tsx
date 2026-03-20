import { Hash, Sparkles } from "lucide-react";
import { AnimatedCounter } from "../components/animated-counter";
import { CondenseAnimation } from "../components/condense-animation";
import { MockAvatar } from "../components/mock-avatar";
import { MockPhone } from "../components/mock-phone";
import { useInView } from "../hooks/use-in-view";

export function ShowcaseAi() {
	const { ref, visible } = useInView();

	return (
		<section ref={ref} className="px-6 py-20">
			<div className="mx-auto max-w-5xl">
				<div className="grid items-center justify-items-center gap-12 lg:justify-items-stretch lg:grid-cols-[1fr_auto]">
					<div className={visible ? "anim-slide-left" : "opacity-0"}>
						<p
							className="text-sm font-medium tracking-wide uppercase"
							style={{ color: "var(--color-accent-soft)" }}
						>
							<Sparkles
								size={14}
								className="mr-1.5 inline"
								style={{ verticalAlign: "-2px" }}
							/>
							AI-powered summaries
						</p>
						<h2
							className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl lg:text-5xl"
							style={{ color: "var(--color-text-primary)" }}
						>
							<AnimatedCounter end={50} visible={visible} /> messages.
							<br />
							<span style={{ color: "var(--color-accent)" }}>
								<AnimatedCounter end={3} duration={800} visible={visible} />{" "}
								sentences.
							</span>
						</h2>
						<p
							className="mt-5 max-w-lg text-lg leading-relaxed"
							style={{ color: "var(--color-text-secondary)" }}
						>
							Every thread with replies gets an AI-generated summary — topic
							tags, a status classification, and a clear takeaway. Skip the
							noise, get the signal.
						</p>

						<div className="mt-8 space-y-4">
							{[
								{
									label: "Summary",
									desc: "2-3 factual sentences capturing the essence of the discussion",
								},
								{
									label: "Why it mattered",
									desc: "One-line practical takeaway — the decision, the fix, the action item",
								},
								{
									label: "Status & topics",
									desc: "Answered, unresolved, announcement, debate — plus 2-5 topic tags",
								},
								{
									label: "Staleness detection",
									desc: "Summaries auto-flag when the thread has new activity since generation",
								},
							].map((item) => (
								<div key={item.label} className="flex gap-3">
									<div
										className="mt-1 h-2 w-2 shrink-0 rounded-full"
										style={{ background: "var(--color-accent)" }}
									/>
									<div>
										<span
											className="text-sm font-semibold"
											style={{ color: "var(--color-text-bright)" }}
										>
											{item.label}
										</span>
										<p
											className="text-sm"
											style={{ color: "var(--color-text-tertiary)" }}
										>
											{item.desc}
										</p>
									</div>
								</div>
							))}
						</div>

						{/* Condense animation: many lines → few lines */}
						<div
							className={`mt-8 rounded-xl border p-4 sm:p-5 ${visible ? "anim-fade-up d-400" : "opacity-0"}`}
							style={{
								borderColor: "var(--color-border-subtle)",
								background:
									"color-mix(in srgb, var(--color-bg-panel) 60%, transparent)",
							}}
						>
							<CondenseAnimation visible={visible} />
						</div>
					</div>

					{/* Mobile view of AI summary */}
					<div
						className={`${visible ? "anim-slide-right d-300" : "opacity-0"} anim-float-slow`}
					>
						<MockPhone>
							<div
								className="flex-1 p-3"
								style={{
									background:
										"radial-gradient(circle at top center, color-mix(in srgb, var(--color-accent) 6%, transparent), transparent 50%), var(--color-bg-base)",
								}}
							>
								{/* Summary panel */}
								<div
									className="rounded-lg border p-3"
									style={{
										borderColor: "var(--color-border-subtle)",
										background:
											"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
										boxShadow: "var(--shadow-panel)",
									}}
								>
									<div className="flex items-center gap-1">
										<Sparkles
											size={9}
											style={{ color: "var(--color-accent-soft)" }}
										/>
										<p
											className="text-[7px] font-medium tracking-[0.16em] uppercase"
											style={{ color: "var(--color-accent-soft)" }}
										>
											AI Summary
										</p>
									</div>

									<p
										className="mt-1.5 text-[10px] font-semibold leading-snug"
										style={{ color: "var(--color-text-primary)" }}
									>
										Post-mortem: API latency spike
									</p>

									<div className="mt-1.5 flex items-center gap-1.5">
										<MockAvatar initials="JO" tone={5} size="sm" />
										<span
											className="text-[8px] font-medium"
											style={{ color: "var(--color-text-bright)" }}
										>
											Jamie Ortiz
										</span>
										<span className="channel-tone-3 rounded border px-1.5 py-0.5 text-[7px] font-medium">
											<Hash
												size={6}
												className="mr-0.5 inline"
												strokeWidth={2.5}
											/>
											incidents
										</span>
									</div>

									{/* Summary text */}
									<div
										className="mt-2.5 rounded-md border p-2"
										style={{
											borderColor:
												"color-mix(in srgb, var(--color-accent) 16%, transparent)",
											background:
												"color-mix(in srgb, var(--color-accent) 5%, transparent)",
										}}
									>
										<p
											className="text-[8px] leading-relaxed"
											style={{ color: "var(--color-text-secondary)" }}
										>
											The connection pool was exhausted due to a missing timeout
											on the aggregation worker. A 30s timeout and circuit
											breaker were added. Monitoring dashboard deployed.
										</p>
										<p
											className="mt-1.5 text-[7px] font-medium italic"
											style={{ color: "var(--color-accent-soft)" }}
										>
											Why it mattered: Production fix deployed same day,
											preventing recurrence during peak hours.
										</p>
									</div>

									{/* Chips */}
									<div className="mt-2 flex flex-wrap gap-1">
										<span
											className="rounded border px-1.5 py-0.5 text-[7px] font-medium"
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
										{["post-mortem", "latency", "database"].map((tag) => (
											<span
												key={tag}
												className="rounded border px-1.5 py-0.5 text-[7px] font-medium"
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
								</div>

								{/* Thread messages below */}
								{[
									{
										initials: "JO",
										tone: 5,
										name: "Jamie Ortiz",
										text: "Root cause: pool exhaustion from missing timeout on aggregation worker.",
										time: "10:32",
										reactions: ["\u{1F44D} 4"],
									},
									{
										initials: "SC",
										tone: 1,
										name: "Sarah Chen",
										text: "I can set up the circuit breaker PR today.",
										time: "10:45",
										reactions: ["\u2764\uFE0F 3"],
									},
									{
										initials: "MP",
										tone: 4,
										name: "Maya Patel",
										text: "Added monitoring dashboard for pool health.",
										time: "11:02",
										reactions: ["\u{1F680} 5"],
									},
								].map((msg) => (
									<div
										key={msg.time}
										className="mt-2 rounded-lg border p-2"
										style={{
											borderColor: "var(--color-border-subtle)",
											background:
												"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
										}}
									>
										<div className="flex gap-1.5">
											<MockAvatar
												initials={msg.initials}
												tone={msg.tone}
												size="sm"
											/>
											<div className="min-w-0 flex-1">
												<div className="flex items-center gap-1">
													<span
														className="text-[7px] font-medium"
														style={{ color: "var(--color-text-bright)" }}
													>
														{msg.name}
													</span>
													<span
														className="text-[6px]"
														style={{ color: "var(--color-text-quiet)" }}
													>
														{msg.time}
													</span>
												</div>
												<p
													className="mt-0.5 text-[7px] leading-relaxed"
													style={{ color: "var(--color-text-tertiary)" }}
												>
													{msg.text}
												</p>
												<div className="mt-1 flex gap-1">
													{msg.reactions.map((r) => (
														<span
															key={r}
															className="rounded border px-1 py-0.5 text-[6px]"
															style={{
																borderColor: "var(--color-border-strong)",
																background:
																	"color-mix(in srgb, white 3%, transparent)",
																color: "var(--color-text-tertiary)",
															}}
														>
															{r}
														</span>
													))}
												</div>
											</div>
										</div>
									</div>
								))}
							</div>
						</MockPhone>
					</div>
				</div>
			</div>
		</section>
	);
}
