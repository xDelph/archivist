export function formatSlackTimestamp(value: string) {
	const milliseconds = Number.parseFloat(value) * 1_000;
	if (Number.isNaN(milliseconds)) {
		return value;
	}

	return new Intl.DateTimeFormat("en", {
		month: "short",
		day: "numeric",
		hour: "numeric",
		minute: "2-digit",
	}).format(new Date(milliseconds));
}

export function formatCompactNumber(value: number) {
	return new Intl.NumberFormat("en", {
		notation: value >= 1_000 ? "compact" : "standard",
		maximumFractionDigits: 1,
	}).format(value);
}

export function formatStatLabel(
	value: number,
	singular: string,
	plural: string,
) {
	return `${formatCompactNumber(value)} ${value === 1 ? singular : plural}`;
}

export function initials(value: string | null | undefined) {
	if (!value) {
		return "AR";
	}

	const letters = value
		.split(/\s+/)
		.map((chunk) => chunk[0])
		.filter(Boolean)
		.join("")
		.slice(0, 2)
		.toUpperCase();

	return letters || "AR";
}
