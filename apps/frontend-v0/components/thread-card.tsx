"use client"

import { useEffect, useMemo, useState } from "react"
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
  LoaderCircle,
  ChevronLeft,
  ChevronRight,
  Download,
  X,
} from "lucide-react"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import { Dialog, DialogContent } from "@/components/ui/dialog"
import type { SlackThread, ThreadFile, ThreadMessage } from "@/lib/types"

interface ThreadCardProps {
  thread: SlackThread
  rank: number
  density?: "normal" | "compact"
  onLoadThreadMessages?: (thread: SlackThread) => Promise<ThreadMessage[]>
}

interface ViewerFile extends ThreadFile {
  key: string
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
}: {
  msg: ThreadMessage
  compact: boolean
  onOpenFile: (fileKey: string) => void
}) {
  return (
    <div className={`flex gap-3 ${compact ? "py-2.5" : "py-3"}`}>
      <Avatar className={`${compact ? "size-6" : "size-7"} shrink-0`}>
        <AvatarFallback
          className={`${msg.author.color} text-[10px] font-medium text-foreground`}
        >
          {msg.author.initials}
        </AvatarFallback>
      </Avatar>
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-foreground">{msg.author.name}</span>
          <span className="text-[10px] text-muted-foreground">{msg.timestamp}</span>
        </div>
        <p className={`text-secondary-foreground ${compact ? "text-[13px] leading-snug" : "text-sm leading-relaxed"}`}>
          {msg.message}
        </p>
        <div className="flex flex-wrap items-center gap-2">
          {(msg.files ?? []).map((file, idx) => (
            <button
              key={`${msg.id}:${idx}`}
              type="button"
              onClick={() => onOpenFile(`${msg.id}:${idx}`)}
              className="inline-flex items-center gap-1.5 rounded-md bg-secondary px-2 py-1 text-[11px] text-muted-foreground transition-colors hover:text-foreground"
            >
              <Paperclip className="size-3" />
              {file.name}
            </button>
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
  )
}

export function ThreadCard({
  thread,
  rank,
  density = "normal",
  onLoadThreadMessages,
}: ThreadCardProps) {
  const compact = density === "compact"

  const [expanded, setExpanded] = useState(false)
  const [messages, setMessages] = useState<ThreadMessage[] | null>(
    thread.threadMessages ?? null
  )
  const [loading, setLoading] = useState(false)
  const [loadError, setLoadError] = useState<string | null>(null)
  const [viewerIndex, setViewerIndex] = useState<number | null>(null)

  useEffect(() => {
    setExpanded(false)
    setMessages(thread.threadMessages ?? null)
    setLoading(false)
    setLoadError(null)
    setViewerIndex(null)
  }, [thread.id, thread.threadMessages])

  const maxScore = 2000
  const scorePercent = Math.min((thread.score / maxScore) * 100, 100)
  const fallbackCount = thread.threadMessages?.length ?? thread.replies
  const messageCount = messages?.length ?? fallbackCount
  const canExpand = fallbackCount > 0
  const loadedMessages = messages ?? []

  const participants = loadedMessages.reduce<ThreadMessage["author"][]>((acc, message) => {
    if (!acc.find((author) => author.initials === message.author.initials)) {
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

  async function toggleExpanded(): Promise<void> {
    const next = !expanded
    setExpanded(next)

    if (!next || !canExpand || messages !== null || !onLoadThreadMessages || loading) {
      return
    }

    setLoading(true)
    setLoadError(null)
    try {
      const fetchedMessages = await onLoadThreadMessages(thread)
      setMessages(fetchedMessages)
    } catch (_err) {
      setLoadError("Failed to load thread messages.")
      setMessages([])
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

  return (
    <>
      <div
        className={`group relative rounded-lg border transition-colors ${
          expanded
            ? "border-primary/40 bg-card"
            : "border-border bg-card hover:border-primary/30 hover:bg-card/80"
        }`}
      >
        <button
          type="button"
          className={`flex w-full cursor-pointer items-start text-left ${
            compact ? "gap-3 px-3 py-3" : "gap-4 p-4"
          }`}
          onClick={() => void toggleExpanded()}
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
            <AvatarFallback
              className={`${thread.author.color} text-xs font-medium text-foreground`}
            >
              {thread.author.initials}
            </AvatarFallback>
          </Avatar>

          <div className={`flex min-w-0 flex-1 flex-col ${compact ? "gap-1.5" : "gap-2"}`}>
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-foreground">{thread.author.name}</span>
              <span
                className={`inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium ${thread.channelColor}`}
              >
                {thread.channel}
              </span>
              {thread.hasFiles && (
                <span className="inline-flex items-center gap-1 rounded-md bg-secondary px-1.5 py-0.5 text-xs text-muted-foreground">
                  <FileIcon type={thread.fileType} />
                </span>
              )}
              <span className="ml-auto text-xs text-muted-foreground">{thread.date}</span>
            </div>

            <p
              className={`text-secondary-foreground ${
                compact ? "text-[13px] leading-snug" : "text-sm leading-relaxed"
              } ${expanded ? "" : "line-clamp-2"}`}
            >
              {thread.message}
            </p>

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
        </button>

        {expanded && canExpand && (
          <div id={`thread-messages-${thread.id}`} className="border-t border-border">
            {loading && (
              <div className="flex items-center gap-2 px-4 py-4 pl-14 text-xs text-muted-foreground sm:pl-[4.25rem]">
                <LoaderCircle className="size-3.5 animate-spin" />
                Loading thread...
              </div>
            )}

            {!loading && loadError && (
              <div className="px-4 py-4 pl-14 text-xs text-destructive sm:pl-[4.25rem]">
                {loadError}
              </div>
            )}

            {!loading && !loadError && loadedMessages.length > 0 && (
              <>
                <div className="flex items-center gap-2 bg-secondary/50 px-4 py-2">
                  <MessageSquare className="size-3.5 text-primary" />
                  <span className="text-xs font-medium text-foreground">
                    {messageCount} {messageCount === 1 ? "reply" : "replies"} in thread
                  </span>
                  <div className="ml-auto flex -space-x-1.5">
                    {participants.slice(0, 5).map((author) => (
                      <Avatar
                        key={`${thread.id}:${author.initials}`}
                        className="size-5 ring-1 ring-card"
                      >
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
                    />
                  ))}
                </div>

                <div className="flex items-center justify-between border-t border-border/50 px-4 py-2.5 pl-14 sm:pl-[4.25rem]">
                  <span className="text-[11px] text-muted-foreground">End of thread</span>
                  <button
                    type="button"
                    className="flex items-center gap-1 text-[11px] text-muted-foreground transition-colors hover:text-foreground"
                    onClick={() => setExpanded(false)}
                  >
                    Collapse
                    <ChevronDown className="size-3 rotate-180" />
                  </button>
                </div>
              </>
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
