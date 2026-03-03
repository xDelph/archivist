import { NextRequest, NextResponse } from "next/server"

interface LinkPreviewPayload {
  url: string
  title: string | null
  description: string | null
  image: string | null
  siteName: string | null
}

interface CachedPreview {
  expiresAt: number
  payload: LinkPreviewPayload | null
}

const CACHE_TTL_MS = 1000 * 60 * 60 * 6
const FETCH_TIMEOUT_MS = 2500
const MAX_HTML_BYTES = 200_000
const previewCache = new Map<string, CachedPreview>()

function isPrivateHostname(hostname: string): boolean {
  const host = hostname.trim().toLowerCase()
  if (!host) return true
  if (host === "localhost" || host === "::1" || host.endsWith(".local")) return true
  if (/^127\./u.test(host) || /^0\./u.test(host) || /^10\./u.test(host)) return true
  if (/^192\.168\./u.test(host)) return true
  const match172 = /^172\.(\d{1,3})\./u.exec(host)
  if (match172) {
    const secondOctet = Number(match172[1])
    if (secondOctet >= 16 && secondOctet <= 31) return true
  }
  return false
}

function decodeHtmlEntities(value: string): string {
  return value
    .replace(/&amp;/gi, "&")
    .replace(/&quot;/gi, '"')
    .replace(/&#39;/gi, "'")
    .replace(/&lt;/gi, "<")
    .replace(/&gt;/gi, ">")
}

function parseAttributes(tag: string): Record<string, string> {
  const attrs: Record<string, string> = {}
  const attrRegex = /([a-zA-Z:-]+)\s*=\s*("([^"]*)"|'([^']*)'|([^\s"'>]+))/gu
  for (const match of tag.matchAll(attrRegex)) {
    const key = match[1].toLowerCase()
    const value = match[3] ?? match[4] ?? match[5] ?? ""
    attrs[key] = decodeHtmlEntities(value.trim())
  }
  return attrs
}

function pickMetaContent(html: string, keys: string[]): string | null {
  const keySet = new Set(keys.map((key) => key.toLowerCase()))
  const metaRegex = /<meta\b[^>]*>/giu
  for (const match of html.matchAll(metaRegex)) {
    const tag = match[0]
    const attrs = parseAttributes(tag)
    const property = attrs.property?.toLowerCase()
    const name = attrs.name?.toLowerCase()
    if ((property && keySet.has(property)) || (name && keySet.has(name))) {
      const content = attrs.content?.trim()
      if (content) return content
    }
  }
  return null
}

function pickTitle(html: string): string | null {
  const titleMatch = html.match(/<title[^>]*>([\s\S]*?)<\/title>/iu)
  if (!titleMatch) return null
  const value = decodeHtmlEntities(titleMatch[1].replace(/\s+/gu, " ").trim())
  return value || null
}

function toAbsoluteUrl(candidate: string | null, baseUrl: string): string | null {
  if (!candidate) return null
  try {
    return new URL(candidate, baseUrl).toString()
  } catch (_err) {
    return null
  }
}

async function fetchPreview(url: string): Promise<LinkPreviewPayload | null> {
  const response = await fetch(url, {
    redirect: "follow",
    signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
    headers: {
      "user-agent":
        "Mozilla/5.0 (compatible; ArchivistLinkPreview/1.0; +https://archivist.delalonde.dev)",
      accept: "text/html,application/xhtml+xml",
      range: "bytes=0-65535",
    },
  })
  if (!response.ok) return null

  const contentType = response.headers.get("content-type") ?? ""
  if (!contentType.toLowerCase().includes("text/html")) return null

  const htmlRaw = (await response.text()).slice(0, MAX_HTML_BYTES)
  const lowered = htmlRaw.toLowerCase()
  const headEnd = lowered.indexOf("</head>")
  const html =
    headEnd >= 0 ? htmlRaw.slice(0, headEnd + "</head>".length) : htmlRaw.slice(0, 65_535)
  const title = pickMetaContent(html, ["og:title", "twitter:title"]) ?? pickTitle(html)
  const description =
    pickMetaContent(html, ["og:description", "twitter:description", "description"]) ?? null
  const image = toAbsoluteUrl(
    pickMetaContent(html, ["og:image", "twitter:image", "twitter:image:src"]),
    response.url,
  )
  const siteName = pickMetaContent(html, ["og:site_name"]) ?? null

  if (!title && !description && !image) return null
  return {
    url,
    title,
    description,
    image,
    siteName,
  }
}

export async function GET(request: NextRequest): Promise<NextResponse> {
  const rawUrl = request.nextUrl.searchParams.get("url")?.trim()
  if (!rawUrl) {
    return NextResponse.json({ error: "missing url" }, { status: 400 })
  }

  let parsed: URL
  try {
    parsed = new URL(rawUrl)
  } catch (_err) {
    return NextResponse.json({ error: "invalid url" }, { status: 400 })
  }

  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    return NextResponse.json({ error: "unsupported url protocol" }, { status: 400 })
  }
  if (isPrivateHostname(parsed.hostname)) {
    return NextResponse.json({ error: "unsupported hostname" }, { status: 400 })
  }

  const cacheKey = parsed.toString()
  const now = Date.now()
  const cached = previewCache.get(cacheKey)
  if (cached && cached.expiresAt > now) {
    return NextResponse.json(cached.payload, { status: 200 })
  }

  try {
    const payload = await fetchPreview(cacheKey)
    previewCache.set(cacheKey, {
      expiresAt: now + CACHE_TTL_MS,
      payload,
    })
    return NextResponse.json(payload, { status: 200 })
  } catch (_err) {
    previewCache.set(cacheKey, {
      expiresAt: now + CACHE_TTL_MS,
      payload: null,
    })
    return NextResponse.json(null, { status: 200 })
  }
}
