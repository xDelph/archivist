"use client"

import { useEffect, useMemo, useRef, useState } from "react"
import { useRouter } from "next/navigation"
import {
  MessageSquare,
  Heart,
  Users,
  FileImage,
  FileText,
  Code2,
  Link2,
  ExternalLink,
  ChevronDown,
  Paperclip,
  ChevronLeft,
  ChevronRight,
  Download,
  X,
} from "lucide-react"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { Dialog, DialogContent } from "@/components/ui/dialog"
import { formatMessageTimestamp } from "@/lib/datetime"
import { extractSlackUrlsInOrder } from "@/lib/slack-links"
import type { SlackThread, ThreadFile, ThreadMessage } from "@/lib/types"

interface ThreadCardProps {
  thread: SlackThread
  rank: number
  expanded: boolean
  onExpandedChange: (nextExpanded: boolean) => void
  maxScore?: number
  density?: "normal" | "compact"
  onLoadThreadMessages?: (thread: SlackThread) => Promise<ThreadMessage[]>
  onMentionClick?: (userName: string) => void
  onChannelClick?: (channelName: string) => void
}

interface ViewerFile extends ThreadFile {
  key: string
}

interface LinkPreviewPayload {
  url: string
  title: string | null
  description: string | null
  image: string | null
  siteName: string | null
}

interface LinkPreviewState {
  loading: boolean
  payload: LinkPreviewPayload | null
}

const linkPreviewCache = new Map<string, LinkPreviewPayload | null>()
const linkPreviewInflight = new Map<string, Promise<LinkPreviewPayload | null>>()
const linkPreviewQueue: Array<() => void> = []
const LINK_PREVIEW_CONCURRENCY = 3
const COLLAPSE_SCROLL_TOP_OFFSET_PX = 96
const THREAD_LOADING_INDICATOR_DELAY_MS = 180
let activeLinkPreviewRequests = 0

function drainLinkPreviewQueue(): void {
  while (
    activeLinkPreviewRequests < LINK_PREVIEW_CONCURRENCY &&
    linkPreviewQueue.length > 0
  ) {
    const task = linkPreviewQueue.shift()
    if (!task) return
    activeLinkPreviewRequests += 1
    task()
  }
}

async function scheduleLinkPreviewFetch(url: string): Promise<LinkPreviewPayload | null> {
  return await new Promise<LinkPreviewPayload | null>((resolve) => {
    const run = () => {
      fetchLinkPreview(url)
        .then(resolve)
        .catch(() => resolve(null))
        .finally(() => {
          activeLinkPreviewRequests = Math.max(activeLinkPreviewRequests - 1, 0)
          drainLinkPreviewQueue()
        })
    }
    linkPreviewQueue.push(run)
    drainLinkPreviewQueue()
  })
}

async function fetchLinkPreview(url: string): Promise<LinkPreviewPayload | null> {
  const endpoint = `/api/link-preview?url=${encodeURIComponent(url)}`
  const response = await fetch(endpoint, { method: "GET", cache: "force-cache" })
  if (!response.ok) return null
  const payload = (await response.json()) as LinkPreviewPayload | null
  return payload
}

async function resolveLinkPreview(url: string): Promise<LinkPreviewPayload | null> {
  if (linkPreviewCache.has(url)) {
    return linkPreviewCache.get(url) ?? null
  }
  const existing = linkPreviewInflight.get(url)
  if (existing) return existing

  const request = scheduleLinkPreviewFetch(url)
    .then((payload) => {
      linkPreviewCache.set(url, payload)
      return payload
    })
    .catch(() => {
      linkPreviewCache.set(url, null)
      return null
    })
    .finally(() => {
      linkPreviewInflight.delete(url)
    })
  linkPreviewInflight.set(url, request)
  return request
}

