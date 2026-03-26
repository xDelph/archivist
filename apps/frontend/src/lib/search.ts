import type { SearchResponse, SearchResult } from "@/lib/api";

export const SEARCH_PAGE_SIZE = 20;

export function flattenSearchPages(
	pages: Pick<SearchResponse, "items">[],
): SearchResult[] {
	return pages.flatMap((page) => page.items);
}
