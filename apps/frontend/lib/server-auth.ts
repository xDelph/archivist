export const SESSION_COOKIE_NAME = "archivist_session"
export const INTERNAL_SECRET_HEADER = "x-archivist-internal-secret"
export const SESSION_HEADER = "x-archivist-session"

export function backendBaseUrl(): string {
  const raw =
    process.env.BACKEND_API_BASE_URL ??
    process.env.NEXT_PUBLIC_API_BASE_URL ??
    ""
  const normalized = raw.trim().replace(/\/$/, "")
  if (!normalized) {
    throw new Error("Missing BACKEND_API_BASE_URL or NEXT_PUBLIC_API_BASE_URL")
  }
  return normalized
}

export function internalSecret(): string {
  const value = (process.env.FRONTEND_BACKEND_SHARED_SECRET ?? "").trim()
  if (!value) {
    throw new Error("Missing FRONTEND_BACKEND_SHARED_SECRET")
  }
  return value
}

export function isSecureCookie(): boolean {
  return process.env.NODE_ENV === "production"
}
