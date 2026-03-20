export function SearchScan({ visible }: { visible: boolean }) {
	if (!visible) return null;

	return (
		<div className="pointer-events-none absolute inset-0 overflow-hidden rounded-xl">
			<div
				className="absolute left-0 right-0 h-10"
				style={{
					background:
						"linear-gradient(to bottom, transparent, color-mix(in srgb, var(--color-accent) 10%, transparent), transparent)",
					animation: "search-scan 3.5s var(--ease-in-out-quad) 0.8s infinite",
				}}
			/>
		</div>
	);
}
