"use client"

import Link from "next/link"
import { Suspense, useEffect, useState } from "react"
import { useRouter, useSearchParams } from "next/navigation"
import { ArrowLeft, Heart, LoaderCircle, Paperclip } from "lucide-react"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { fetchThreadDetail } from "@/lib/api"
import type { ThreadMessage } from "@/lib/types"

function messageHtmlClassName(): string {
  return "slack-text break-words text-sm leading-relaxed text-secondary-foreground [&_.mention]:font-medium [&_.mention]:text-primary [&_a]:text-blue-500 [&_a]:underline [&_a]:underline-offset-2 [&_a:hover]:text-blue-400 [&_a.url-highlight]:text-red-500 [&_a.url-highlight:hover]:text-red-400 [&_mark.search-highlight]:rounded-sm [&_mark.search-highlight]:bg-red-500/20 [&_mark.search-highlight]:px-0.5 [&_mark.search-highlight]:text-red-500 [&_code.slack-inline-code]:rounded [&_code.slack-inline-code]:bg-secondary [&_code.slack-inline-code]:px-1 [&_code.slack-inline-code]:py-0.5 [&_code.slack-inline-code]:font-mono [&_code.slack-inline-code]:text-[0.85em] [&_pre.slack-code]:mt-2 [&_pre.slack-code]:overflow-x-auto [&_pre.slack-code]:rounded-md [&_pre.slack-code]:border [&_pre.slack-code]:border-border/70 [&_pre.slack-code]:bg-secondary/70 [&_pre.slack-code]:p-3 [&_pre.slack-code]:font-mono [&_pre.slack-code]:text-[12px]"
}

function ThreadPageFallback() {
  return (
    <div className="min-h-screen bg-background">
      <main className="mx-auto max-w-4xl px-4 py-6 lg:px-6">
        <div className="flex items-center gap-2 rounded-lg border border-border bg-card px-4 py-12 text-sm text-muted-foreground">
          <LoaderCircle className="size-4 animate-spin" />
          Loading thread messages...
        </div>
      </main>
    </div>
  )
}

