import { Hash, Pin, Star, Terminal } from "lucide-react";
import { useInView } from "../hooks/use-in-view";

export function SlackCommands() {
	const { ref, visible } = useInView();

	return (
		<section ref={ref} className="px-6 py-20">
			<div className="mx-auto max-w-5xl">
				<div className="grid items-center justify-items-center gap-12 lg:justify-items-stretch lg:grid-cols-[auto_1fr]">
					{/* Slack-style command mock */}
					<div className={visible ? "anim-slide-left d-200" : "opacity-0"}>
						<div
							className="w-full max-w-sm overflow-hidden rounded-2xl border"
							style={{
								borderColor: "var(--color-border-subtle)",
								background: "var(--color-bg-panel)",
								boxShadow: "var(--shadow-panel)",
							}}
						>
							{/* Slack-like header */}
							<div
								className="flex items-center gap-2 border-b px-4 py-3"
								style={{
									borderColor: "var(--color-border-subtle)",
									background: "var(--color-bg-deep)",
								}}
							>
								<Hash size={14} style={{ color: "var(--color-text-quiet)" }} />
								<span
									className="text-sm font-medium"
									style={{ color: "var(--color-text-secondary)" }}
								>
									engineering
								</span>
							</div>

							{/* Commands */}
							<div className="space-y-3 p-4">
								{/* /pin-highlight command */}
								<div>
									<div
										className="flex items-center gap-1.5 text-[12px]"
										style={{ color: "var(--color-text-secondary)" }}
									>
										<Terminal
											size={12}
											style={{ color: "var(--color-accent-soft)" }}
										/>
										<span className="font-mono font-medium">
											/pin-highlight
										</span>
										<span
											className="text-[10px]"
											style={{ color: "var(--color-text-quiet)" }}
										>
											https://slack.com/archives/C123/p456...
										</span>
									</div>
									<div
										className="mt-1.5 ml-5 rounded-lg border-l-2 py-1.5 pl-3 text-[11px]"
										style={{
											borderColor: "var(--color-accent)",
											color: "var(--color-text-tertiary)",
										}}
									>
										<Pin
											size={10}
											className="mr-1 inline"
											style={{
												color: "var(--color-accent-soft)",
												verticalAlign: "-1px",
											}}
										/>
										Thread pinned as highlight for the whole team.
									</div>
								</div>

								{/* /list-highlights command */}
								<div>
									<div
										className="flex items-center gap-1.5 text-[12px]"
										style={{ color: "var(--color-text-secondary)" }}
									>
										<Terminal
											size={12}
											style={{ color: "var(--color-accent-soft)" }}
										/>
										<span className="font-mono font-medium">
											/list-highlights
										</span>
									</div>
									<div
										className="mt-1.5 ml-5 rounded-lg border-l-2 py-1.5 pl-3"
										style={{
											borderColor: "var(--color-accent)",
										}}
									>
										<div className="space-y-1.5">
											{[
												{
													ch: "engineering",
													title: "RFC: Event-driven architecture",
												},
												{
													ch: "product",
													title: "Q2 roadmap priorities",
												},
												{
													ch: "incidents",
													title: "Post-mortem: API latency spike",
												},
											].map((h) => (
												<div
													key={h.title}
													className="flex items-center gap-1.5 text-[10px]"
												>
													<Star
														size={9}
														fill="currentColor"
														style={{
															color: "var(--color-accent-soft)",
														}}
													/>
													<span
														className="channel-tone-1 rounded border px-1.5 py-0.5 text-[8px] font-medium"
														style={{
															boxShadow: "inset 0 1px 0 rgba(255,255,255,0.04)",
														}}
													>
														#{h.ch}
													</span>
													<span
														style={{
															color: "var(--color-text-tertiary)",
														}}
													>
														{h.title}
													</span>
												</div>
											))}
										</div>
									</div>
								</div>

								{/* Typing indicator */}
								<div className="flex items-center gap-1.5">
									<Terminal
										size={12}
										style={{ color: "var(--color-accent-soft)" }}
									/>
									<span
										className="font-mono text-[12px] font-medium"
										style={{ color: "var(--color-text-secondary)" }}
									>
										/unpin-highlight
									</span>
									<span
										className="anim-typing inline-block h-3.5 w-0.5 rounded-full"
										style={{ background: "var(--color-accent)" }}
									/>
								</div>
							</div>
						</div>
					</div>

					<div className={visible ? "anim-slide-right d-100" : "opacity-0"}>
						<p
							className="text-sm font-medium tracking-wide uppercase"
							style={{ color: "var(--color-accent-soft)" }}
						>
							Slack commands
						</p>
						<h2
							className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl"
							style={{ color: "var(--color-text-primary)" }}
						>
							Manage from where you already work
						</h2>
						<p
							className="mt-4 text-lg leading-relaxed"
							style={{ color: "var(--color-text-secondary)" }}
						>
							Pin important threads as team highlights, list what&apos;s pinned,
							or unpin — all without leaving Slack. Admins curate, everyone
							benefits.
						</p>

						<div className="mt-8 space-y-3">
							{[
								{
									cmd: "/pin-highlight",
									desc: "Pin a thread as a highlight. Paste the Slack permalink or use channel:timestamp format.",
								},
								{
									cmd: "/list-highlights",
									desc: "See all currently pinned highlights across channels. Ephemeral — only you see the response.",
								},
								{
									cmd: "/unpin-highlight",
									desc: "Remove a thread from highlights. Admin-only, same format as pin.",
								},
							].map((item) => (
								<div
									key={item.cmd}
									className="rounded-xl border p-4"
									style={{
										borderColor: "var(--color-border-subtle)",
										background: "color-mix(in srgb, white 3%, transparent)",
									}}
								>
									<code
										className="text-sm font-semibold"
										style={{ color: "var(--color-accent)" }}
									>
										{item.cmd}
									</code>
									<p
										className="mt-1 text-sm"
										style={{ color: "var(--color-text-tertiary)" }}
									>
										{item.desc}
									</p>
								</div>
							))}
						</div>
					</div>
				</div>
			</div>
		</section>
	);
}
