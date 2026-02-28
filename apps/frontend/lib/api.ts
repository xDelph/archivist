import type {
  ActivityPoint,
  ChannelStat,
  DashboardData,
  OverviewStats,
  SlackThread,
  ThreadMessage,
} from "@/lib/types"

const API_BASE_URL = process.env.NEXT_PUBLIC_API_BASE_URL?.replace(/\/$/, "") ?? ""

const AUTHOR_COLORS = [
  "bg-chart-1",
  "bg-chart-2",
  "bg-chart-3",
  "bg-chart-4",
  "bg-chart-5",
  "bg-info",
]

const CHANNEL_COLORS = [
  "bg-chart-1/15 text-chart-1",
  "bg-chart-2/15 text-chart-2",
  "bg-chart-3/15 text-chart-3",
  "bg-chart-4/15 text-chart-4",
  "bg-chart-5/15 text-chart-5",
  "bg-info/15 text-info",
  "bg-warning/15 text-warning",
]

interface ApiThread {
  id: string
  channelId: string
  ts: string
  author: {
    name: string
    initials: string
    avatarUrl: string
  }
  channel: string
  message: string
  messageHtml: string
  searchText?: string
  date: string
  replies: number
  reactions: number
  participants: number
  score: number
  hasFiles: boolean
  url?: string
}

interface ApiChannelStat {
  name: string
  count: number
  percentage: number
}

interface ApiActivityPoint {
  date: string
  messages: number
  threads: number
}

interface ApiOverviewStats {
  totalMessages: number
  totalThreads: number
  totalFiles: number
  totalUsers: number
  messagesChange: number
  threadsChange: number
  filesChange: number
  usersChange: number
}

interface ApiThreadsResponse {
  tab: "top" | "week" | "month" | "recent"
  workspaceUrl: string | null
  threads: ApiThread[]
  users: string[]
  channelStats: ApiChannelStat[]
  activityData: ApiActivityPoint[]
  overviewStats: ApiOverviewStats
}

interface ApiThreadFile {
  name: string
  mimetype: string
  url: string
}

interface ApiReactionDetail {
  name: string
  emoji: string
  count: number
}

interface ApiThreadMessage {
  id: string
  ts: string
  author: {
    name: string
    initials: string
    avatarUrl: string
  }
  message: string
  messageHtml: string
  timestamp: string
  timestampIso: string
  reactions: number
  reactionDetails: ApiReactionDetail[]
  files: ApiThreadFile[]
}

interface ApiThreadResponse {
  channelId?: string
  channel?: string
  ts?: string
  messages: ApiThreadMessage[]
}

interface ThreadDetailResponse {
  channel: string | null
  messages: ThreadMessage[]
}

interface DashboardFetchOptions {
  tab: "top" | "week" | "month" | "recent"
  sort?: string
  period?: string
  channel?: string
  user?: string
  search?: string
  limit?: number
}

function buildUrl(path: string): string {
  return `${API_BASE_URL}${path}`
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms)
  })
}

function hashIndex(value: string, modulo: number): number {
  let hash = 0
  for (let i = 0; i < value.length; i += 1) {
    hash = (hash * 31 + value.charCodeAt(i)) >>> 0
  }
  return hash % modulo
}

function mapThread(apiThread: ApiThread): SlackThread {
  const authorColor = AUTHOR_COLORS[hashIndex(apiThread.author.name, AUTHOR_COLORS.length)]
  const channelColor =
    CHANNEL_COLORS[hashIndex(apiThread.channel, CHANNEL_COLORS.length)]
  return {
    id: apiThread.id,
    channelId: apiThread.channelId,
    ts: apiThread.ts,
    author: {
      name: apiThread.author.name,
      initials: apiThread.author.initials,
      color: authorColor,
      avatarUrl: apiThread.author.avatarUrl ?? "",
    },
    channel: apiThread.channel,
    channelColor,
    message: apiThread.message,
    messageHtml: apiThread.messageHtml,
    searchText: apiThread.searchText ?? "",
    date: apiThread.date,
    replies: apiThread.replies,
    reactions: apiThread.reactions,
    participants: apiThread.participants,
    score: apiThread.score,
    hasFiles: apiThread.hasFiles,
    url: apiThread.url,
  }
}

function mapChannelStats(stats: ApiChannelStat[]): ChannelStat[] {
  return stats.map((stat) => ({
    name: stat.name,
    count: stat.count,
    percentage: stat.percentage,
    color: CHANNEL_COLORS[hashIndex(stat.name, CHANNEL_COLORS.length)].split(" ")[0],
  }))
}

function mapActivityData(data: ApiActivityPoint[]): ActivityPoint[] {
  return data.map((point) => ({
    date: point.date,
    messages: point.messages,
    threads: point.threads,
  }))
}