function ThreadPageContent() {
  const router = useRouter()
  const searchParams = useSearchParams()
  const [channelLabel, setChannelLabel] = useState("")
  const [threadTs, setThreadTs] = useState("")
  const [search, setSearch] = useState("")
  const [messages, setMessages] = useState<ThreadMessage[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false

    async function loadThread(): Promise<void> {
      const nextChannelId = searchParams.get("channel_id") ?? ""
      const nextTs = searchParams.get("ts") ?? ""
      const nextSearch = searchParams.get("search") ?? ""

      if (!nextChannelId || !nextTs) {
        if (!cancelled) {
          setChannelLabel(nextChannelId)
          setThreadTs(nextTs)
          setSearch(nextSearch)
          setMessages([])
          setError("Missing channel_id or ts in URL.")
        }
        return
      }

      if (!cancelled) {
        setChannelLabel(nextChannelId)
        setThreadTs(nextTs)
        setSearch(nextSearch)
        setLoading(true)
        setError(null)
      }

      try {
        const nextThread = await fetchThreadDetail(nextChannelId, nextTs, nextSearch || undefined)
        if (!cancelled) {
          setMessages(nextThread.messages)
          setChannelLabel(nextThread.channel ?? nextChannelId)
        }
      } catch (_err) {
        if (!cancelled) {
          setMessages([])
          setError("Failed to load thread messages.")
        }
      } finally {
        if (!cancelled) {
          setLoading(false)
        }
      }
    }

    void loadThread()

    return () => {
      cancelled = true
    }
  }, [searchParams])

  function handleMessageClickCapture(event: React.MouseEvent<HTMLElement>): void {
    const target = event.target as HTMLElement
    const anchor = target.closest("a")
    if (!anchor) return
    const href = anchor.getAttribute("href")
    if (!href) return

    try {
      const url = new URL(href, window.location.href)
      if (url.origin !== window.location.origin || url.pathname !== "/record/thread") {
        return
      }
      event.preventDefault()
      event.stopPropagation()
      router.push(`${url.pathname}${url.search}${url.hash}`)
    } catch (_err) {}
  }

  return (
    <div className="min-h-screen bg-background">
      <main className="mx-auto max-w-4xl px-4 py-6 lg:px-6">
        <div className="mb-4 flex flex-wrap items-center gap-3">
          <Link
            href="/"
            className="inline-flex items-center gap-1.5 rounded-md border border-border bg-card px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
          >
            <ArrowLeft className="size-4" />
            Back To Records
          </Link>
          <div className="text-xs text-muted-foreground">
            {channelLabel && <span className="mr-3">Channel: {channelLabel}</span>}
            {threadTs && <span>Thread: {threadTs}</span>}
          </div>
        </div>

        {loading && (
          <div className="flex items-center gap-2 rounded-lg border border-border bg-card px-4 py-12 text-sm text-muted-foreground">
            <LoaderCircle className="size-4 animate-spin" />
            Loading thread messages...
          </div>
        )}

        {!loading && error && (
          <div className="rounded-lg border border-border bg-card px-4 py-6 text-sm text-muted-foreground">
            {error}
          </div>
        )}

        {!loading && !error && messages.length === 0 && (
          <div className="rounded-lg border border-border bg-card px-4 py-6 text-sm text-muted-foreground">
            No messages found for this thread.
          </div>
        )}

        {!loading && !error && messages.length > 0 && (
          <div className="rounded-lg border border-border bg-card">
            <div className="border-b border-border px-4 py-3 text-xs text-muted-foreground">
              {messages.length} {messages.length === 1 ? "message" : "messages"}
              {search && (
                <span className="ml-2">
                  • Search: <span className="text-foreground">{search}</span>
                </span>
              )}
            </div>

            <div className="divide-y divide-border/60 px-4">
              {messages.map((msg) => (
                <div key={msg.id} className="flex gap-3 py-3">
                  <Avatar className="size-8 shrink-0">
                    {msg.author.avatarUrl && (
                      <AvatarImage src={msg.author.avatarUrl} alt={msg.author.name} />
                    )}
                    <AvatarFallback className={`${msg.author.color} text-[10px] font-medium text-foreground`}>
                      {msg.author.initials}
                    </AvatarFallback>
                  </Avatar>

                  <div className="min-w-0 flex-1">
                    <div className="mb-1 flex flex-wrap items-center gap-x-2 gap-y-0.5">
                      <span className="text-xs font-medium text-foreground">{msg.author.name}</span>
                      <span className="text-[10px] text-muted-foreground" title={msg.timestampIso}>
                        {msg.timestamp}
                      </span>
                    </div>

                    <div
                      className={messageHtmlClassName()}
                      onClickCapture={handleMessageClickCapture}
                      dangerouslySetInnerHTML={{ __html: msg.messageHtml }}
                    />

                    <div className="mt-2 flex flex-wrap items-center gap-2">
                      {(msg.files ?? []).map((file, idx) => (
                        <a
                          key={`${msg.id}:${idx}`}
                          href={file.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="inline-flex items-center gap-1.5 rounded-md bg-secondary px-2 py-1 text-[11px] text-muted-foreground transition-colors hover:text-foreground"
                        >
                          <Paperclip className="size-3" />
                          {file.name}
                        </a>
                      ))}
                      {msg.reactions != null && msg.reactions > 0 && (
                        <span className="inline-flex items-center gap-1 text-[11px] text-muted-foreground">
                          <Heart className="size-3" />
                          {msg.reactions}
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </main>
    </div>
  )
}

export default function ThreadPage() {
  return (
    <Suspense fallback={<ThreadPageFallback />}>
      <ThreadPageContent />
    </Suspense>
  )
}
