"use client"

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { Archive, ExternalLink, LoaderCircle, Moon, Sun, Rows3, Rows2 } from "lucide-react"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { StatCard } from "@/components/stat-card"
import { ThreadCard } from "@/components/thread-card"
import { ActivityChart } from "@/components/activity-chart"
import { ChannelSidebar } from "@/components/channel-sidebar"
import { SearchBar } from "@/components/search-bar"
import { fetchDashboardData, fetchThreadMessages } from "@/lib/api"
import type { DashboardData, SlackThread, ThreadMessage } from "@/lib/types"

type DashboardTab = "top" | "week" | "month" | "recent"
type ThemePref = "dark" | "light"
type DensityPref = "normal" | "compact"

const THEME_KEY = "archivist_theme"
const DENSITY_KEY = "archivist_density"
const STATS_VISIBILITY_KEY = "archivist_stats_visibility"
const SEARCH_DEBOUNCE_MS = 250
const TABS: DashboardTab[] = ["top", "week", "month", "recent"]

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
  if (value === "week" || value === "month" || value === "recent") {
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

function parseOptionalFilter(value: string | null): string | null {
  if (!value) return null
  const normalized = value.trim()
  return normalized.length > 0 ? normalized : null
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}

function normalizeSearchValue(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
}

function stripHtmlForSearch(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&nbsp;/gi, " ")
    .replace(/&amp;/gi, "&")
    .replace(/&lt;/gi, "<")
    .replace(/&gt;/gi, ">")
    .replace(/\s+/g, " ")
    .trim()
}

function highlightHtml(html: string, term: string): string {
  if (!term) return html
  const lowerTerm = term.toLowerCase()
  const regex = new RegExp(escapeRegex(term), "gi")
  return html.replace(/(<[^>]+>)|([^<]+)/g, (_all, tag: string, text: string) => {
    if (tag) {
      if (tag.toLowerCase().startsWith("<a ")) {
        const lowerTag = tag.toLowerCase()
        const hasAttrMatch = (attr: string): boolean => {
          const marker = `${attr}="`
          const idx = lowerTag.indexOf(marker)
          if (idx === -1) return false
          const after = lowerTag.slice(idx + marker.length)
          const end = after.indexOf('"')
          return end !== -1 && after.slice(0, end).includes(lowerTerm)
        }
        if (hasAttrMatch("href") || hasAttrMatch("data-src")) {
          return tag.replace("<a ", '<a class="url-highlight" ')
        }
      }
      return tag
    }
    return text.replace(regex, (match) => `<mark class="search-highlight">${match}</mark>`)
  })
}

function InitialLoadingShell() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-4">
      <div className="select-none text-center">
        <h1 className="text-5xl font-semibold tracking-tight text-foreground sm:text-6xl">
          Archivist
        </h1>
        <p className="mt-2 text-base text-muted-foreground sm:text-lg">Slack message archiver</p>
        <div className="mt-8 inline-flex items-center gap-2 text-sm text-muted-foreground">
          <span className="size-2.5 animate-pulse rounded-full bg-orange-400" />
          loading
        </div>
      </div>
    </div>
  )
}

