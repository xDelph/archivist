"use client"

import { Hash } from "lucide-react"
import { useMorphingText, useTwoPhaseNumber } from "@/hooks/use-two-phase-motion"
import type { ChannelStat } from "@/lib/types"
import { ScrollArea } from "@/components/ui/scroll-area"

interface ChannelSidebarProps {
  channels: ChannelStat[]
  selected: string | null
  onSelect: (channel: string | null) => void
  syncKey?: string
}

interface ChannelButtonProps {
  channel: ChannelStat
  selected: boolean
  onSelect: (channel: string | null) => void
  syncKey?: string
}

function ChannelButton({ channel, selected, onSelect, syncKey }: ChannelButtonProps) {
  const animatedCount = useTwoPhaseNumber(channel.count, undefined, syncKey)
  const animatedLabel = useMorphingText(channel.name.replace("#", ""), undefined, syncKey)
  const animatedPercentage = useTwoPhaseNumber(channel.percentage, undefined, syncKey)
  const countLabel = Math.max(0, Math.round(animatedCount)).toLocaleString()

  return (
    <button
      onClick={() => onSelect(channel.name)}
      className={`flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors ${
        selected
          ? "bg-primary/10 text-primary"
          : "text-muted-foreground hover:bg-secondary hover:text-foreground"
      }`}
    >
      <Hash className="size-3.5" />
      <span className="flex-1">{animatedLabel}</span>
      <div className="flex items-center gap-2">
        <div className="hidden h-1 w-10 overflow-hidden rounded-full bg-secondary md:block">
          <div
            className={`h-full rounded-full ${channel.color}`}
            style={{ width: `${Math.max(0, Math.min(100, animatedPercentage))}%` }}
          />
        </div>
        <span className="text-xs tabular-nums">{countLabel}</span>
      </div>
    </button>
  )
}

export function ChannelSidebar({
  channels,
  selected,
  onSelect,
  syncKey,
}: ChannelSidebarProps) {
  const totalThreads = channels.reduce((sum, channel) => sum + channel.count, 0)
  const animatedTotalThreads = useTwoPhaseNumber(totalThreads, undefined, syncKey)
  const totalThreadsLabel = Math.max(0, Math.round(animatedTotalThreads)).toLocaleString()

  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <h3 className="mb-3 text-sm font-medium text-foreground">Channels</h3>
      <div className="flex flex-col gap-1">
        <button
          onClick={() => onSelect(null)}
          className={`flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors ${
            selected === null
              ? "bg-primary/10 text-primary"
              : "text-muted-foreground hover:bg-secondary hover:text-foreground"
          }`}
        >
          <Hash className="size-3.5" />
          <span className="flex-1">All channels</span>
          <span className="text-xs tabular-nums">{totalThreadsLabel}</span>
        </button>
        <ScrollArea className="h-[10rem] pr-1">
          <div className="flex flex-col gap-1">
            {channels.map((channel, index) => (
              <ChannelButton
                key={`channel-row-${index}`}
                channel={channel}
                selected={selected === channel.name}
                onSelect={onSelect}
                syncKey={syncKey}
              />
            ))}
          </div>
        </ScrollArea>
      </div>
    </div>
  )
}
