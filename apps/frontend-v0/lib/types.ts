export interface ThreadFile {
  name: string
  mimetype: string
  url: string
}

export interface ThreadMessage {
  id: string
  author: {
    name: string
    initials: string
    color: string
  }
  message: string
  timestamp: string
  reactions?: number
  files?: ThreadFile[]
}

export interface SlackThread {
  id: string
  channelId: string
  ts: string
  author: {
    name: string
    initials: string
    color: string
  }
  channel: string
  channelColor: string
  message: string
  date: string
  replies: number
  reactions: number
  participants: number
  score: number
  hasFiles: boolean
  fileType?: "image" | "pdf" | "code" | "link"
  url?: string
  threadMessages?: ThreadMessage[]
}

export interface ChannelStat {
  name: string
  color: string
  count: number
  percentage: number
}

export interface ActivityPoint {
  date: string
  messages: number
  threads: number
}

export interface OverviewStats {
  totalMessages: number
  totalThreads: number
  totalFiles: number
  totalUsers: number
  messagesChange: number
  threadsChange: number
  filesChange: number
  usersChange: number
}

export interface DashboardData {
  tab: "top" | "week" | "month"
  workspaceUrl: string | null
  threads: SlackThread[]
  users: string[]
  channelStats: ChannelStat[]
  activityData: ActivityPoint[]
  overviewStats: OverviewStats
}