function useLinkPreviews(urls: string[]): Record<string, LinkPreviewState> {
  const [states, setStates] = useState<Record<string, LinkPreviewState>>({})

  useEffect(() => {
    if (urls.length === 0) {
      setStates({})
      return
    }

    const nextStates: Record<string, LinkPreviewState> = {}
    for (const url of urls) {
      if (linkPreviewCache.has(url)) {
        nextStates[url] = {
          loading: false,
          payload: linkPreviewCache.get(url) ?? null,
        }
      } else {
        nextStates[url] = { loading: true, payload: null }
      }
    }
    setStates(nextStates)

    let cancelled = false
    let timeoutId: number | null = null
    let idleId: number | null = null

    const startFetches = () => {
      for (const url of urls) {
        if (linkPreviewCache.has(url)) continue
        void resolveLinkPreview(url).then((payload) => {
          if (cancelled) return
          setStates((current) => {
            if (!current[url]) return current
            return {
              ...current,
              [url]: { loading: false, payload },
            }
          })
        })
      }
    }

    // Defer previews slightly so thread message rendering wins first.
    timeoutId = window.setTimeout(() => {
      if ("requestIdleCallback" in window) {
        idleId = window.requestIdleCallback(() => {
          if (!cancelled) startFetches()
        })
      } else {
        startFetches()
      }
    }, 120)

    return () => {
      cancelled = true
      if (timeoutId != null) window.clearTimeout(timeoutId)
      if (idleId != null && "cancelIdleCallback" in window) {
        window.cancelIdleCallback(idleId)
      }
    }
  }, [urls])

  return states
}

function getClosestFromTarget(target: EventTarget | null, selector: string): Element | null {
  if (target instanceof Element) {
    return target.closest(selector)
  }
  if (target instanceof Node && target.parentElement) {
    return target.parentElement.closest(selector)
  }
  return null
}

function getInternalThreadPathFromClickTarget(target: EventTarget | null): string | null {
  const anchor = getClosestFromTarget(target, "a")
  if (!anchor) return null
  const href = anchor.getAttribute("href")
  if (!href) return null

  try {
    const url = new URL(href, window.location.href)
    if (url.origin !== window.location.origin || url.pathname !== "/record/thread") {
      return null
    }
    return `${url.pathname}${url.search}${url.hash}`
  } catch (_err) {
    return null
  }
}

function FileIcon({ type }: { type?: "image" | "pdf" | "code" | "link" }) {
  switch (type) {
    case "image":
      return <FileImage className="size-3.5" />
    case "pdf":
      return <FileText className="size-3.5" />
    case "code":
      return <Code2 className="size-3.5" />
    case "link":
      return <Link2 className="size-3.5" />
    default:
      return null
  }
}

function inferFileType(mimetype: string): "image" | "pdf" | "code" | "link" {
  if (mimetype.startsWith("image/")) return "image"
  if (mimetype.startsWith("video/")) return "link"
  if (mimetype === "application/pdf") return "pdf"
  if (mimetype.startsWith("text/") || mimetype.includes("json")) return "code"
  return "link"
}

function FileViewerContent({ file }: { file: ViewerFile }) {
  if (file.mimetype.startsWith("image/")) {
    return (
      <img
        src={file.url}
        alt={file.name}
        className="max-h-[70vh] w-full object-contain"
      />
    )
  }

  if (file.mimetype.startsWith("video/")) {
    return <video src={file.url} controls className="max-h-[70vh] w-full" />
  }

  return (
    <iframe
      src={file.url}
      title={file.name}
      className="h-[70vh] w-full border-0"
    />
  )
}

