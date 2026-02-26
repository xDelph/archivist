"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { Archive, ExternalLink, LoaderCircle } from "lucide-react"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { StatCard } from "@/components/stat-card"
import { ThreadCard } from "@/components/thread-card"
import { ActivityChart } from "@/components/activity-chart"
import { ChannelSidebar } from "@/components/channel-sidebar"
import { SearchBar } from "@/components/search-bar"
import { fetchDashboardData, fetchThreadMessages } from "@/lib/api"
import type { DashboardData, SlackThread, ThreadMessage } from "@/lib/types"

type DashboardTab = "top" | "week" | "month"

const EMPTY_DASHBOARD: DashboardData = {
  tab: "top",
  workspaceUrl: null,
  threads: [],
  channelStats: [],
  activityData: [],
  overviewStats: {
    totalMessages: 0,
    totalThreads: 0,
    totalFiles: 0,
    totalUsers: 0,
    messagesChange: 0,
    threadsChange: 0,
    filesChange: 0,
    usersChange: 0,
  },
}

function sortThreads(threads: SlackThread[], sortBy: string): SlackThread[] {
  return [...threads].sort((a, b) => {
    switch (sortBy) {
      case "replies":
        return b.replies - a.replies
      case "reactions":
        return b.reactions - a.reactions
      case "date":
        return Number.parseFloat(b.ts) - Number.parseFloat(a.ts)
      default:
        return b.score - a.score
    }
  })
}

function filterThreads(
  threads: SlackThread[],
  query: string,
  channel: string | null
): SlackThread[] {
  return threads.filter((thread) => {
    const normalized = query.toLowerCase()
    const matchesQuery =
      !normalized ||
      thread.message.toLowerCase().includes(normalized) ||
      thread.author.name.toLowerCase().includes(normalized) ||
      thread.channel.toLowerCase().includes(normalized)
    const matchesChannel = !channel || thread.channel === channel
    return matchesQuery && matchesChannel
  })
}

