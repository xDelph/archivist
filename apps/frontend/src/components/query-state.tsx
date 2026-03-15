import type { ReactNode } from "react";

interface QueryStateProps {
	isPending: boolean;
	isError: boolean;
	isEmpty: boolean;
	loading: ReactNode;
	error: ReactNode;
	empty: ReactNode;
	children: ReactNode;
}

export function QueryState({
	isPending,
	isError,
	isEmpty,
	loading,
	error,
	empty,
	children,
}: QueryStateProps) {
	if (isPending) {
		return loading;
	}

	if (isError) {
		return error;
	}

	if (isEmpty) {
		return empty;
	}

	return children;
}

interface CardSkeletonListProps {
	count?: number;
	cardClassName?: string;
	className?: string;
}

export function CardSkeletonList({
	count = 3,
	cardClassName = "surface-frost h-36 rounded-[1.45rem]",
	className = "space-y-2.5",
}: CardSkeletonListProps) {
	return (
		<div className={className}>
			{Array.from({ length: count }, (_, index) => (
				<div
					key={`skeleton-${index + 1}`}
					className={`animate-pulse ${cardClassName}`}
				/>
			))}
		</div>
	);
}
