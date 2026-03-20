import {
	Bookmark,
	CloudOff,
	Download,
	House,
	Search,
	Smartphone,
	Wifi,
	WifiOff,
} from "lucide-react";
import { MockAvatar } from "../components/mock-avatar";
import { MockPhone } from "../components/mock-phone";
import { StackingArchive } from "../components/stacking-archive";
import { useInView } from "../hooks/use-in-view";

export function Offline() {
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
							<WifiOff
								size={14}
								className="mr-1.5 inline"
								style={{ verticalAlign: "-2px" }}
							/>
							Offline-first PWA
						</p>
						<h2
							className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl lg:text-5xl"
							style={{ color: "var(--color-text-primary)" }}
						>
							No signal?
							<br />
							<span style={{ color: "var(--color-accent)" }}>No problem.</span>
						</h2>
						<p
							className="mt-5 max-w-lg text-lg leading-relaxed"
							style={{ color: "var(--color-text-secondary)" }}
						>
							Arkivist is a Progressive Web App. Save threads to your offline
							library and read them anywhere — on a plane, in the subway, or
							during an outage. Your knowledge travels with you.
						</p>

						<div className="mt-8 grid gap-4 sm:grid-cols-2">
							{[
								{
									icon: Download,
									title: "Save for offline",
									desc: "Bookmark any thread and it's automatically cached to IndexedDB — messages, reactions, files, and all avatars.",
								},
								{
									icon: Smartphone,
									title: "Install as app",
									desc: "Add to home screen on iOS or Android. Full-screen experience with native-like navigation and transitions.",
								},
								{
									icon: CloudOff,
									title: "Smart cache strategy",
									desc: "Service Worker with stale-while-revalidate for assets, network-first for navigation, and automatic cache trimming.",
								},
								{
									icon: Wifi,
									title: "Seamless sync",
									desc: "When you're back online, Arkivist picks up where you left off. No manual refresh needed.",
								},
							].map((item) => (
								<div
									key={item.title}
									className="rounded-xl border p-4"
									style={{
										borderColor: "var(--color-border-subtle)",
										background: "color-mix(in srgb, white 3%, transparent)",
									}}
								>
									<div className="flex items-center gap-2">
										<item.icon
											size={18}
											className="shrink-0"
											style={{ color: "var(--color-accent-soft)" }}
										/>
										<h3
											className="text-sm font-semibold"
											style={{ color: "var(--color-text-bright)" }}
										>
											{item.title}
										</h3>
									</div>
									<p
										className="mt-1.5 text-[13px] leading-relaxed"
										style={{ color: "var(--color-text-tertiary)" }}
									>
										{item.desc}
									</p>
								</div>
							))}
						</div>

						{/* Stacking archive animation */}
						<div
							className={`mt-8 flex justify-center ${visible ? "anim-fade-up d-400" : "opacity-0"}`}
						>
							<StackingArchive visible={visible} />
						</div>
					</div>

					{/* Mobile offline view */}
					<div
						className={`${visible ? "anim-slide-right d-300" : "opacity-0"} anim-float-slow`}
					>
						<MockPhone>
							<div
								className="flex-1 p-3"
								style={{ background: "var(--color-bg-base)" }}
							>
								{/* Offline banner */}
								<div
									className="flex items-center gap-2 rounded-lg border px-2.5 py-2"
									style={{
										borderColor:
											"color-mix(in srgb, var(--color-accent) 30%, transparent)",
										background:
											"color-mix(in srgb, var(--color-accent) 10%, transparent)",
									}}
								>
									<CloudOff
										size={11}
										style={{ color: "var(--color-accent-soft)" }}
									/>
									<span
										className="text-[8px] font-medium"
										style={{ color: "var(--color-accent-soft)" }}
									>
										You&apos;re offline — showing saved threads
									</span>
								</div>

								{/* Saved section */}
								<div
									className="mt-2.5 rounded-lg border p-3"
									style={{
										borderColor: "var(--color-border-subtle)",
										background:
											"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
										boxShadow: "var(--shadow-panel)",
									}}
								>
									<p
										className="text-[7px] font-medium tracking-[0.16em] uppercase"
										style={{ color: "var(--color-accent-soft)" }}
									>
										Saved
									</p>
									<p
										className="mt-0.5 text-[10px] font-semibold"
										style={{ color: "var(--color-text-primary)" }}
									>
										Your library
									</p>
								</div>

								{/* Saved thread cards */}
								{[
									{
										initials: "SC",
										tone: 1,
										title: "RFC: Event-driven architecture",
										channel: "engineering",
										channelTone: 1,
										replies: 23,
										offline: true,
									},
									{
										initials: "JO",
										tone: 5,
										title: "Post-mortem: API latency spike",
										channel: "incidents",
										channelTone: 3,
										replies: 31,
										offline: true,
									},
									{
										initials: "AR",
										tone: 3,
										title: "Q2 roadmap priorities",
										channel: "product",
										channelTone: 2,
										replies: 15,
										offline: false,
									},
								].map((t) => (
									<div
										key={t.title}
										className="mt-2 rounded-lg border p-2.5"
										style={{
											borderColor: "var(--color-border-subtle)",
											background:
												"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
										}}
									>
										<div className="flex gap-2">
											<MockAvatar
												initials={t.initials}
												tone={t.tone}
												size="sm"
											/>
											<div className="min-w-0 flex-1">
												<div className="flex items-center gap-1">
													<span
														className={`channel-tone-${t.channelTone} rounded border px-1 py-0.5 text-[6px] font-medium`}
													>
														#{t.channel}
													</span>
													{t.offline && (
														<span
															className="inline-flex items-center gap-0.5 rounded-full border px-1.5 py-0.5 text-[6px] font-medium"
															style={{
																borderColor:
																	"color-mix(in srgb, var(--color-accent) 30%, transparent)",
																background:
																	"color-mix(in srgb, var(--color-accent) 10%, transparent)",
																color: "var(--color-accent-soft)",
															}}
														>
															<CloudOff size={6} />
															Offline
														</span>
													)}
												</div>
												<p
													className="mt-0.5 text-[8px] font-medium leading-snug"
													style={{
														color: t.offline
															? "var(--color-text-primary)"
															: "var(--color-text-quiet)",
													}}
												>
													{t.title}
												</p>
												<span
													className="mt-0.5 text-[7px]"
													style={{ color: "var(--color-text-soft)" }}
												>
													{t.replies} replies
												</span>
											</div>
										</div>
									</div>
								))}
							</div>

							{/* Mobile bottom nav */}
							<div
								className="mt-auto flex border-t"
								style={{
									borderColor: "var(--color-border-subtle)",
									background:
										"color-mix(in srgb, var(--color-bg-deep) 88%, black 12%)",
								}}
							>
								{[
									{ icon: House, label: "Catch up", active: false },
									{ icon: Search, label: "Search", active: false },
									{ icon: Bookmark, label: "Saved", active: true },
								].map((item) => (
									<div
										key={item.label}
										className="flex flex-1 flex-col items-center gap-0.5 py-2"
										style={{
											color: item.active
												? "var(--color-accent-strong)"
												: "var(--color-text-quiet)",
										}}
									>
										<item.icon size={14} />
										<span className="text-[7px] font-medium">{item.label}</span>
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
