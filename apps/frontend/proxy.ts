import { NextRequest, NextResponse } from "next/server"

import { SESSION_COOKIE_NAME } from "@/lib/server-auth"

const PUBLIC_PATHS = new Set([
  "/login",
  "/register",
  "/forgot-password",
  "/reset-password",
])

function isPublicPath(pathname: string): boolean {
  if (PUBLIC_PATHS.has(pathname)) {
    return true
  }
  if (pathname.startsWith("/api/")) {
    return true
  }
  if (pathname.startsWith("/_next/")) {
    return true
  }
  if (pathname === "/favicon.ico" || pathname.startsWith("/favicon")) {
    return true
  }
  if (pathname.startsWith("/icon") || pathname.startsWith("/apple-icon")) {
    return true
  }
  return false
}

export function proxy(request: NextRequest): NextResponse {
  const { pathname } = request.nextUrl
  const hasSession = Boolean(request.cookies.get(SESSION_COOKIE_NAME)?.value)
  const publicPath = isPublicPath(pathname)

  if (!hasSession && !publicPath) {
    const target = new URL("/login", request.url)
    target.searchParams.set("next", pathname)
    return NextResponse.redirect(target)
  }

  if (hasSession && (pathname === "/login" || pathname === "/register")) {
    return NextResponse.redirect(new URL("/", request.url))
  }

  return NextResponse.next()
}

export const config = {
  matcher: ["/((?!_next/static|_next/image).*)"],
}
