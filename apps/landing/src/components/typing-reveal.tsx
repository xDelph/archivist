import { useEffect, useState } from "react";

export function TypingReveal({
	text,
	speed = 25,
	delay = 0,
	visible,
	className,
	style,
}: {
	text: string;
	speed?: number;
	delay?: number;
	visible: boolean;
	className?: string;
	style?: React.CSSProperties;
}) {
	const [charCount, setCharCount] = useState(0);

	useEffect(() => {
		if (!visible) return;

		const timeout = setTimeout(() => {
			let i = 0;
			const interval = setInterval(() => {
				i++;
				setCharCount(i);
				if (i >= text.length) clearInterval(interval);
			}, speed);
			return () => clearInterval(interval);
		}, delay);

		return () => clearTimeout(timeout);
	}, [visible, text, speed, delay]);

	return (
		<span className={className} style={style}>
			{text.slice(0, charCount)}
			{visible && charCount < text.length && (
				<span
					className="anim-typing ml-0.5 inline-block"
					style={{ color: "var(--color-accent-soft)" }}
				>
					▎
				</span>
			)}
		</span>
	);
}
