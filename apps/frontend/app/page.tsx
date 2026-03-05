"use client"

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import {
  Archive,
  ExternalLink,
  LoaderCircle,
  Moon,
  Sun,
  Rows3,
  Rows2,
  Eye,
  EyeOff,
} from "lucide-react"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { StatCard } from "@/components/stat-card"
import { ThreadCard } from "@/components/thread-card"
import { ActivityChart } from "@/components/activity-chart"
import { AccountMenu } from "@/components/account-menu"
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
const TABS: DashboardTab[] = ["recent", "week", "month", "top"]
const TAB_TRANSITION_DURATION_MS = 1000
const LANDING_FADE_DURATION_MS = 1000
const EMPTY_DASHBOARD: DashboardData = {
  tab: "recent",
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
  if (value === "recent" || value === "week" || value === "month" || value === "top") {
    return value
  }
  return "recent"
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

function tabDocumentLabel(tab: DashboardTab): string {
  if (tab === "week") return "This Week"
  if (tab === "month") return "This Month"
  if (tab === "recent") return "Last Messages"
  return "Top Threads"
}

function buildStatsMotionKey(dashboard: DashboardData): string {
  const stats = dashboard.overviewStats
  const channelsPart = dashboard.channelStats
    .map((channel) => `${channel.name}:${channel.count}`)
    .join("|")
  const activityPart = dashboard.activityData
    .map((point) => `${point.date}:${point.messages}:${point.threads}`)
    .join("|")
  return [
    dashboard.tab,
    stats.totalMessages,
    stats.totalThreads,
    stats.totalFiles,
    stats.totalUsers,
    channelsPart,
    activityPart,
  ].join("::")
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

function InitialLoadingShell({ className = "min-h-screen" }: { className?: string }) {
  return (
    <div className={`flex ${className} items-center justify-center bg-background px-4`}>
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
  const [activeTab, setActiveTab] = useState<DashboardTab>("recent")
  const [pendingTab, setPendingTab] = useState<DashboardTab | null>(null)
  const [tabTransitionDirection, setTabTransitionDirection] = useState<1 | -1>(1)
  const [tabCache, setTabCache] = useState<Partial<Record<DashboardTab, DashboardData>>>({})
  const [lastDashboard, setLastDashboard] = useState<DashboardData | null>(null)
  const [theme, setTheme] = useState<ThemePref>("dark")
  const [density, setDensity] = useState<DensityPref>("normal")
  const [statsVisible, setStatsVisible] = useState(false)
  const [loadingTabs, setLoadingTabs] = useState<DashboardTab[]>([])
  const [loadError, setLoadError] = useState<string | null>(null)
  const [ready, setReady] = useState(false)
  const [isDesktopViewport, setIsDesktopViewport] = useState(false)
  const [threadListMinHeight, setThreadListMinHeight] = useState<number | null>(null)
  const [isSlideAnimating, setIsSlideAnimating] = useState(false)
  const [showLandingOverlay, setShowLandingOverlay] = useState(true)
  const [isLandingOverlayFading, setIsLandingOverlayFading] = useState(false)
  const [isDashboardVisible, setIsDashboardVisible] = useState(false)
  const inFlightTabs = useRef<Set<DashboardTab>>(new Set())
  const tabSwitchTimeoutRef = useRef<number | null>(null)
  const threadListRef = useRef<HTMLElement | null>(null)
  const landingOverlayTimeoutRef = useRef<number | null>(null)

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

    setIsDesktopViewport(window.matchMedia("(min-width: 1024px)").matches)
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
    return () => {
      if (tabSwitchTimeoutRef.current != null) {
        window.clearTimeout(tabSwitchTimeoutRef.current)
      }
      if (landingOverlayTimeoutRef.current != null) {
        window.clearTimeout(landingOverlayTimeoutRef.current)
      }
    }
  }, [])

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
    const mediaQuery = window.matchMedia("(min-width: 1024px)")
    const syncViewport = () => setIsDesktopViewport(mediaQuery.matches)
    syncViewport()

    if (typeof mediaQuery.addEventListener === "function") {
      mediaQuery.addEventListener("change", syncViewport)
      return () => mediaQuery.removeEventListener("change", syncViewport)
    }

    mediaQuery.addListener(syncViewport)
    return () => mediaQuery.removeListener(syncViewport)
  }, [ready])

  useEffect(() => {
    if (!ready) return
    try {
      localStorage.setItem(STATS_VISIBILITY_KEY, statsVisible ? "visible" : "hidden")
    } catch (_err) {}
  }, [ready, statsVisible])

  useEffect(() => {
    if (!ready) return

    const params = new URLSearchParams()
    if (activeTab !== "recent") params.set("tab", activeTab)
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
    document.title = `Archivist ${tabDocumentLabel(activeTab)}`
  }, [ready, activeTab])

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
    } catch (error) {
      setTabCache((prev) => {
        if (!prev[tab]) return prev
        const next = { ...prev }
        delete next[tab]
        return next
      })
      if (withLoader) {
        const details = error instanceof Error ? error.message : "Unknown error"
        setLoadError(`Failed to load ${tab} dashboard data. ${details}`)
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
    const nextMissingTab = TABS.find(
      (tab) => tab !== activeTab && !tabCache[tab] && !inFlightTabs.current.has(tab)
    )
    if (!nextMissingTab) return

    const timeout = window.setTimeout(() => {
      void fetchTabData(nextMissingTab, false)
    }, 200)

    return () => {
      window.clearTimeout(timeout)
    }
  }, [ready, activeDashboard, activeTab, tabCache, fetchTabData])

  const currentDashboard =
    activeDashboard ?? (loadError && !activeDashboard ? EMPTY_DASHBOARD : lastDashboard ?? EMPTY_DASHBOARD)
  const motionTab = isSlideAnimating && pendingTab ? pendingTab : null
  const motionDashboard =
    motionTab && tabCache[motionTab] ? tabCache[motionTab] : currentDashboard
  const statsMotionKey = useMemo(
    () => buildStatsMotionKey(motionDashboard),
    [motionDashboard]
  )
  const isLoading = loadingTabs.length > 0
  const hasAnyDashboard = Boolean(activeDashboard || lastDashboard)
  const isInitialLoad = ready && !hasAnyDashboard && isLoading
  const shouldShowInitialLoading = !ready || (isInitialLoad && !loadError)
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

  useEffect(() => {
    if (landingOverlayTimeoutRef.current != null) {
      window.clearTimeout(landingOverlayTimeoutRef.current)
      landingOverlayTimeoutRef.current = null
    }

    if (shouldShowInitialLoading) {
      setShowLandingOverlay(true)
      setIsLandingOverlayFading(false)
      setIsDashboardVisible(false)
      return
    }

    setIsDashboardVisible(true)
    setIsLandingOverlayFading(true)
    landingOverlayTimeoutRef.current = window.setTimeout(() => {
      setShowLandingOverlay(false)
      landingOverlayTimeoutRef.current = null
    }, LANDING_FADE_DURATION_MS)
  }, [shouldShowInitialLoading])

  useEffect(() => {
    if (pendingTab !== null && pendingTab !== activeTab) {
      const list = threadListRef.current
      if (list) {
        setThreadListMinHeight(list.offsetHeight)
      }
      return
    }

    if (threadListMinHeight == null) return
    const timeout = window.setTimeout(() => {
      setThreadListMinHeight(null)
    }, TAB_TRANSITION_DURATION_MS)

    return () => {
      window.clearTimeout(timeout)
    }
  }, [pendingTab, activeTab, threadListMinHeight])

  const buildThreadsForDashboard = useCallback((dashboard: DashboardData): SlackThread[] => {
    const threads = [...dashboard.threads]
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
  }, [debouncedQuery, period, selectedChannel, selectedUser, sortBy])

  const filteredThreadsByTab = useMemo(() => {
    const byTab: Partial<Record<DashboardTab, SlackThread[]>> = {}
    for (const tab of TABS) {
      const dashboard = tabCache[tab]
      if (dashboard) {
        byTab[tab] = buildThreadsForDashboard(dashboard)
      }
    }
    if (!byTab[currentDashboard.tab]) {
      byTab[currentDashboard.tab] = buildThreadsForDashboard(currentDashboard)
    }
    return byTab
  }, [tabCache, currentDashboard, buildThreadsForDashboard])

  const currentThreads = useMemo(
    () => filteredThreadsByTab[currentDashboard.tab] ?? buildThreadsForDashboard(currentDashboard),
    [filteredThreadsByTab, currentDashboard, buildThreadsForDashboard]
  )

  const paneTabs = useMemo(
    () =>
      TABS.filter((tab) => tab === activeTab || tab === pendingTab || Boolean(tabCache[tab])),
    [activeTab, pendingTab, tabCache]
  )

  const scoreScaleByTab = useMemo(() => {
    const byTab: Partial<Record<DashboardTab, number>> = {}
    for (const tab of paneTabs) {
      const threads = filteredThreadsByTab[tab] ?? []
      byTab[tab] = Math.max(1, ...threads.map((thread) => thread.score))
    }
    return byTab
  }, [paneTabs, filteredThreadsByTab])

  useEffect(() => {
    if (!expandedThreadId) return
    const existsInCurrentList = currentThreads.some((thread) => thread.id === expandedThreadId)
    if (!existsInCurrentList) {
      setExpandedThreadId(null)
    }
  }, [currentThreads, expandedThreadId])

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

  async function logout(): Promise<void> {
    try {
      await fetch("/api/auth/logout", { method: "POST" })
    } finally {
      window.location.href = "/login"
    }
  }

  const isOutgoingPhase = pendingTab !== null && pendingTab !== activeTab

  const threadListMotionStyle = {
    minHeight: threadListMinHeight ?? undefined,
  }

  const outgoingPaneStyle = {
    transform: isOutgoingPhase
      ? isSlideAnimating
        ? tabTransitionDirection > 0
          ? "translateX(-100%)"
          : "translateX(100%)"
        : "translateX(0%)"
      : "translateX(0%)",
    transitionDuration: `${TAB_TRANSITION_DURATION_MS}ms`,
    transitionTimingFunction: "cubic-bezier(0.22, 1, 0.36, 1)",
  }

  const incomingPaneStyle = {
    transform: isOutgoingPhase
      ? isSlideAnimating
        ? "translateX(0%)"
        : tabTransitionDirection > 0
          ? "translateX(100%)"
          : "translateX(-100%)"
      : "translateX(0%)",
    transitionDuration: `${TAB_TRANSITION_DURATION_MS}ms`,
    transitionTimingFunction: "cubic-bezier(0.22, 1, 0.36, 1)",
  }

  function handleTabChange(value: string): void {
    const nextTab = value as DashboardTab
    if (nextTab === activeTab || nextTab === pendingTab) return
    const currentIndex = TABS.indexOf(activeTab)
    const nextIndex = TABS.indexOf(nextTab)
    setTabTransitionDirection(nextIndex >= currentIndex ? 1 : -1)
    setExpandedThreadId(null)

    if (!tabCache[nextTab] && !inFlightTabs.current.has(nextTab)) {
      void fetchTabData(nextTab, false)
    }

    setIsSlideAnimating(false)
    setPendingTab(nextTab)

    if (tabSwitchTimeoutRef.current != null) {
      window.clearTimeout(tabSwitchTimeoutRef.current)
    }

    window.requestAnimationFrame(() => {
      setIsSlideAnimating(true)

      tabSwitchTimeoutRef.current = window.setTimeout(() => {
        setActiveTab(nextTab)
        setPendingTab(null)
        setIsSlideAnimating(false)
        if (nextTab === "recent" && sortBy === "score") {
          setSortBy("date")
        }
        tabSwitchTimeoutRef.current = null
      }, TAB_TRANSITION_DURATION_MS)
    })
  }

  function renderPaneContent(tab: DashboardTab) {
    const dashboardForTab =
      tabCache[tab] ?? (tab === currentDashboard.tab ? currentDashboard : null)

    if (!dashboardForTab) {
      return (
        <div className="flex items-center justify-center gap-2 rounded-lg border border-border bg-card py-16 text-sm text-muted-foreground">
          <LoaderCircle className="size-4 animate-spin" />
          Loading tab...
        </div>
      )
    }

    const threads = filteredThreadsByTab[tab] ?? []
    const paneScoreScale = scoreScaleByTab[tab] ?? 1
    const isActivePane = tab === activeTab
    const canInteract = isActivePane && !isOutgoingPhase
    const showActiveStates = isActivePane && !isOutgoingPhase

    if (showActiveStates && isLoading && threads.length === 0 && !isTabTransitionLoading) {
      return (
        <div className="flex items-center justify-center gap-2 rounded-lg border border-border bg-card py-16 text-sm text-muted-foreground">
          <LoaderCircle className="size-4 animate-spin" />
          Loading dashboard...
        </div>
      )
    }

    if (showActiveStates && loadError && !activeDashboard) {
      return (
        <div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-border bg-card py-16 text-center">
          <p className="text-sm text-muted-foreground">{loadError}</p>
          <button onClick={retryCurrentTab} className="text-xs text-primary hover:underline">
            Retry
          </button>
        </div>
      )
    }

    if (threads.length === 0 && (!showActiveStates || (!isLoading && !loadError))) {
      return (
        <div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-border bg-card py-16 text-center">
          <p className="text-sm text-muted-foreground">
            No threads found matching your filters.
          </p>
          {showActiveStates && (
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
          )}
        </div>
      )
    }

    return (
      <div className="relative flex flex-col gap-2">
        {showActiveStates && isTabTransitionLoading && (
          <div className="pointer-events-none absolute inset-0 z-10 rounded-lg bg-background/45 backdrop-blur-[1px]">
            <div className="flex items-center gap-2 px-4 py-3 text-xs text-muted-foreground">
              <span className="size-1.5 animate-pulse rounded-full bg-primary/70" />
              Updating {loadingLabel} data...
            </div>
          </div>
        )}

        {threads.map((thread, index) => (
          <ThreadCard
            key={`pane:${tab}:${thread.id}`}
            thread={thread}
            rank={index + 1}
            expanded={canInteract && expandedThreadId === thread.id}
            onExpandedChange={(nextExpanded) => {
              if (canInteract) {
                setExpandedThreadId(nextExpanded ? thread.id : null)
              }
            }}
            maxScore={paneScoreScale}
            density={density}
            onLoadThreadMessages={canInteract ? loadThreadMessages : undefined}
            onMentionClick={canInteract ? applyUserFilterFromMention : undefined}
            onChannelClick={canInteract ? applyChannelFilterFromMention : undefined}
          />
        ))}
      </div>
    )
  }

  return (
    <div className="relative min-h-screen overflow-x-clip bg-background">
      <div
        className={`transition-opacity ${
          isDashboardVisible ? "opacity-100" : "opacity-0"
        }`}
        style={{ transitionDuration: `${LANDING_FADE_DURATION_MS}ms` }}
      >
        <header className="sticky top-0 z-30 border-b border-border bg-background/80 backdrop-blur-md">
        <div className="mx-auto flex h-14 max-w-screen-2xl items-center gap-4 px-4 lg:px-6">
          <div className="flex items-center gap-2">
            <div className="flex size-8 items-center justify-center rounded-lg bg-primary">
              <Archive className="size-4 text-primary-foreground" />
            </div>
            <span className="text-base font-semibold text-foreground">Archivist</span>
          </div>

          <Tabs
            value={pendingTab ?? activeTab}
            onValueChange={handleTabChange}
            className="ml-4 hidden sm:flex"
          >
            <TabsList className="h-8 bg-secondary">
              <TabsTrigger
                value="recent"
                className="px-3 text-xs transition-all duration-300 data-[state=active]:-translate-y-px"
              >
                Last 50
              </TabsTrigger>
              <TabsTrigger
                value="week"
                className="px-3 text-xs transition-all duration-300 data-[state=active]:-translate-y-px"
              >
                This Week
              </TabsTrigger>
              <TabsTrigger
                value="month"
                className="px-3 text-xs transition-all duration-300 data-[state=active]:-translate-y-px"
              >
                This Month
              </TabsTrigger>
              <TabsTrigger
                value="top"
                className="px-3 text-xs transition-all duration-300 data-[state=active]:-translate-y-px"
              >
                Top Threads
              </TabsTrigger>
            </TabsList>
          </Tabs>

          <div className="ml-auto flex items-center gap-2">
            <button
              type="button"
              onClick={() => setStatsVisible((prev) => !prev)}
              className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-border bg-secondary text-muted-foreground transition-colors hover:text-foreground"
              title={statsVisible ? "Hide stats panels" : "Show stats panels"}
              aria-label={statsVisible ? "Hide stats panels" : "Show stats panels"}
            >
              {statsVisible ? <EyeOff className="size-3.5" /> : <Eye className="size-3.5" />}
            </button>
            <button
              type="button"
              onClick={() => setDensity((prev) => (prev === "compact" ? "normal" : "compact"))}
              className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-border bg-secondary text-muted-foreground transition-colors hover:text-foreground"
              title={density === "compact" ? "Switch to normal density" : "Switch to compact density"}
              aria-label={density === "compact" ? "Switch to normal density" : "Switch to compact density"}
            >
              {density === "compact" ? <Rows2 className="size-3.5" /> : <Rows3 className="size-3.5" />}
            </button>
            <button
              type="button"
              onClick={() => setTheme((prev) => (prev === "dark" ? "light" : "dark"))}
              className="inline-flex h-8 w-8 items-center justify-center rounded-md border border-border bg-secondary text-muted-foreground transition-colors hover:text-foreground"
              title={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
              aria-label={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
            >
              {theme === "dark" ? <Moon className="size-3.5" /> : <Sun className="size-3.5" />}
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
                className="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-secondary px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
              >
                <ExternalLink className="size-3" />
                <span>Open Slack</span>
              </a>
            )}
            <AccountMenu onLogout={logout} />
          </div>
        </div>
        </header>

        <main className="mx-auto max-w-screen-2xl px-4 py-6 lg:px-6">
        <div className="mb-4 sm:hidden">
          <Tabs
            value={pendingTab ?? activeTab}
            onValueChange={handleTabChange}
          >
            <TabsList className="w-full bg-secondary">
              <TabsTrigger
                value="recent"
                className="text-xs transition-all duration-300 data-[state=active]:-translate-y-px"
              >
                Last 50
              </TabsTrigger>
              <TabsTrigger
                value="week"
                className="text-xs transition-all duration-300 data-[state=active]:-translate-y-px"
              >
                Week
              </TabsTrigger>
              <TabsTrigger
                value="month"
                className="text-xs transition-all duration-300 data-[state=active]:-translate-y-px"
              >
                Month
              </TabsTrigger>
              <TabsTrigger
                value="top"
                className="text-xs transition-all duration-300 data-[state=active]:-translate-y-px"
              >
                Top Threads
              </TabsTrigger>
            </TabsList>
          </Tabs>
        </div>

        {statsVisible && (
          <>
            <section className="mb-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
              <StatCard
                title="Total Messages"
                value={motionDashboard.overviewStats.totalMessages}
                syncKey={statsMotionKey}
                change={
                  motionDashboard.tab === "recent"
                    ? undefined
                    : motionDashboard.overviewStats.messagesChange
                }
                icon="messages"
              />
              <StatCard
                title="Threads"
                value={motionDashboard.overviewStats.totalThreads}
                syncKey={statsMotionKey}
                change={
                  motionDashboard.tab === "recent"
                    ? undefined
                    : motionDashboard.overviewStats.threadsChange
                }
                icon="threads"
              />
              <StatCard
                title="Files Archived"
                value={motionDashboard.overviewStats.totalFiles}
                syncKey={statsMotionKey}
                change={
                  motionDashboard.tab === "recent"
                    ? undefined
                    : motionDashboard.overviewStats.filesChange
                }
                icon="files"
              />
              <StatCard
                title="Active Users"
                value={motionDashboard.overviewStats.totalUsers}
                syncKey={statsMotionKey}
                change={
                  motionDashboard.tab === "recent"
                    ? undefined
                    : motionDashboard.overviewStats.usersChange
                }
                icon="users"
              />
            </section>

            <section className="mb-6 grid gap-4 lg:grid-cols-[1fr_260px]">
              <ActivityChart
                data={motionDashboard.activityData}
                tab={motionDashboard.tab}
                syncKey={statsMotionKey}
              />
              {isDesktopViewport && (
                <div className="hidden lg:block">
                  <ChannelSidebar
                    channels={motionDashboard.channelStats}
                    selected={selectedChannel}
                    onSelect={toggleChannelFilter}
                    syncKey={statsMotionKey}
                  />
                </div>
              )}
            </section>

            {!isDesktopViewport && (
              <div className="mb-4 lg:hidden">
                <ChannelSidebar
                  channels={motionDashboard.channelStats}
                  selected={selectedChannel}
                  onSelect={toggleChannelFilter}
                  syncKey={statsMotionKey}
                />
              </div>
            )}
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

        <section
          ref={threadListRef}
          className={`relative overflow-x-clip ${isOutgoingPhase ? "pointer-events-none" : ""}`}
          style={threadListMotionStyle}
        >
          <div className="relative">
            {paneTabs.map((tab) => {
              const isActivePane = tab === activeTab
              const isPendingPane = pendingTab === tab
              const shouldShowPane = isOutgoingPhase ? isActivePane || isPendingPane : isActivePane
              const paneClass = [
                "flex flex-col gap-2 will-change-transform",
                isPendingPane && isOutgoingPhase ? "absolute inset-0" : "relative",
                shouldShowPane ? "" : "hidden",
              ].join(" ")
              const paneStyle = isOutgoingPhase
                ? isActivePane
                  ? outgoingPaneStyle
                  : isPendingPane
                    ? incomingPaneStyle
                    : undefined
                : undefined

              return (
                <div key={`pane:${tab}`} className={paneClass} style={paneStyle}>
                  {renderPaneContent(tab)}
                </div>
              )
            })}
          </div>
        </section>

        <footer className="mt-8 border-t border-border pt-4 text-center text-xs text-muted-foreground">
          Archivist archives all messages from your Slack workspace.
          <br />
          Showing {currentThreads.length} threads sorted by {sortBy}.
        </footer>
        </main>
      </div>
      {showLandingOverlay && (
        <div
          className={`pointer-events-none fixed inset-0 z-[100] transition-opacity ${
            isLandingOverlayFading ? "opacity-0" : "opacity-100"
          }`}
          style={{ transitionDuration: `${LANDING_FADE_DURATION_MS}ms` }}
        >
          <InitialLoadingShell className="h-full" />
        </div>
      )}
    </div>
  )
}
