"use client"

import { useCallback, useEffect, useState } from "react"
import { Archive, ExternalLink, LoaderCircle, Moon, Sun, Rows3, Rows2 } from "lucide-react"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { StatCard } from "@/components/stat-card"
import { ThreadCard } from "@/components/thread-card"
import { ActivityChart } from "@/components/activity-chart"
import { ChannelSidebar } from "@/components/channel-sidebar"
import { SearchBar } from "@/components/search-bar"
import { fetchDashboardData, fetchThreadMessages } from "@/lib/api"
import type { DashboardData, SlackThread, ThreadMessage } from "@/lib/types"

type DashboardTab = "top" | "week" | "month"
type ThemePref = "dark" | "light"
type DensityPref = "normal" | "compact"

const THEME_KEY = "archivist_theme"
const DENSITY_KEY = "archivist_density"

const EMPTY_DASHBOARD: DashboardData = {
  tab: "top",
  workspaceUrl: null,
  threads: [],
  users: [],
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

function parseTab(value: string | null): DashboardTab {
  if (value === "week" || value === "month") {
    return value
  }
  return "top"
}

function parseSort(value: string | null): string {
  if (value === "date" || value === "reactions" || value === "replies") {
    return value
  }
  return "score"
}

function parsePeriod(value: string | null): string {
  if (value === "7d" || value === "30d") {
    return value
  }
  return "all"
}

export default function ArchivistDashboard() {
  const [query, setQuery] = useState("")
  const [debouncedQuery, setDebouncedQuery] = useState("")
  const [sortBy, setSortBy] = useState("score")
  const [period, setPeriod] = useState("all")
  const [selectedUser, setSelectedUser] = useState<string | null>(null)
  const [selectedChannel, setSelectedChannel] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<DashboardTab>("top")
  const [dashboard, setDashboard] = useState<DashboardData | null>(null)
  const [theme, setTheme] = useState<ThemePref>("dark")
  const [density, setDensity] = useState<DensityPref>("normal")
  const [loading, setLoading] = useState(false)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [ready, setReady] = useState(false)
  const [reloadToken, setReloadToken] = useState(0)

  useEffect(() => {
    const params = new URLSearchParams(window.location.search)
    setActiveTab(parseTab(params.get("tab")))
    setSortBy(parseSort(params.get("sort")))
    setPeriod(parsePeriod(params.get("period")))
    setSelectedUser(params.get("user"))
    setSelectedChannel(params.get("channel"))
    setQuery(params.get("search") ?? "")
    setDebouncedQuery(params.get("search") ?? "")

    try {
      const storedTheme = localStorage.getItem(THEME_KEY)
      if (storedTheme === "dark" || storedTheme === "light") {
        setTheme(storedTheme)
      }
      const storedDensity = localStorage.getItem(DENSITY_KEY)
      if (storedDensity === "normal" || storedDensity === "compact") {
        setDensity(storedDensity)
      }
    } catch (_err) {}

    setReady(true)
  }, [])

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      setDebouncedQuery(query.trim())
    }, 300)
    return () => {
      window.clearTimeout(timeout)
    }
  }, [query])

  useEffect(() => {
    if (!ready) return
    const root = document.documentElement
    root.classList.toggle("theme-light", theme === "light")
    try {
      localStorage.setItem(THEME_KEY, theme)
    } catch (_err) {}
  }, [ready, theme])

  useEffect(() => {
    if (!ready) return
    const root = document.documentElement
    root.classList.toggle("compact-mode", density === "compact")
    try {
      localStorage.setItem(DENSITY_KEY, density)
    } catch (_err) {}
  }, [ready, density])

  useEffect(() => {
    if (!ready) return

    const params = new URLSearchParams()
    if (activeTab !== "top") params.set("tab", activeTab)
    if (sortBy !== "score") params.set("sort", sortBy)
    if (period !== "all") params.set("period", period)
    if (selectedUser) params.set("user", selectedUser)
    if (selectedChannel) params.set("channel", selectedChannel)
    if (debouncedQuery) params.set("search", debouncedQuery)

    const next = params.toString()
    const url = next ? `${window.location.pathname}?${next}` : window.location.pathname
    window.history.replaceState(null, "", url)
  }, [ready, activeTab, sortBy, period, selectedUser, selectedChannel, debouncedQuery])

  useEffect(() => {
    if (!ready) return

    let cancelled = false
    setLoading(true)
    setLoadError(null)

    fetchDashboardData({
      tab: activeTab,
      sort: sortBy,
      period,
      user: selectedUser ?? undefined,
      channel: selectedChannel ?? undefined,
      search: debouncedQuery || undefined,
      limit: 200,
    })
      .then((nextDashboard) => {
        if (cancelled) return
        setDashboard(nextDashboard)
      })
      .catch(() => {
        if (cancelled) return
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
  }, [
    ready,
    activeTab,
    sortBy,
    period,
    selectedUser,
    selectedChannel,
    debouncedQuery,
    reloadToken,
  ])

  const currentDashboard = dashboard ?? EMPTY_DASHBOARD
  const currentThreads = currentDashboard.threads

  const loadThreadMessages = useCallback(
    async (thread: SlackThread): Promise<ThreadMessage[]> =>
      fetchThreadMessages(thread.channelId, thread.ts, debouncedQuery || undefined),
    [debouncedQuery]
  )

  function retryCurrentTab(): void {
    setReloadToken((value) => value + 1)
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

          <Tabs
            value={activeTab}
            onValueChange={(value) => setActiveTab(value as DashboardTab)}
            className="ml-4 hidden sm:flex"
          >
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

          <div className="ml-auto flex items-center gap-2">
            <button
              type="button"
              onClick={() => setDensity((prev) => (prev === "compact" ? "normal" : "compact"))}
              className="hidden items-center gap-1 rounded-md border border-border bg-secondary px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground sm:inline-flex"
            >
              {density === "compact" ? <Rows2 className="size-3.5" /> : <Rows3 className="size-3.5" />}
              {density === "compact" ? "Compact" : "Normal"}
            </button>
            <button
              type="button"
              onClick={() => setTheme((prev) => (prev === "dark" ? "light" : "dark"))}
              className="hidden items-center gap-1 rounded-md border border-border bg-secondary px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground sm:inline-flex"
            >
              {theme === "dark" ? <Moon className="size-3.5" /> : <Sun className="size-3.5" />}
              {theme === "dark" ? "Dark" : "Light"}
            </button>

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
            period={period}
            onPeriodChange={setPeriod}
            users={currentDashboard.users}
            selectedUser={selectedUser}
            onUserChange={setSelectedUser}
          />
        </section>

        <section className="flex flex-col gap-2">
          {loading && currentThreads.length === 0 && (
            <div className="flex items-center justify-center gap-2 rounded-lg border border-border bg-card py-16 text-sm text-muted-foreground">
              <LoaderCircle className="size-4 animate-spin" />
              Loading dashboard...
            </div>
          )}

          {loadError && currentThreads.length === 0 && (
            <div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-border bg-card py-16 text-center">
              <p className="text-sm text-muted-foreground">{loadError}</p>
              <button onClick={retryCurrentTab} className="text-xs text-primary hover:underline">
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
                  setDebouncedQuery("")
                  setPeriod("all")
                  setSelectedUser(null)
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
              density={density}
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
