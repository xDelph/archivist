import { z } from "zod";

export const homeTabValues = [
	"highlights",
	"fresh",
	"steady",
	"trending",
] as const;

export const homeTabSchema = z.enum(homeTabValues);

export type HomeTab = (typeof homeTabValues)[number];

export function normalizeHomeTab(tab?: string): HomeTab {
	return homeTabSchema.catch("highlights").parse(tab);
}

export function toHomeTabSearch(tab: HomeTab) {
	return tab === "highlights" ? undefined : tab;
}
