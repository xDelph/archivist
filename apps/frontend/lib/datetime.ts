export function formatMessageTimestamp(timestampIso: string, ts: string): string {
  const parsed = parseMessageDate(timestampIso, ts)
  if (!parsed) {
    return ""
  }

  return new Intl.DateTimeFormat(undefined, {
    day: "2-digit",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(parsed)
}

function parseMessageDate(timestampIso: string, ts: string): Date | null {
  if (timestampIso) {
    const parsedIso = new Date(timestampIso)
    if (!Number.isNaN(parsedIso.getTime())) {
      return parsedIso
    }
  }

  const seconds = Number.parseFloat(ts)
  if (!Number.isFinite(seconds)) {
    return null
  }

  const parsedTs = new Date(seconds * 1000)
  if (Number.isNaN(parsedTs.getTime())) {
    return null
  }
  return parsedTs
}
