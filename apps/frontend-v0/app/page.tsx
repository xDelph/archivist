"use client"

import { useState, useMemo } from "react"
import { Archive, ExternalLink } from "lucide-react"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { StatCard } from "@/components/stat-card"
import { ThreadCard } from "@/components/thread-card"
import { ActivityChart } from "@/components/activity-chart"
import { ChannelSidebar } from "@/components/channel-sidebar"
import { SearchBar } from "@/components/search-bar"
import {
  allTimeThreads,
  weekThreads,
  monthThreads,
  channelStats,
  activityData,
  overviewStats,
  type SlackThread,
} from "@/lib/mock-data"

function sortThreads(
  threads: SlackThread[],
  sortBy: string
): SlackThread[] {
  return [...threads].sort((a, b) => {
    switch (sortBy) {
      case "replies":
        return b.replies - a.replies
      case "reactions":
        return b.reactions - a.reactions
      case "date":
        return new Date(b.date).getTime() - new Date(a.date).getTime()
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
  return threads.filter((t) => {
    const matchesQuery =
      !query ||
      t.message.toLowerCase().includes(query.toLowerCase()) ||
      t.author.name.toLowerCase().includes(query.toLowerCase()) ||
      t.channel.toLowerCase().includes(query.toLowerCase())
    const matchesChannel = !channel || t.channel === channel
    return matchesQuery && matchesChannel
  })
}

export default function ArchivistDashboard() {
  const [query, setQuery] = useState("")
  const [sortBy, setSortBy] = useState("score")
  const [selectedChannel, setSelectedChannel] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState("all-time")

  const threadMap: Record<string, SlackThread[]> = {
    "all-time": allTimeThreads,
    "this-week": weekThreads,
    "this-month": monthThreads,
  }

  const currentThreads = useMemo(() => {
    const base = threadMap[activeTab] ?? allTimeThreads
    const filtered = filterThreads(base, query, selectedChannel)
    return sortThreads(filtered, sortBy)
  }, [activeTab, query, selectedChannel, sortBy])

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="sticky top-0 z-30 border-b border-border bg-background/80 backdrop-blur-md">
        <div className="mx-auto flex h-14 max-w-screen-2xl items-center gap-4 px-4 lg:px-6">
          <div className="flex items-center gap-2">
            <div className="flex size-8 items-center justify-center rounded-lg bg-primary">
              <Archive className="size-4 text-primary-foreground" />
            </div>
            <span className="text-base font-semibold text-foreground">
              Archivist
            </span>
          </div>

          {/* Tab nav in header */}
          <Tabs
            value={activeTab}
            onValueChange={setActiveTab}
            className="ml-4 hidden sm:flex"
          >
            <TabsList className="h-8 bg-secondary">
              <TabsTrigger value="all-time" className="text-xs px-3">
                Top Threads
              </TabsTrigger>
              <TabsTrigger value="this-week" className="text-xs px-3">
                This Week
              </TabsTrigger>
              <TabsTrigger value="this-month" className="text-xs px-3">
                This Month
              </TabsTrigger>
            </TabsList>
          </Tabs>

          <div className="ml-auto flex items-center gap-3">
            <a
              href="#"
              className="hidden items-center gap-1.5 text-xs text-muted-foreground transition-colors hover:text-foreground sm:flex"
            >
              Open Slack
              <ExternalLink className="size-3" />
            </a>
          </div>
        </div>
      </header>

      {/* Body */}
      <main className="mx-auto max-w-screen-2xl px-4 py-6 lg:px-6">
        {/* Mobile tab selector */}
        <div className="mb-4 sm:hidden">
          <Tabs value={activeTab} onValueChange={setActiveTab}>
            <TabsList className="w-full bg-secondary">
              <TabsTrigger value="all-time" className="flex-1 text-xs">
                Top Threads
              </TabsTrigger>
              <TabsTrigger value="this-week" className="flex-1 text-xs">
                Week
              </TabsTrigger>
              <TabsTrigger value="this-month" className="flex-1 text-xs">
                Month
              </TabsTrigger>
            </TabsList>
          </Tabs>
        </div>

        {/* Overview Stats */}
        <section className="mb-6 grid grid-cols-2 gap-3 lg:grid-cols-4">
          <StatCard
            title="Total Messages"
            value={overviewStats.totalMessages.toLocaleString()}
            change={overviewStats.messagesChange}
            icon="messages"
          />
          <StatCard
            title="Threads"
            value={overviewStats.totalThreads.toLocaleString()}
            change={overviewStats.threadsChange}
            icon="threads"
          />
          <StatCard
            title="Files Archived"
            value={overviewStats.totalFiles.toLocaleString()}
            change={overviewStats.filesChange}
            icon="files"
          />
          <StatCard
            title="Active Users"
            value={overviewStats.totalUsers.toLocaleString()}
            change={overviewStats.usersChange}
            icon="users"
          />
        </section>

        {/* Chart + Sidebar above threads */}
        <section className="mb-6 grid gap-4 lg:grid-cols-[1fr_260px]">
          <ActivityChart data={activityData} />
          <div className="hidden lg:block">
            <ChannelSidebar
              channels={channelStats}
              selected={selectedChannel}
              onSelect={setSelectedChannel}
            />
          </div>
        </section>

        {/* Mobile channel filter */}
        <div className="mb-4 lg:hidden">
          <ChannelSidebar
            channels={channelStats}
            selected={selectedChannel}
            onSelect={setSelectedChannel}
          />
        </div>

        {/* Search & Sort */}
        <section className="mb-4">
          <SearchBar
            query={query}
            onQueryChange={setQuery}
            sortBy={sortBy}
            onSortChange={setSortBy}
          />
        </section>

        {/* Thread List */}
        <section className="flex flex-col gap-2">
          {currentThreads.length === 0 ? (
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
          ) : (
            currentThreads.map((thread, i) => (
              <ThreadCard key={thread.id} thread={thread} rank={i + 1} />
            ))
          )}
        </section>

        {/* Footer */}
        <footer className="mt-8 border-t border-border pt-4 text-center text-xs text-muted-foreground">
          Archivist archives all messages from your Slack workspace.
          <br />
          Showing {currentThreads.length} threads sorted by {sortBy}.
        </footer>
      </main>
    </div>
  )
}
