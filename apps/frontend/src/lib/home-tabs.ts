import { z } from "zod";

export const homeTabValues = [
	"fresh",
	"steady",
	"trending",
	"channels",
] as const;

export const homeTabSchema = z.enum(homeTabValues);

export type HomeTab = (typeof homeTabValues)[number];

export function normalizeHomeTab(tab?: string): HomeTab {
	return homeTabSchema.catch("fresh").parse(tab);
}

export function toHomeTabSearch(tab: HomeTab) {
	return tab === "fresh" ? undefined : tab;
}