function mapOverviewStats(stats: ApiOverviewStats): OverviewStats {
  return {
    totalMessages: stats.totalMessages,
    totalThreads: stats.totalThreads,
    totalFiles: stats.totalFiles,
    totalUsers: stats.totalUsers,
    messagesChange: stats.messagesChange,
    threadsChange: stats.threadsChange,
    filesChange: stats.filesChange,
    usersChange: stats.usersChange,
  }
}

function ensureOk(response: Response): void {
  if (!response.ok) {
    throw new Error(`API request failed (${response.status})`)
  }
}

async function fetchJsonWithRetry<T>(url: string, attempts = 3): Promise<T> {
  let lastError: unknown = null

  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    try {
      const response = await fetch(url, { cache: "no-store" })
      if (!response.ok) {
        const bodyPreview = (await response.text()).slice(0, 140).replace(/\s+/g, " ").trim()
        throw new Error(
          bodyPreview
            ? `API request failed (${response.status}): ${bodyPreview}`
            : `API request failed (${response.status})`
        )
      }
      return (await response.json()) as T
    } catch (error) {
      lastError = error
      if (attempt < attempts) {
        await sleep(150 * attempt)
        continue
      }
    }
  }

  throw lastError instanceof Error ? lastError : new Error("API request failed")
}

export async function fetchDashboardData(
  options: DashboardFetchOptions
): Promise<DashboardData> {
  const params = new URLSearchParams({
    tab: options.tab,
    limit: String(options.limit ?? 200),
  })
  if (options.sort) params.set("sort", options.sort)
  if (options.period) params.set("period", options.period)
  if (options.channel) params.set("channel", options.channel.replace(/^#/, ""))
  if (options.user) params.set("user", options.user)
  if (options.search) params.set("search", options.search)

  const payload = await fetchJsonWithRetry<ApiThreadsResponse>(
    buildUrl(`/api/record/threads?${params.toString()}`)
  )
  return {
    tab: payload.tab,
    workspaceUrl: payload.workspaceUrl ?? null,
    threads: payload.threads.map(mapThread),
    users: payload.users ?? [],
    channelStats: mapChannelStats(payload.channelStats),
    activityData: mapActivityData(payload.activityData),
    overviewStats: mapOverviewStats(payload.overviewStats),
  }
}

export async function fetchThreadMessages(
  channelId: string,
  ts: string,
  search?: string
): Promise<ThreadMessage[]> {
  const qs = new URLSearchParams({
    channel_id: channelId,
    ts,
  })
  if (search) {
    qs.set("search", search)
  }
  const payload = await fetchJsonWithRetry<ApiThreadResponse>(
    buildUrl(`/api/record/thread?${qs.toString()}`)
  )
  return payload.messages.map((message) => ({
    id: message.id,
    ts: message.ts,
    author: {
      name: message.author.name,
      initials: message.author.initials,
      color: AUTHOR_COLORS[hashIndex(message.author.name, AUTHOR_COLORS.length)],
      avatarUrl: message.author.avatarUrl ?? "",
    },
    message: message.message,
    messageHtml: message.messageHtml,
    timestamp: message.timestamp,
    timestampIso: message.timestampIso,
    reactions: message.reactions,
    reactionDetails: (message.reactionDetails ?? []).map((reaction) => ({
      name: reaction.name,
      emoji: reaction.emoji,
      count: reaction.count,
    })),
    files: message.files.map((file) => ({
      name: file.name,
      mimetype: file.mimetype,
      url: file.url,
    })),
  }))
}

export async function fetchThreadDetail(
  channelId: string,
  ts: string,
  search?: string
): Promise<ThreadDetailResponse> {
  const qs = new URLSearchParams({
    channel_id: channelId,
    ts,
  })
  if (search) {
    qs.set("search", search)
  }
  const payload = await fetchJsonWithRetry<ApiThreadResponse>(
    buildUrl(`/api/record/thread?${qs.toString()}`)
  )
  return {
    channel: payload.channel ?? null,
    messages: payload.messages.map((message) => ({
      id: message.id,
      ts: message.ts,
      author: {
        name: message.author.name,
        initials: message.author.initials,
        color: AUTHOR_COLORS[hashIndex(message.author.name, AUTHOR_COLORS.length)],
        avatarUrl: message.author.avatarUrl ?? "",
      },
      message: message.message,
      messageHtml: message.messageHtml,
      timestamp: message.timestamp,
      timestampIso: message.timestampIso,
      reactions: message.reactions,
      reactionDetails: (message.reactionDetails ?? []).map((reaction) => ({
        name: reaction.name,
        emoji: reaction.emoji,
        count: reaction.count,
      })),
      files: message.files.map((file) => ({
        name: file.name,
        mimetype: file.mimetype,
        url: file.url,
      })),
    })),
  }
}
