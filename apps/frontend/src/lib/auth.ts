import { fetchCurrentUser, isApiErrorWithStatus } from "@/lib/api";
import { queryOptions } from "@tanstack/react-query";

export function currentUserQueryOptions() {
	return queryOptions({
		queryKey: ["auth", "me"],
		queryFn: fetchCurrentUser,
		staleTime: 60_000,
		retry: false,
	});
}

export function isUnauthorizedError(error: unknown) {
	return isApiErrorWithStatus(error, 401);
}
