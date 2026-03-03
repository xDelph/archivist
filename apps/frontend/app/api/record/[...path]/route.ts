import { NextRequest, NextResponse } from "next/server"

import {
  INTERNAL_SECRET_HEADER,
  SESSION_COOKIE_NAME,
  SESSION_HEADER,
  backendBaseUrl,
  internalSecret,
} from "@/lib/server-auth"

function isLambdaCrashBody(body: string): boolean {
  return (
    body.includes("LambdaError: RequestId") ||
    body.includes("Address already in use") ||
    body.includes("AddrInUse")
  )
}

export async function GET(
  request: NextRequest,
  context: { params: Promise<{ path: string[] }> }
): Promise<NextResponse> {
  const { path } = await context.params
  const segments = path ?? []
  const query = request.nextUrl.search
  const upstreamUrl = `${backendBaseUrl()}/api/record/${segments.join("/")}${query}`

  const headers = new Headers()
  headers.set(INTERNAL_SECRET_HEADER, internalSecret())
  const session = request.cookies.get(SESSION_COOKIE_NAME)?.value
  if (session) {
    headers.set(SESSION_HEADER, session)
  }

  let upstream: globalThis.Response
  try {
    upstream = await fetch(upstreamUrl, {
      method: "GET",
      headers,
      cache: "no-store",
    })
  } catch (err) {
    console.error("[api/record proxy] upstream request failed", err)
    return NextResponse.json(
      { ok: false, error: "Backend service unavailable. Please retry in a few seconds." },
      { status: 503 }
    )
  }

  const contentType = upstream.headers.get("content-type") ?? "application/json"
  const text = await upstream.text()
  if (isLambdaCrashBody(text)) {
    console.error("[api/record proxy] upstream lambda crash", {
      status: upstream.status,
      preview: text.slice(0, 220),
    })
    return NextResponse.json(
      { ok: false, error: "Backend service unavailable. Please retry in a few seconds." },
      { status: 503 }
    )
  }
  const response = new NextResponse(text, {
    status: upstream.status,
    headers: {
      "Content-Type": contentType,
      "Cache-Control": "no-store",
    },
  })

  if (upstream.status === 401) {
    response.cookies.delete(SESSION_COOKIE_NAME)
  }

  return response
}