export default function ArchivistDashboard() {
  const [query, setQuery] = useState("")
  const [debouncedQuery, setDebouncedQuery] = useState("")
  const [sortBy, setSortBy] = useState("score")
  const [period, setPeriod] = useState("all")
  const [selectedUser, setSelectedUser] = useState<string | null>(null)
  const [selectedChannel, setSelectedChannel] = useState<string | null>(null)
  const [expandedThreadId, setExpandedThreadId] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<DashboardTab>("top")
  const [tabCache, setTabCache] = useState<Partial<Record<DashboardTab, DashboardData>>>({})
  const [lastDashboard, setLastDashboard] = useState<DashboardData | null>(null)
  const [theme, setTheme] = useState<ThemePref>("dark")
  const [density, setDensity] = useState<DensityPref>("normal")
  const [statsVisible, setStatsVisible] = useState(false)
  const [loadingTabs, setLoadingTabs] = useState<DashboardTab[]>([])
  const [loadError, setLoadError] = useState<string | null>(null)
  const [ready, setReady] = useState(false)
  const inFlightTabs = useRef<Set<DashboardTab>>(new Set())

  useEffect(() => {
    const params = new URLSearchParams(window.location.search)
    setActiveTab(parseTab(params.get("tab")))
    setSortBy(parseSort(params.get("sort")))
    setPeriod(parsePeriod(params.get("period")))
    setSelectedUser(parseOptionalFilter(params.get("user")))
    setSelectedChannel(parseOptionalFilter(params.get("channel")))
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
      const storedStatsVisibility = localStorage.getItem(STATS_VISIBILITY_KEY)
      if (storedStatsVisibility === "visible" || storedStatsVisibility === "hidden") {
        setStatsVisible(storedStatsVisibility === "visible")
      } else {
        setStatsVisible(false)
      }
    } catch (_err) {}

    setReady(true)
  }, [])

  useEffect(() => {
    const timeout = window.setTimeout(() => {
      setDebouncedQuery(query.trim())
    }, SEARCH_DEBOUNCE_MS)
    return () => {
      window.clearTimeout(timeout)
    }
  }, [query])

  useEffect(() => {
    if (!ready) return
    const root = document.documentElement
    const suppressTransitions = document.createElement("style")
    suppressTransitions.appendChild(
      document.createTextNode("*{transition:none !important; animation:none !important;}")
    )
    document.head.appendChild(suppressTransitions)
    root.classList.toggle("theme-light", theme === "light")
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        suppressTransitions.remove()
      })
    })
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
    try {
      localStorage.setItem(STATS_VISIBILITY_KEY, statsVisible ? "visible" : "hidden")
    } catch (_err) {}
  }, [ready, statsVisible])

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

  const fetchTabData = useCallback(async (tab: DashboardTab, withLoader: boolean) => {
    if (inFlightTabs.current.has(tab)) return
    inFlightTabs.current.add(tab)
    if (withLoader) {
      setLoadingTabs((prev) => (prev.includes(tab) ? prev : [...prev, tab]))
      setLoadError(null)
    }

    try {
      const nextDashboard = await fetchDashboardData({ tab, limit: 200 })
      setTabCache((prev) => ({ ...prev, [tab]: nextDashboard }))
      if (withLoader) {
        setLoadError(null)
      }
    } catch (_err) {
      if (withLoader) {
        setLoadError("Failed to load dashboard data.")
      }
    } finally {
      inFlightTabs.current.delete(tab)
      if (withLoader) {
        setLoadingTabs((prev) => prev.filter((value) => value !== tab))
      }
    }
  }, [])

  const activeDashboard = tabCache[activeTab] ?? null

  useEffect(() => {
    if (!ready || activeDashboard) return
    void fetchTabData(activeTab, true)
  }, [ready, activeDashboard, activeTab, fetchTabData])

  useEffect(() => {
    if (activeDashboard) {
      setLastDashboard(activeDashboard)
      setLoadError(null)
    }
  }, [activeDashboard])

  useEffect(() => {
    if (!ready || !activeDashboard) return
    const missingTabs = TABS.filter((tab) => tab !== activeTab && !tabCache[tab])
    if (missingTabs.length === 0) return

    const timeout = window.setTimeout(() => {
      missingTabs.forEach((tab) => {
        void fetchTabData(tab, false)
      })
    }, 200)

    return () => {
      window.clearTimeout(timeout)
    }
  }, [ready, activeDashboard, activeTab, tabCache, fetchTabData])

  const currentDashboard = activeDashboard ?? lastDashboard ?? EMPTY_DASHBOARD
  const isLoading = loadingTabs.length > 0
  const hasAnyDashboard = Boolean(activeDashboard || lastDashboard)
  const isInitialLoad = ready && !hasAnyDashboard && isLoading
  const isTabTransitionLoading = isLoading && Boolean(lastDashboard) && !activeDashboard
  const loadingTab = loadingTabs[loadingTabs.length - 1] ?? activeTab
  const loadingLabel =
    loadingTab === "top"
      ? "top threads"
      : loadingTab === "week"
        ? "weekly"
        : loadingTab === "month"
          ? "monthly"
          : "recent"

  const currentThreads = useMemo(() => {
    const threads = [...currentDashboard.threads]
    const searchTerm = normalizeSearchValue(debouncedQuery)
    const selectedChannelNormalized = selectedChannel?.replace(/^#/, "") ?? null
    const cutoffSecs =
      period === "7d"
        ? Date.now() / 1000 - 7 * 86_400
        : period === "30d"
          ? Date.now() / 1000 - 30 * 86_400
          : 0

    const filtered = threads.filter((thread) => {
      if (cutoffSecs > 0 && Number.parseFloat(thread.ts) < cutoffSecs) return false
      if (selectedUser && thread.author.name !== selectedUser) return false
      if (
        selectedChannelNormalized &&
        thread.channel.replace(/^#/, "") !== selectedChannelNormalized
      ) {
        return false
      }
      if (
        searchTerm &&
        !normalizeSearchValue(thread.message).includes(searchTerm) &&
        !normalizeSearchValue(stripHtmlForSearch(thread.messageHtml)).includes(searchTerm) &&
        !normalizeSearchValue(thread.searchText ?? "").includes(searchTerm) &&
        !normalizeSearchValue(thread.author.name).includes(searchTerm) &&
        !normalizeSearchValue(thread.channel).includes(searchTerm)
      ) {
        return false
      }
      return true
    })

    filtered.sort((a, b) => {
      if (sortBy === "date") return Number.parseFloat(b.ts) - Number.parseFloat(a.ts)
      if (sortBy === "reactions") return b.reactions - a.reactions
      if (sortBy === "replies") return b.replies - a.replies
      return b.score - a.score
    })

    if (!debouncedQuery) {
      return filtered
    }

    return filtered.map((thread) => ({
      ...thread,
      messageHtml: highlightHtml(thread.messageHtml, debouncedQuery),
    }))
  }, [currentDashboard.threads, debouncedQuery, period, selectedChannel, selectedUser, sortBy])

  useEffect(() => {
    if (!expandedThreadId) return
    const existsInCurrentList = currentThreads.some((thread) => thread.id === expandedThreadId)
    if (!existsInCurrentList) {
      setExpandedThreadId(null)
    }
  }, [currentThreads, expandedThreadId])

  const scoreScaleMax = useMemo(
    () => Math.max(1, ...currentThreads.map((thread) => thread.score)),
    [currentThreads]
  )

  const loadThreadMessages = useCallback(
    async (thread: SlackThread): Promise<ThreadMessage[]> =>
      fetchThreadMessages(thread.channelId, thread.ts, debouncedQuery || undefined),
    [debouncedQuery]
  )

  const toggleUserFilter = useCallback((userName: string | null) => {
    const nextUser = userName?.trim() ?? ""
    setExpandedThreadId(null)
    setSelectedUser((prev) => {
      if (!nextUser) return null
      return prev === nextUser ? null : nextUser
    })
  }, [])

  const toggleChannelFilter = useCallback((channelName: string | null) => {
    const nextNormalized = channelName?.replace(/^#/, "").trim() ?? ""
    setExpandedThreadId(null)
    setSelectedChannel((prev) => {
      if (!nextNormalized) return null
      const prevNormalized = prev?.replace(/^#/, "").trim() ?? ""
      return prevNormalized.toLowerCase() === nextNormalized.toLowerCase()
        ? null
        : `#${nextNormalized}`
    })
  }, [])

  const applyUserFilterFromMention = useCallback((userName: string) => {
    toggleUserFilter(userName)
  }, [toggleUserFilter])

  const applyChannelFilterFromMention = useCallback((channelName: string) => {
    toggleChannelFilter(channelName)
  }, [toggleChannelFilter])

  function retryCurrentTab(): void {
    setTabCache((prev) => {
      const next = { ...prev }
      delete next[activeTab]
      return next
    })
    setLoadError(null)
  }

  if (!ready || (isInitialLoad && !loadError)) {
    return <InitialLoadingShell />
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
            onValueChange={(value) => {
              const nextTab = value as DashboardTab
              setActiveTab(nextTab)
              if (nextTab === "recent" && sortBy === "score") {
                setSortBy("date")
              }
            }}
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
              <TabsTrigger value="recent" className="px-3 text-xs">
                Last 50
              </TabsTrigger>
            </TabsList>
          </Tabs>

          <div className="ml-auto flex items-center gap-2">
            <button
              type="button"
              onClick={() => setStatsVisible((prev) => !prev)}
              className="inline-flex h-8 items-center gap-1 rounded-md border border-border bg-secondary px-2 py-1 text-[11px] text-muted-foreground transition-colors hover:text-foreground sm:text-xs"
            >
              <span className="sm:hidden">Stats</span>
              <span className="hidden sm:inline">
                {statsVisible ? "Hide stats" : "Show stats"}
              </span>
            </button>
            <button
              type="button"
              onClick={() => setDensity((prev) => (prev === "compact" ? "normal" : "compact"))}
              className="inline-flex h-8 items-center gap-1 rounded-md border border-border bg-secondary px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
              aria-label={density === "compact" ? "Switch to normal density" : "Switch to compact density"}
            >
              {density === "compact" ? <Rows2 className="size-3.5" /> : <Rows3 className="size-3.5" />}
              <span className="hidden sm:inline">
                {density === "compact" ? "Compact" : "Normal"}
              </span>
            </button>
            <button
              type="button"
              onClick={() => setTheme((prev) => (prev === "dark" ? "light" : "dark"))}
              className="inline-flex h-8 items-center gap-1 rounded-md border border-border bg-secondary px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
              aria-label={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
            >
              {theme === "dark" ? <Moon className="size-3.5" /> : <Sun className="size-3.5" />}
              <span className="hidden sm:inline">{theme === "dark" ? "Dark" : "Light"}</span>
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
                className="inline-flex h-8 items-center gap-1 rounded-md border border-border bg-secondary px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground sm:gap-1.5"
              >
                <ExternalLink className="size-3" />
                <span className="hidden sm:inline">Open Slack</span>
              </a>
            )}
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-screen-2xl px-4 py-6 lg:px-6">
        <div className="mb-4 sm:hidden">
          <Tabs
            value={activeTab}
            onValueChange={(value) => {
              const nextTab = value as DashboardTab
              setActiveTab(nextTab)
              if (nextTab === "recent" && sortBy === "score") {
                setSortBy("date")
              }
            }}
          >
            <TabsList className="w-full bg-secondary">
              <TabsTrigger value="top" className="text-xs">
                Top Threads
              </TabsTrigger>
              <TabsTrigger value="week" className="text-xs">
                Week
              </TabsTrigger>
              <TabsTrigger value="month" className="text-xs">
                Month
              </TabsTrigger>
              <TabsTrigger value="recent" className="text-xs">
                Last 50
              </TabsTrigger>
            </TabsList>
          </Tabs>
        </div>

        {statsVisible && (
          <>
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
              <ActivityChart data={currentDashboard.activityData} tab={activeTab} />
              <div className="hidden lg:block">
                <ChannelSidebar
                  channels={currentDashboard.channelStats}
                  selected={selectedChannel}
                  onSelect={toggleChannelFilter}
                />
              </div>
            </section>

            <div className="mb-4 lg:hidden">
              <ChannelSidebar
                channels={currentDashboard.channelStats}
                selected={selectedChannel}
                onSelect={toggleChannelFilter}
              />
            </div>
          </>
        )}

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
            onUserChange={toggleUserFilter}
          />
        </section>

        <section className="flex flex-col gap-2">
          {isLoading && currentThreads.length === 0 && (
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

          {!isLoading && !loadError && currentThreads.length === 0 && (
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
                  setExpandedThreadId(null)
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
              expanded={expandedThreadId === thread.id}
              onExpandedChange={(nextExpanded) =>
                setExpandedThreadId(nextExpanded ? thread.id : null)
              }
              maxScore={scoreScaleMax}
              density={density}
              onLoadThreadMessages={loadThreadMessages}
              onMentionClick={applyUserFilterFromMention}
              onChannelClick={applyChannelFilterFromMention}
            />
          ))}
        </section>

        <footer className="mt-8 border-t border-border pt-4 text-center text-xs text-muted-foreground">
          Archivist archives all messages from your Slack workspace.
          <br />
          Showing {currentThreads.length} threads sorted by {sortBy}.
        </footer>
      </main>

      {isTabTransitionLoading && (
        <div className="fixed inset-0 z-40 flex items-center justify-center bg-background/50 backdrop-blur-[1px]">
          <div className="flex items-center gap-3 rounded-lg border border-border bg-card px-5 py-4 text-sm text-muted-foreground shadow-sm">
            <LoaderCircle className="size-4 animate-spin" />
            Updating {loadingLabel} data...
          </div>
        </div>
      )}
    </div>
  )
}
