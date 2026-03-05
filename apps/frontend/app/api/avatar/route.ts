import { NextRequest } from "next/server"

const AVATAR_PROXY_HOST_SUFFIXES = ["slack-edge.com", "gravatar.com"]
const AVATAR_MAX_AGE_SECONDS = 60 * 60 * 24

function isAllowedAvatarHost(hostname: string): boolean {
  const normalized = hostname.toLowerCase()
  return AVATAR_PROXY_HOST_SUFFIXES.some(
    (suffix) => normalized === suffix || normalized.endsWith(`.${suffix}`)
  )
}

export async function GET(request: NextRequest): Promise<Response> {
  const rawUrl = request.nextUrl.searchParams.get("u")
  if (!rawUrl) {
    return new Response("Missing avatar URL", { status: 400 })
  }

  let parsedUrl: URL
  try {
    parsedUrl = new URL(rawUrl)
  } catch (_err) {
    return new Response("Invalid avatar URL", { status: 400 })
  }

  if (!["http:", "https:"].includes(parsedUrl.protocol)) {
    return new Response("Unsupported avatar URL protocol", { status: 400 })
  }

  if (!isAllowedAvatarHost(parsedUrl.hostname)) {
    return new Response("Avatar host not allowed", { status: 400 })
  }

  try {
    const upstream = await fetch(parsedUrl.toString(), {
      cache: "force-cache",
      next: { revalidate: AVATAR_MAX_AGE_SECONDS },
      headers: {
        accept: "image/avif,image/webp,image/apng,image/*,*/*;q=0.8",
      },
    })

    if (!upstream.ok || !upstream.body) {
      return new Response("Avatar unavailable", { status: upstream.status || 502 })
    }

    const headers = new Headers()
    const contentType = upstream.headers.get("content-type")
    if (contentType) {
      headers.set("Content-Type", contentType)
    }
    const cacheControl = `public, max-age=${AVATAR_MAX_AGE_SECONDS}, stale-while-revalidate=${AVATAR_MAX_AGE_SECONDS * 7}`
    headers.set("Cache-Control", cacheControl)

    return new Response(upstream.body, { status: 200, headers })
  } catch (_err) {
    return new Response("Avatar fetch failed", { status: 502 })
  }
}
