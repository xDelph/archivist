import { usePrefersReducedMotion } from "@/lib/use-prefers-reduced-motion";
import { cn } from "@/lib/utils";
import { type ReactNode, useEffect, useRef, useState } from "react";

const EXIT_DURATION_MS = 130;
const ENTER_DURATION_MS = 210;

type TransitionPhase = "idle" | "exiting" | "entering";

interface TabPanelTransitionProps {
	value: string;
	children: ReactNode;
	className?: string;
}

export function TabPanelTransition({
	value,
	children,
	className,
}: TabPanelTransitionProps) {
	const prefersReducedMotion = usePrefersReducedMotion();
	const [renderedValue, setRenderedValue] = useState(value);
	const [renderedChildren, setRenderedChildren] = useState(children);
	const [phase, setPhase] = useState<TransitionPhase>("idle");
	const exitTimerRef = useRef<number | null>(null);
	const enterTimerRef = useRef<number | null>(null);

	useEffect(() => {
		return () => {
			if (exitTimerRef.current !== null) {
				window.clearTimeout(exitTimerRef.current);
			}

			if (enterTimerRef.current !== null) {
				window.clearTimeout(enterTimerRef.current);
			}
		};
	}, []);

	useEffect(() => {
		if (prefersReducedMotion) {
			if (exitTimerRef.current !== null) {
				window.clearTimeout(exitTimerRef.current);
			}

			if (enterTimerRef.current !== null) {
				window.clearTimeout(enterTimerRef.current);
			}

			setRenderedValue(value);
			setRenderedChildren(children);
			setPhase("idle");
			return;
		}

		if (value === renderedValue) {
			setRenderedChildren(children);
			return;
		}

		if (exitTimerRef.current !== null) {
			window.clearTimeout(exitTimerRef.current);
		}

		if (enterTimerRef.current !== null) {
			window.clearTimeout(enterTimerRef.current);
		}

		setPhase("exiting");
		exitTimerRef.current = window.setTimeout(() => {
			setRenderedValue(value);
			setRenderedChildren(children);
			setPhase("entering");
			enterTimerRef.current = window.setTimeout(() => {
				setPhase("idle");
			}, ENTER_DURATION_MS);
		}, EXIT_DURATION_MS);
	}, [children, prefersReducedMotion, renderedValue, value]);

	return (
		<div
			className={cn("tab-panel-transition", className)}
			data-phase={phase}
			data-value={renderedValue}
		>
			<div className="tab-panel-transition__stage">{renderedChildren}</div>
		</div>
	);
}
