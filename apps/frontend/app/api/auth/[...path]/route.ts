import { NextRequest, NextResponse } from "next/server"

import {
  INTERNAL_SECRET_HEADER,
  SESSION_COOKIE_NAME,
  SESSION_HEADER,
  backendBaseUrl,
  internalSecret,
  isSecureCookie,
} from "@/lib/server-auth"

const SESSION_MAX_AGE_SECONDS = 60 * 60 * 24 * 30

function isLambdaCrashBody(body: string): boolean {
  return (
    body.includes("LambdaError: RequestId") ||
    body.includes("Address already in use") ||
    body.includes("AddrInUse")
  )
}

function unavailableResponse(): NextResponse {
  return NextResponse.json(
    { ok: false, error: "Backend service unavailable. Please retry in a few seconds." },
    { status: 503 }
  )
}

function sanitizeResponseBody(body: unknown): unknown {
  if (!body || typeof body !== "object" || Array.isArray(body)) {
    return body
  }
  const { sessionToken: _ignored, ...rest } = body as Record<string, unknown>
  return rest
}

async function proxyAuth(
  request: NextRequest,
  path: string[],
  method: "GET" | "POST" | "PATCH"
): Promise<NextResponse> {
  const upstreamUrl = `${backendBaseUrl()}/api/auth/${path.join("/")}`
  const headers = new Headers()
  headers.set(INTERNAL_SECRET_HEADER, internalSecret())
  headers.set("Content-Type", "application/json")
  const session = request.cookies.get(SESSION_COOKIE_NAME)?.value
  if (session) {
    headers.set(SESSION_HEADER, session)
  }

  const hasBody = method !== "GET"
  let upstream: globalThis.Response
  try {
    upstream = await fetch(upstreamUrl, {
      method,
      headers,
      cache: "no-store",
      body: hasBody ? await request.text() : undefined,
    })
  } catch (err) {
    console.error("[api/auth proxy] upstream request failed", err)
    return unavailableResponse()
  }

  const contentType = upstream.headers.get("content-type") ?? "application/json"
  const bodyText = await upstream.text()
  if (isLambdaCrashBody(bodyText)) {
    console.error("[api/auth proxy] upstream lambda crash", {
      status: upstream.status,
      preview: bodyText.slice(0, 220),
    })
    return unavailableResponse()
  }
  const response = new NextResponse(bodyText, {
    status: upstream.status,
    headers: { "Content-Type": contentType },
  })

  const isLoginOrRegister =
    method === "POST" &&
    (path.join("/") === "login" || path.join("/") === "register")
  const isLogout = method === "POST" && path.join("/") === "logout"
  const isMe = method === "GET" && path.join("/") === "me"

  if (contentType.includes("application/json")) {
    let json: Record<string, unknown> = {}
    if (bodyText) {
      try {
        json = JSON.parse(bodyText) as Record<string, unknown>
      } catch (_err) {
        json = {}
      }
    }
    if (isLoginOrRegister && upstream.ok && typeof json.sessionToken === "string") {
      const res = NextResponse.json(sanitizeResponseBody(json), {
        status: upstream.status,
      })
      res.cookies.set({
        name: SESSION_COOKIE_NAME,
        value: json.sessionToken,
        httpOnly: true,
        secure: isSecureCookie(),
        sameSite: "lax",
        path: "/",
        maxAge: SESSION_MAX_AGE_SECONDS,
      })
      res.headers.set("Cache-Control", "no-store")
      return res
    }
  }

  if (isLogout || (isMe && upstream.status === 401)) {
    response.cookies.delete(SESSION_COOKIE_NAME)
  }

  response.headers.set("Cache-Control", "no-store")
  return response
}

export async function GET(
  request: NextRequest,
  context: { params: Promise<{ path: string[] }> }
): Promise<NextResponse> {
  const { path } = await context.params
  return proxyAuth(request, path ?? [], "GET")
}

export async function POST(
  request: NextRequest,
  context: { params: Promise<{ path: string[] }> }
): Promise<NextResponse> {
  const { path } = await context.params
  return proxyAuth(request, path ?? [], "POST")
}

export async function PATCH(
  request: NextRequest,
  context: { params: Promise<{ path: string[] }> }
): Promise<NextResponse> {
  const { path } = await context.params
  return proxyAuth(request, path ?? [], "PATCH")
}
