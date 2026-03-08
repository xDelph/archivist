export interface ApiHealth {
	service: string;
	version: string;
	workspace_mode?: string;
	repository_mode?: string;
	search_backend?: string;
}

const DEFAULT_API_BASE_URL = "http://127.0.0.1:4000";

export function resolveApiBaseUrl(baseUrl = import.meta.env.VITE_API_BASE_URL) {
	const candidate = baseUrl?.trim() || DEFAULT_API_BASE_URL;
	return candidate.replace(/\/+$/, "");
}

export function buildApiUrl(path: string, baseUrl = resolveApiBaseUrl()) {
	const normalizedPath = path.startsWith("/") ? path : `/${path}`;
	return `${baseUrl}${normalizedPath}`;
}

export async function fetchApiHealth() {
	const response = await fetch(buildApiUrl("/health"));
	if (!response.ok) {
		throw new Error(`Health check failed with status ${response.status}`);
	}

	return (await response.json()) as ApiHealth;
}
