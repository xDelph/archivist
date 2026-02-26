"use client"

import { useState } from "react"
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
} from "lucide-react"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import type { SlackThread, ThreadMessage } from "@/lib/mock-data"

interface ThreadCardProps {
  thread: SlackThread
  rank: number
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

function ThreadMessageItem({ msg }: { msg: ThreadMessage }) {
  return (
    <div className="flex gap-3 py-3">
      <Avatar className="size-7 shrink-0">
        <AvatarFallback
          className={`${msg.author.color} text-[10px] font-medium text-foreground`}
        >
          {msg.author.initials}
        </AvatarFallback>
      </Avatar>
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-foreground">
            {msg.author.name}
          </span>
          <span className="text-[10px] text-muted-foreground">
            {msg.timestamp}
          </span>
        </div>
        <p className="text-sm leading-relaxed text-secondary-foreground">
          {msg.message}
        </p>
        <div className="flex items-center gap-3">
          {msg.hasFile && msg.fileName && (
            <span className="inline-flex items-center gap-1.5 rounded-md bg-secondary px-2 py-1 text-[11px] text-muted-foreground">
              <Paperclip className="size-3" />
              {msg.fileName}
            </span>
          )}
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

export function ThreadCard({ thread, rank }: ThreadCardProps) {
  const [expanded, setExpanded] = useState(false)
  const maxScore = 2000
  const scorePercent = Math.min((thread.score / maxScore) * 100, 100)
  const messageCount = thread.threadMessages?.length ?? 0

  return (
    <div
      className={`group relative rounded-lg border transition-colors ${
        expanded
          ? "border-primary/40 bg-card"
          : "border-border bg-card hover:border-primary/30 hover:bg-card/80"
      }`}
    >
      {/* Main card - clickable header */}
      <button
        type="button"
        className="flex w-full cursor-pointer items-start gap-4 p-4 text-left"
        onClick={() => setExpanded((prev) => !prev)}
        aria-expanded={expanded}
        aria-controls={`thread-messages-${thread.id}`}
      >
        {/* Rank */}
        <div className="flex w-6 shrink-0 items-start justify-center pt-0.5">
          <span
            className={`text-sm font-semibold ${
              rank <= 3 ? "text-primary" : "text-muted-foreground"
            }`}
          >
            {rank}
          </span>
        </div>

        {/* Avatar */}
        <Avatar className="size-9 shrink-0">
          <AvatarFallback
            className={`${thread.author.color} text-xs font-medium text-foreground`}
          >
            {thread.author.initials}
          </AvatarFallback>
        </Avatar>

        {/* Content */}
        <div className="flex min-w-0 flex-1 flex-col gap-2">
          {/* Header row */}
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-foreground">
              {thread.author.name}
            </span>
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
            <span className="ml-auto text-xs text-muted-foreground">
              {thread.date}
            </span>
          </div>

          {/* Message preview */}
          <p
            className={`text-sm leading-relaxed text-secondary-foreground ${
              expanded ? "" : "line-clamp-2"
            }`}
          >
            {thread.message}
          </p>

          {/* Footer */}
          <div className="flex items-center gap-4">
            {/* Metrics */}
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

            {/* Score bar */}
            <div className="ml-auto flex items-center gap-2">
              <div className="hidden h-1.5 w-20 overflow-hidden rounded-full bg-secondary sm:block">
                <div
                  className="h-full rounded-full bg-primary transition-all"
                  style={{ width: `${scorePercent}%` }}
                />
              </div>
              <span className="text-xs font-medium text-primary">
                {thread.score}
              </span>
            </div>

            {/* External link */}
            {thread.url && (
              <a
                href={thread.url}
                target="_blank"
                rel="noopener noreferrer"
                className="text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
                onClick={(e) => e.stopPropagation()}
              >
                <ExternalLink className="size-3.5" />
                <span className="sr-only">Open link</span>
              </a>
            )}

            {/* Expand indicator */}
            {messageCount > 0 && (
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

      {/* Expanded thread messages */}
      {expanded && messageCount > 0 && (
        <div
          id={`thread-messages-${thread.id}`}
          className="border-t border-border"
        >
          {/* Thread header bar */}
          <div className="flex items-center gap-2 bg-secondary/50 px-4 py-2">
            <MessageSquare className="size-3.5 text-primary" />
            <span className="text-xs font-medium text-foreground">
              {messageCount} {messageCount === 1 ? "reply" : "replies"} in
              thread
            </span>
            <div className="ml-auto flex -space-x-1.5">
              {thread.threadMessages
                .reduce<ThreadMessage["author"][]>((acc, m) => {
                  if (!acc.find((a) => a.initials === m.author.initials)) {
                    acc.push(m.author)
                  }
                  return acc
                }, [])
                .slice(0, 5)
                .map((author) => (
                  <Avatar key={author.initials} className="size-5 ring-1 ring-card">
                    <AvatarFallback
                      className={`${author.color} text-[8px] font-medium text-foreground`}
                    >
                      {author.initials}
                    </AvatarFallback>
                  </Avatar>
                ))}
            </div>
          </div>

          {/* Messages */}
          <div className="flex flex-col divide-y divide-border/50 px-4 pl-14 sm:pl-[4.25rem]">
            {thread.threadMessages.map((msg) => (
              <ThreadMessageItem key={msg.id} msg={msg} />
            ))}
          </div>

          {/* Thread footer */}
          <div className="flex items-center justify-between border-t border-border/50 px-4 py-2.5 pl-14 sm:pl-[4.25rem]">
            <span className="text-[11px] text-muted-foreground">
              End of thread
            </span>
            <button
              type="button"
              className="flex items-center gap-1 text-[11px] text-muted-foreground transition-colors hover:text-foreground"
              onClick={() => setExpanded(false)}
            >
              Collapse
              <ChevronDown className="size-3 rotate-180" />
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