function ThreadMessageItem({
  msg,
  compact,
  onOpenFile,
  onMentionClick,
  onChannelClick,
  onNavigateToThread,
}: {
  msg: ThreadMessage
  compact: boolean
  onOpenFile: (fileKey: string) => void
  onMentionClick?: (userName: string) => void
  onChannelClick?: (channelName: string) => void
  onNavigateToThread?: (path: string) => void
}) {
  const reactionDetails = (msg.reactionDetails ?? []).filter((reaction) => reaction.count > 0)
  const formattedTimestamp = formatMessageTimestamp(msg.timestampIso, msg.ts)
  const previewLinks = useMemo(() => {
    const extracted = extractSlackUrlsInOrder(msg.message)
    const fileUrls = new Set((msg.files ?? []).map((file) => file.url))
    return extracted.filter((url) => !fileUrls.has(url))
  }, [msg.files, msg.message])
  const linkPreviews = useLinkPreviews(previewLinks)

  function handleMessageClick(event: React.MouseEvent<HTMLElement>): void {
    const mention = getClosestFromTarget(event.target, ".mention")
    if (mention) {
      const raw = mention.textContent?.trim() ?? ""
      if (raw.startsWith("@") && onMentionClick) {
        const userName = raw.slice(1).trim()
        if (
          !userName ||
          userName === "here" ||
          userName === "channel" ||
          userName === "everyone"
        ) {
          return
        }

        event.preventDefault()
        event.stopPropagation()
        onMentionClick(userName)
        return
      }

      if (raw.startsWith("#") && onChannelClick) {
        const channelName = raw.slice(1).trim()
        if (!channelName) return

        event.preventDefault()
        event.stopPropagation()
        onChannelClick(`#${channelName}`)
        return
      }
    }

    if (!onNavigateToThread) return
    const nextPath = getInternalThreadPathFromClickTarget(event.target)
    if (!nextPath) return

    event.preventDefault()
    event.stopPropagation()
    onNavigateToThread(nextPath)
  }

  return (
    <div className={`flex gap-3 ${compact ? "py-2.5" : "py-3"}`}>
      <Avatar className={`${compact ? "size-6" : "size-7"} shrink-0`}>
        {msg.author.avatarUrl && (
          <AvatarImage src={msg.author.avatarUrl} alt={msg.author.name} />
        )}
        <AvatarFallback
          className={`${msg.author.color} text-[10px] font-medium text-foreground`}
        >
          {msg.author.initials}
        </AvatarFallback>
      </Avatar>
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-0.5">
          <span className="text-xs font-medium text-foreground">{msg.author.name}</span>
          <span className="text-[10px] text-muted-foreground" title={msg.timestampIso}>
            {formattedTimestamp}
          </span>
        </div>
        <div
          className={`slack-text break-words text-secondary-foreground ${
            compact ? "text-[13px] leading-snug" : "text-sm leading-relaxed"
          } [&_.mention]:cursor-pointer [&_.mention]:font-medium [&_.mention]:text-primary [&_.mention:hover]:underline [&_a]:text-blue-500 [&_a]:underline [&_a]:underline-offset-2 [&_a:hover]:text-blue-400 [&_a.url-highlight]:text-red-500 [&_a.url-highlight:hover]:text-red-400 [&_mark.search-highlight]:rounded-sm [&_mark.search-highlight]:bg-red-500/20 [&_mark.search-highlight]:px-0.5 [&_mark.search-highlight]:text-red-500 [&_code.slack-inline-code]:rounded [&_code.slack-inline-code]:bg-secondary [&_code.slack-inline-code]:px-1 [&_code.slack-inline-code]:py-0.5 [&_code.slack-inline-code]:font-mono [&_code.slack-inline-code]:text-[0.85em] [&_pre.slack-code]:mt-2 [&_pre.slack-code]:overflow-x-auto [&_pre.slack-code]:rounded-md [&_pre.slack-code]:border [&_pre.slack-code]:border-border/70 [&_pre.slack-code]:bg-secondary/70 [&_pre.slack-code]:p-3 [&_pre.slack-code]:font-mono [&_pre.slack-code]:text-[12px]`}
          onClickCapture={handleMessageClick}
          dangerouslySetInnerHTML={{ __html: msg.messageHtml }}
        />
        <div className="flex flex-col gap-2">
          {(msg.files ?? []).length > 0 && (
            <div className="grid w-full grid-cols-1 gap-2 sm:grid-cols-2">
              {(msg.files ?? []).map((file, idx) => {
                const fileType = inferFileType(file.mimetype)
                return (
                  <button
                    key={`${msg.id}:${idx}`}
                    type="button"
                    onClick={() => onOpenFile(`${msg.id}:${idx}`)}
                    className="overflow-hidden rounded-md border border-border/70 bg-secondary/50 text-left transition-colors hover:border-primary/50 hover:bg-secondary"
                  >
                    {fileType === "image" ? (
                      <>
                        <img
                          src={file.url}
                          alt={file.name}
                          loading="lazy"
                          className="h-28 w-full object-cover"
                        />
                        <div className="flex items-center gap-1.5 px-2 py-1.5 text-[11px] text-muted-foreground">
                          <Paperclip className="size-3" />
                          <span className="truncate">{file.name}</span>
                        </div>
                      </>
                    ) : (
                      <div className="flex min-h-[3.5rem] items-center gap-2 px-2.5 py-2">
                        <FileIcon type={fileType} />
                        <div className="min-w-0">
                          <p className="truncate text-xs text-foreground">{file.name}</p>
                          <p className="truncate text-[10px] text-muted-foreground">
                            {file.mimetype}
                          </p>
                        </div>
                      </div>
                    )}
                  </button>
                )
              })}
            </div>
          )}
          {previewLinks.length > 0 && (
            <div className="flex w-full flex-col gap-2">
              {previewLinks.map((url, idx) => {
                const state = linkPreviews[url]
                const preview = state?.payload
                return (
                  <a
                    key={`${msg.id}:link:${idx}:${url}`}
                    href={url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="group overflow-hidden rounded-md border border-border/70 bg-secondary/40 text-left transition-colors hover:border-primary/50 hover:bg-secondary/70"
                  >
                    {preview?.image && (
                      <img
                        src={preview.image}
                        alt={preview.title ?? preview.siteName ?? url}
                        loading="lazy"
                        className="h-32 w-full object-cover"
                      />
                    )}
                    <div className="flex items-start gap-2 px-3 py-2">
                      <Link2 className="mt-0.5 size-3.5 shrink-0 text-muted-foreground group-hover:text-foreground" />
                      <div className="min-w-0">
                        <p className="line-clamp-2 text-xs font-medium text-foreground">
                          {preview?.title ?? url}
                        </p>
                        {preview?.description && (
                          <p className="mt-1 line-clamp-2 text-[11px] text-muted-foreground">
                            {preview.description}
                          </p>
                        )}
                        <p className="mt-1 truncate text-[10px] text-muted-foreground">
                          {preview?.siteName ?? new URL(url).hostname}
                          {state?.loading ? " · loading preview…" : ""}
                        </p>
                      </div>
                    </div>
                  </a>
                )
              })}
            </div>
          )}
          <div className="flex flex-wrap items-center gap-2">
          {reactionDetails.length > 0 && (
            <div className="flex flex-wrap items-center gap-1">
              {reactionDetails.map((reaction) => (
                <span
                  key={`${msg.id}:rx:${reaction.name}`}
                  className="inline-flex items-center gap-1 rounded-full border border-border/70 bg-secondary/70 px-1.5 py-0.5 text-[11px] text-muted-foreground"
                  title={reaction.name}
                >
                  <span aria-hidden>{reaction.emoji}</span>
                  <span>{reaction.count}</span>
                </span>
              ))}
            </div>
          )}
          {reactionDetails.length === 0 && msg.reactions != null && msg.reactions > 0 && (
            <span className="inline-flex items-center gap-1 text-[11px] text-muted-foreground">
              <Heart className="size-3" />
              {msg.reactions}
            </span>
          )}
          </div>
        </div>
      </div>
    </div>
  )
}

export function ThreadCard({
  thread,
  rank,
  expanded,
  onExpandedChange,
  maxScore,
  density = "normal",
  onLoadThreadMessages,
  onMentionClick,
  onChannelClick,
}: ThreadCardProps) {
  const router = useRouter()
  const compact = density === "compact"

  const [messages, setMessages] = useState<ThreadMessage[] | null>(
    thread.threadMessages ?? null
  )
  const [loading, setLoading] = useState(false)
  const [showLoadingState, setShowLoadingState] = useState(false)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [viewerIndex, setViewerIndex] = useState<number | null>(null)
  const cardRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    setMessages(thread.threadMessages ?? null)
    setLoading(false)
    setShowLoadingState(false)
    setLoadError(null)
    setViewerIndex(null)
  }, [thread.id, thread.threadMessages])

  useEffect(() => {
    if (!loading) {
      setShowLoadingState(false)
      return
    }

    const timeoutId = window.setTimeout(() => {
      setShowLoadingState(true)
    }, THREAD_LOADING_INDICATOR_DELAY_MS)

    return () => {
      window.clearTimeout(timeoutId)
    }
  }, [loading])

  const scoreScaleMax = Math.max(maxScore ?? thread.score, 1)
  const scorePercent = Math.min((thread.score / scoreScaleMax) * 100, 100)
  const canExpand = Boolean(onLoadThreadMessages || thread.threadMessages)
  const loadedMessages = messages ?? []
  const loadedReplyCount = loadedMessages.length > 0 ? Math.max(loadedMessages.length - 1, 0) : 0
  const replyCount = messages ? loadedReplyCount : thread.replies
  const messageCount = messages ? loadedMessages.length : thread.replies + 1

  const participants = loadedMessages.reduce<ThreadMessage["author"][]>((acc, message) => {
    if (!acc.find((author) => author.name === message.author.name)) {
      acc.push(message.author)
    }
    return acc
  }, [])

  const viewerFiles = useMemo<ViewerFile[]>(
    () =>
      loadedMessages.flatMap((message) =>
        (message.files ?? []).map((file, idx) => ({
          ...file,
          key: `${message.id}:${idx}`,
        }))
      ),
    [loadedMessages]
  )

  const activeViewerFile = viewerIndex == null ? null : viewerFiles[viewerIndex] ?? null

  useEffect(() => {
    if (viewerIndex == null) return

    function onKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape") {
        setViewerIndex(null)
      }
      if (event.key === "ArrowLeft") {
        setViewerIndex((current) => {
          if (current == null) return current
          return current > 0 ? current - 1 : current
        })
      }
      if (event.key === "ArrowRight") {
        setViewerIndex((current) => {
          if (current == null) return current
          return current < viewerFiles.length - 1 ? current + 1 : current
        })
      }
    }

    window.addEventListener("keydown", onKeyDown)
    return () => {
      window.removeEventListener("keydown", onKeyDown)
    }
  }, [viewerIndex, viewerFiles.length])

  function scrollPreviewIntoViewAfterCollapse(preview: HTMLDivElement): void {
    const rect = preview.getBoundingClientRect()
    const viewportHeight = window.innerHeight || document.documentElement.clientHeight
    const shouldAdjust =
      rect.top < COLLAPSE_SCROLL_TOP_OFFSET_PX || rect.bottom > viewportHeight

    if (!shouldAdjust) return

    const targetTop = Math.max(
      window.scrollY + rect.top - COLLAPSE_SCROLL_TOP_OFFSET_PX,
      0
    )
    window.scrollTo({ top: targetTop, behavior: "smooth" })
  }

  async function toggleExpanded(): Promise<void> {
    const next = !expanded
    if (!next) {
      onExpandedChange(false)
      const preview = cardRef.current
      if (preview) {
        window.requestAnimationFrame(() => {
          window.requestAnimationFrame(() => {
            scrollPreviewIntoViewAfterCollapse(preview)
          })
        })
      }
      return
    }

    onExpandedChange(true)

    if (!canExpand || messages !== null || !onLoadThreadMessages || loading) {
      return
    }

    setLoading(true)
    setLoadError(null)
    try {
      const fetchedMessages = await onLoadThreadMessages(thread)
      setMessages(fetchedMessages)
    } catch (_err) {
      try {
        const retryMessages = await onLoadThreadMessages(thread)
        setMessages(retryMessages)
      } catch (_retryErr) {
        setLoadError("Failed to load thread messages.")
        setMessages([])
      }
    } finally {
      setLoading(false)
    }
  }

  function openFileByKey(fileKey: string): void {
    const index = viewerFiles.findIndex((file) => file.key === fileKey)
    if (index >= 0) {
      setViewerIndex(index)
    }
  }

  function navigateToThread(path: string): void {
    router.push(path)
  }

  function handlePreviewContentClick(event: React.MouseEvent<HTMLElement>): void {
    const mention = getClosestFromTarget(event.target, ".mention")
    if (mention) {
      const raw = mention.textContent?.trim() ?? ""
      if (raw.startsWith("@") && onMentionClick) {
        const userName = raw.slice(1).trim()
        if (
          !userName ||
          userName === "here" ||
          userName === "channel" ||
          userName === "everyone"
        ) {
          return
        }

        event.preventDefault()
        event.stopPropagation()
        onMentionClick(userName)
        return
      }

      if (raw.startsWith("#") && onChannelClick) {
        const channelName = raw.slice(1).trim()
        if (!channelName) return

        event.preventDefault()
        event.stopPropagation()
        onChannelClick(`#${channelName}`)
        return
      }
    }

    const nextPath = getInternalThreadPathFromClickTarget(event.target)
    if (!nextPath) return

    event.preventDefault()
    event.stopPropagation()
    navigateToThread(nextPath)
  }

  return (
    <>
      <div
        ref={cardRef}
        className={`group relative rounded-lg border transition-colors ${
          expanded
            ? "border-primary/40 bg-card"
            : "border-border bg-card hover:border-primary/30 hover:bg-card/80"
        }`}
      >
        <div
          role="button"
          tabIndex={0}
          className={`flex w-full cursor-pointer items-start text-left ${
            compact ? "gap-3 px-3 py-3" : "gap-4 p-4"
          }`}
          onClick={() => void toggleExpanded()}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === " ") {
              event.preventDefault()
              void toggleExpanded()
            }
          }}
          aria-expanded={expanded}
          aria-controls={`thread-messages-${thread.id}`}
        >
          <div className="flex w-6 shrink-0 items-start justify-center pt-0.5">
            <span
              className={`text-sm font-semibold ${
                rank <= 3 ? "text-primary" : "text-muted-foreground"
              }`}
            >
              {rank}
            </span>
          </div>

          <Avatar className={`${compact ? "size-8" : "size-9"} shrink-0`}>
            {thread.author.avatarUrl && (
              <AvatarImage src={thread.author.avatarUrl} alt={thread.author.name} />
            )}
            <AvatarFallback
              className={`${thread.author.color} text-xs font-medium text-foreground`}
            >
              {thread.author.initials}
            </AvatarFallback>
          </Avatar>

          <div className={`flex min-w-0 flex-1 flex-col ${compact ? "gap-1.5" : "gap-2"}`}>
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-foreground">{thread.author.name}</span>
              <button
                type="button"
                className={`inline-flex cursor-pointer items-center rounded-md px-2 py-0.5 text-xs font-medium ${thread.channelColor} hover:brightness-110`}
                onClick={(event) => {
                  if (!onChannelClick) return
                  event.preventDefault()
                  event.stopPropagation()
                  onChannelClick(thread.channel)
                }}
              >
                {thread.channel}
              </button>
              {thread.hasFiles && (
                <span className="inline-flex items-center gap-1 rounded-md bg-secondary px-1.5 py-0.5 text-xs text-muted-foreground">
                  <FileIcon type={thread.fileType} />
                </span>
              )}
              <span className="ml-auto text-xs text-muted-foreground">{thread.date}</span>
            </div>

            <div
              className={`slack-text break-words text-secondary-foreground ${
                compact ? "text-[13px] leading-snug" : "text-sm leading-relaxed"
              } line-clamp-2 [&_.mention]:cursor-pointer [&_.mention]:font-medium [&_.mention]:text-primary [&_.mention:hover]:underline [&_a]:text-blue-500 [&_a]:underline [&_a]:underline-offset-2 [&_a:hover]:text-blue-400 [&_a.url-highlight]:text-red-500 [&_a.url-highlight:hover]:text-red-400 [&_mark.search-highlight]:rounded-sm [&_mark.search-highlight]:bg-red-500/20 [&_mark.search-highlight]:px-0.5 [&_mark.search-highlight]:text-red-500 [&_code.slack-inline-code]:rounded [&_code.slack-inline-code]:bg-secondary [&_code.slack-inline-code]:px-1 [&_code.slack-inline-code]:py-0.5 [&_code.slack-inline-code]:font-mono [&_code.slack-inline-code]:text-[0.85em] [&_pre.slack-code]:mt-2 [&_pre.slack-code]:overflow-x-auto [&_pre.slack-code]:rounded-md [&_pre.slack-code]:border [&_pre.slack-code]:border-border/70 [&_pre.slack-code]:bg-secondary/70 [&_pre.slack-code]:p-3 [&_pre.slack-code]:font-mono [&_pre.slack-code]:text-[12px]`}
              onClickCapture={handlePreviewContentClick}
              dangerouslySetInnerHTML={{ __html: thread.messageHtml }}
            />

            <div className="flex items-center gap-4">
              <div className="flex items-center gap-3 text-xs text-muted-foreground">
                <span className="inline-flex items-center gap-1">
                  <MessageSquare className="size-3.5" />
                  {thread.replies}
                </span>
                <span className="inline-flex items-center gap-1">
                  <Heart className="size-3.5" />
                  {thread.reactions}
                </span>
                <span className="inline-flex items-center gap-1">
                  <Users className="size-3.5" />
                  {thread.participants}
                </span>
              </div>

              <div className="ml-auto flex items-center gap-2">
                <div className="hidden h-1.5 w-20 overflow-hidden rounded-full bg-secondary sm:block">
                  <div
                    className="h-full rounded-full bg-primary transition-all"
                    style={{ width: `${scorePercent}%` }}
                  />
                </div>
                <span className="text-xs font-medium text-primary">{thread.score}</span>
              </div>

              {thread.url && (
                <a
                  href={thread.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
                  onClick={(event) => event.stopPropagation()}
                >
                  <ExternalLink className="size-3.5" />
                  <span className="sr-only">Open link</span>
                </a>
              )}

              {canExpand && (
                <div className="flex items-center gap-1 text-xs text-muted-foreground">
                  <ChevronDown
                    className={`size-4 transition-transform duration-200 ${
                      expanded ? "rotate-180" : ""
                    }`}
                  />
                </div>
              )}
            </div>
          </div>
        </div>

        {expanded && canExpand && (
          <div
            id={`thread-messages-${thread.id}`}
            className="border-t border-border motion-safe:animate-in motion-safe:fade-in-0 motion-safe:slide-in-from-top-1 motion-safe:duration-200"
          >
            {loading && showLoadingState && (
              <div className="px-4 py-4 pl-14 sm:pl-[4.25rem]">
                <div className="space-y-2 motion-safe:animate-in motion-safe:fade-in-0 motion-safe:duration-200">
                  <div className="h-2.5 w-24 rounded-full bg-secondary/90" />
                  <div className="h-2 w-full max-w-[28rem] rounded-full bg-secondary/70" />
                  <div className="h-2 w-full max-w-[22rem] rounded-full bg-secondary/70" />
                </div>
                <span className="sr-only">Loading thread...</span>
              </div>
            )}

            {!loading && loadError && (
              <div className="px-4 py-4 pl-14 text-xs text-destructive sm:pl-[4.25rem]">
                {loadError}
              </div>
            )}

            {!loading && !loadError && loadedMessages.length > 0 && (
              <div className="motion-safe:animate-in motion-safe:fade-in-0 motion-safe:slide-in-from-top-2 motion-safe:duration-300">
                <div className="flex items-center gap-2 bg-secondary/50 px-4 py-2">
                  <MessageSquare className="size-3.5 text-primary" />
                  <span className="text-xs font-medium text-foreground">
                    {replyCount > 0
                      ? `${replyCount} ${replyCount === 1 ? "reply" : "replies"} in thread`
                      : `${messageCount} ${messageCount === 1 ? "message" : "messages"} in thread`}
                  </span>
                  <div className="ml-auto flex -space-x-1.5">
                    {participants.slice(0, 5).map((author) => (
                      <Avatar
                        key={`${thread.id}:${author.name}`}
                        className="size-5 ring-1 ring-card"
                      >
                        {author.avatarUrl && (
                          <AvatarImage src={author.avatarUrl} alt={author.name} />
                        )}
                        <AvatarFallback
                          className={`${author.color} text-[8px] font-medium text-foreground`}
                        >
                          {author.initials}
                        </AvatarFallback>
                      </Avatar>
                    ))}
                  </div>
                </div>

                <div className="flex flex-col divide-y divide-border/50 px-4 pl-14 sm:pl-[4.25rem]">
                  {loadedMessages.map((message) => (
                    <ThreadMessageItem
                      key={message.id}
                      msg={message}
                      compact={compact}
                      onOpenFile={openFileByKey}
                      onMentionClick={onMentionClick}
                      onChannelClick={onChannelClick}
                      onNavigateToThread={navigateToThread}
                    />
                  ))}
                </div>

                <button
                  type="button"
                  className="flex w-full items-center justify-between border-t border-border/50 px-4 py-2.5 pl-14 text-[11px] text-muted-foreground transition-colors hover:bg-secondary/40 hover:text-foreground sm:pl-[4.25rem]"
                  onClick={() => void toggleExpanded()}
                >
                  <span>End of thread</span>
                  <span className="flex items-center gap-1">
                    Collapse
                    <ChevronDown className="size-3 rotate-180" />
                  </span>
                </button>
              </div>
            )}

            {!loading && !loadError && loadedMessages.length === 0 && (
              <div className="px-4 py-4 pl-14 text-xs text-muted-foreground sm:pl-[4.25rem]">
                No replies in this thread.
              </div>
            )}
          </div>
        )}
      </div>

      <Dialog
        open={viewerIndex !== null}
        onOpenChange={(open) => {
          if (!open) {
            setViewerIndex(null)
          }
        }}
      >
        {activeViewerFile && (
          <DialogContent
            className="max-w-5xl overflow-hidden border-border bg-card p-0"
            showCloseButton={false}
          >
            <div className="flex items-center justify-between border-b border-border px-4 py-3">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium text-foreground">
                  {activeViewerFile.name}
                </p>
                <p className="text-xs text-muted-foreground">
                  {viewerIndex != null ? `${viewerIndex + 1} / ${viewerFiles.length}` : ""}
                </p>
              </div>
              <div className="flex items-center gap-2">
                <a
                  href={activeViewerFile.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="inline-flex items-center gap-1 rounded-md border border-border px-2 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
                >
                  <Download className="size-3" />
                  Download
                </a>
                <button
                  type="button"
                  className="rounded-md border border-border p-1.5 text-muted-foreground transition-colors hover:text-foreground"
                  onClick={() => setViewerIndex(null)}
                >
                  <X className="size-4" />
                </button>
              </div>
            </div>

            <div className="relative bg-black/30 p-4">
              <FileViewerContent file={activeViewerFile} />
              <button
                type="button"
                className="absolute left-3 top-1/2 -translate-y-1/2 rounded-full bg-background/80 p-1.5 text-foreground disabled:opacity-40"
                onClick={() =>
                  setViewerIndex((idx) => (idx != null && idx > 0 ? idx - 1 : idx))
                }
                disabled={(viewerIndex ?? 0) <= 0}
              >
                <ChevronLeft className="size-4" />
              </button>
              <button
                type="button"
                className="absolute right-3 top-1/2 -translate-y-1/2 rounded-full bg-background/80 p-1.5 text-foreground disabled:opacity-40"
                onClick={() =>
                  setViewerIndex((idx) =>
                    idx != null && idx < viewerFiles.length - 1 ? idx + 1 : idx
                  )
                }
                disabled={(viewerIndex ?? 0) >= viewerFiles.length - 1}
              >
                <ChevronRight className="size-4" />
              </button>
            </div>
          </DialogContent>
        )}
      </Dialog>
    </>
  )
}
