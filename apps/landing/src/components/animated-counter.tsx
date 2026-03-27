import { useEffect, useState } from "react";

export function AnimatedCounter({
	end,
	duration = 1200,
	visible,
}: {
	end: number;
	duration?: number;
	visible: boolean;
}) {
	const [count, setCount] = useState(0);

	useEffect(() => {
		if (!visible) return;
		const startTime = performance.now();

		function tick() {
			const elapsed = performance.now() - startTime;
			const progress = Math.min(elapsed / duration, 1);
			// ease-out-quart
			const eased = 1 - (1 - progress) ** 4;
			setCount(Math.round(end * eased));
			if (progress < 1) requestAnimationFrame(tick);
		}

		requestAnimationFrame(tick);
	}, [visible, end, duration]);

	return <>{count}</>;
}
