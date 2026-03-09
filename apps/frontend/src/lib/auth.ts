import { isApiErrorWithStatus } from "@/lib/api";
import { authQueries } from "@/lib/queries";

/**
 * @deprecated Use `authQueries.me()` from `@/lib/queries` directly.
 */
export function currentUserQueryOptions() {
	return authQueries.me();
}

export function isUnauthorizedError(error: unknown) {
	return isApiErrorWithStatus(error, 401);
}
