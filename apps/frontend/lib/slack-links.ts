const TRAILING_URL_PUNCTUATION = /[)\]}\.,;!?]+$/u

function decodeSlackEntities(value: string): string {
  return value.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&")
}

function isUrlBoundary(previous: string | undefined): boolean {
  if (previous == null) return true
  return /\s|[([{'"`]/u.test(previous)
}

function findPlainUrlEnd(text: string, start: number): number {
  for (let index = start; index < text.length; index += 1) {
    if (/\s/u.test(text[index])) return index
  }
  return text.length
}

function cleanExtractedUrl(candidate: string): string {
  return candidate.trim().replace(TRAILING_URL_PUNCTUATION, "")
}

export function extractSlackUrlsInOrder(rawText: string): string[] {
  if (!rawText) return []

  const text = decodeSlackEntities(rawText)
  const urls: string[] = []

  let index = 0
  while (index < text.length) {
    const current = text[index]

    if (current === "<") {
      const close = text.indexOf(">", index + 1)
      if (close === -1) {
        index += 1
        continue
      }
      const inner = text.slice(index + 1, close)
      if (inner.includes("<")) {
        index += 1
        continue
      }
      if (inner.startsWith("http://") || inner.startsWith("https://")) {
        const pipe = inner.indexOf("|")
        const url = cleanExtractedUrl(pipe >= 0 ? inner.slice(0, pipe) : inner)
        if (url.startsWith("http://") || url.startsWith("https://")) {
          urls.push(url)
        }
      }
      index = close + 1
      continue
    }

    const isHttp = text.startsWith("http://", index)
    const isHttps = text.startsWith("https://", index)
    if ((isHttp || isHttps) && isUrlBoundary(text[index - 1])) {
      const end = findPlainUrlEnd(text, index)
      const url = cleanExtractedUrl(text.slice(index, end))
      if (url.startsWith("http://") || url.startsWith("https://")) {
        urls.push(url)
      }
      index = end
      continue
    }

    index += 1
  }

  return urls
}