export default function ArchivistDashboard() {
  const [query, setQuery] = useState("")
  const [sortBy, setSortBy] = useState("score")
  const [selectedChannel, setSelectedChannel] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<DashboardTab>("top")
  const [dashboards, setDashboards] = useState<Partial<Record<DashboardTab, DashboardData>>>({})
  const [loading, setLoading] = useState(false)
  const [loadError, setLoadError] = useState<string | null>(null)

  useEffect(() => {
    if (dashboards[activeTab]) {
      return
    }

    let cancelled = false
    setLoading(true)
    setLoadError(null)

    fetchDashboardData(activeTab)
      .then((dashboard) => {
        if (cancelled) {
          return
        }
        setDashboards((prev) => ({ ...prev, [activeTab]: dashboard }))
      })
      .catch(() => {
        if (cancelled) {
          return
        }
        setLoadError("Failed to load dashboard data.")
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false)
        }
      })

    return () => {
      cancelled = true
    }
  }, [activeTab, dashboards])

  const currentDashboard = dashboards[activeTab] ?? EMPTY_DASHBOARD
  const currentThreads = useMemo(() => {
    const filtered = filterThreads(currentDashboard.threads, query, selectedChannel)
    return sortThreads(filtered, sortBy)
  }, [currentDashboard.threads, query, selectedChannel, sortBy])

  const loadThreadMessages = useCallback(
    async (thread: SlackThread): Promise<ThreadMessage[]> =>
      fetchThreadMessages(thread.channelId, thread.ts),
    []
  )

  function retryCurrentTab(): void {
    setDashboards((prev) => {
      const next = { ...prev }
      delete next[activeTab]
      return next
    })
    setLoadError(null)
  }

  return (
    <div className="min-h-screen bg-background">
      <header className="sticky top-0 z-30 border-b border-border bg-background/80 backdrop-blur-md">
        <div className="mx-auto flex h-14 max-w-screen-2xl items-center gap-4 px-4 lg:px-6">
          <div className="flex items-center gap-2">
            <div className="flex size-8 items-center justify-center rounded-lg bg-primary">
              <Archive className="size-4 text-primary-foreground" />
            </div>
            <span className="text-base font-semibold text-foreground">Archivist</span>
          </div>

          <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as DashboardTab)} className="ml-4 hidden sm:flex">
            <TabsList className="h-8 bg-secondary">
              <TabsTrigger value="top" className="px-3 text-xs">
                Top Threads
              </TabsTrigger>
              <TabsTrigger value="week" className="px-3 text-xs">
                This Week
              </TabsTrigger>
              <TabsTrigger value="month" className="px-3 text-xs">
                This Month
              </TabsTrigger>
            </TabsList>
          </Tabs>

          <div className="ml-auto flex items-center gap-3">
            {currentDashboard.workspaceUrl && (
              <a
                href={
                  currentDashboard.workspaceUrl.startsWith("http")
                    ? currentDashboard.workspaceUrl
                    : `https://${currentDashboard.workspaceUrl}`
                }
                target="_blank"
                rel="noopener noreferrer"
                className="hidden items-center gap-1.5 text-xs text-muted-foreground transition-colors hover:text-foreground sm:flex"
              >
                Open Slack
                <ExternalLink className="size-3" />
              </a>
            )}
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-screen-2xl px-4 py-6 lg:px-6">
        <div className="mb-4 sm:hidden">
          <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as DashboardTab)}>
            <TabsList className="w-full bg-secondary">
              <TabsTrigger value="top" className="flex-1 text-xs">
                Top Threads
              </TabsTrigger>
              <TabsTrigger value="week" className="flex-1 text-xs">
                Week
              </TabsTrigger>
              <TabsTrigger value="month" className="flex-1 text-xs">
                Month
              </TabsTrigger>
            </TabsList>
          </Tabs>
        </div>

        <section className="mb-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
          <StatCard
            title="Total Messages"
            value={currentDashboard.overviewStats.totalMessages.toLocaleString()}
            change={currentDashboard.overviewStats.messagesChange}
            icon="messages"
          />
          <StatCard
            title="Threads"
            value={currentDashboard.overviewStats.totalThreads.toLocaleString()}
            change={currentDashboard.overviewStats.threadsChange}
            icon="threads"
          />
          <StatCard
            title="Files Archived"
            value={currentDashboard.overviewStats.totalFiles.toLocaleString()}
            change={currentDashboard.overviewStats.filesChange}
            icon="files"
          />
          <StatCard
            title="Active Users"
            value={currentDashboard.overviewStats.totalUsers.toLocaleString()}
            change={currentDashboard.overviewStats.usersChange}
            icon="users"
          />
        </section>

        <section className="mb-6 grid gap-4 lg:grid-cols-[1fr_260px]">
          <ActivityChart data={currentDashboard.activityData} />
          <div className="hidden lg:block">
            <ChannelSidebar
              channels={currentDashboard.channelStats}
              selected={selectedChannel}
              onSelect={setSelectedChannel}
            />
          </div>
        </section>

        <div className="mb-4 lg:hidden">
          <ChannelSidebar
            channels={currentDashboard.channelStats}
            selected={selectedChannel}
            onSelect={setSelectedChannel}
          />
        </div>

        <section className="mb-4">
          <SearchBar
            query={query}
            onQueryChange={setQuery}
            sortBy={sortBy}
            onSortChange={setSortBy}
          />
        </section>

        <section className="flex flex-col gap-2">
          {loading && currentDashboard.threads.length === 0 && (
            <div className="flex items-center justify-center gap-2 rounded-lg border border-border bg-card py-16 text-sm text-muted-foreground">
              <LoaderCircle className="size-4 animate-spin" />
              Loading dashboard...
            </div>
          )}

          {loadError && currentDashboard.threads.length === 0 && (
            <div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-border bg-card py-16 text-center">
              <p className="text-sm text-muted-foreground">{loadError}</p>
              <button
                onClick={retryCurrentTab}
                className="text-xs text-primary hover:underline"
              >
                Retry
              </button>
            </div>
          )}

          {!loading && !loadError && currentThreads.length === 0 && (
            <div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-border bg-card py-16 text-center">
              <p className="text-sm text-muted-foreground">
                No threads found matching your filters.
              </p>
              <button
                onClick={() => {
                  setQuery("")
                  setSelectedChannel(null)
                }}
                className="text-xs text-primary hover:underline"
              >
                Clear filters
              </button>
            </div>
          )}

          {currentThreads.map((thread, index) => (
            <ThreadCard
              key={thread.id}
              thread={thread}
              rank={index + 1}
              onLoadThreadMessages={loadThreadMessages}
            />
          ))}
        </section>

        <footer className="mt-8 border-t border-border pt-4 text-center text-xs text-muted-foreground">
          Archivist archives all messages from your Slack workspace.
          <br />
          Showing {currentThreads.length} threads sorted by {sortBy}.
        </footer>
      </main>
    </div>
  )
}
