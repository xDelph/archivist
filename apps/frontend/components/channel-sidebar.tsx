import type { ChannelStat } from "@/lib/types"
import { Hash } from "lucide-react"

interface ChannelSidebarProps {
  channels: ChannelStat[]
  selected: string | null
  onSelect: (channel: string | null) => void
}

export function ChannelSidebar({
  channels,
  selected,
  onSelect,
}: ChannelSidebarProps) {
  const topChannels = channels.slice(0, 5)
  const remainingChannels = channels.slice(5)
  const totalThreads = channels.reduce((sum, channel) => sum + channel.count, 0)

  function renderChannelButton(channel: ChannelStat) {
    return (
      <button
        key={channel.name}
        onClick={() => onSelect(channel.name)}
        className={`flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors ${
          selected === channel.name
            ? "bg-primary/10 text-primary"
            : "text-muted-foreground hover:bg-secondary hover:text-foreground"
        }`}
      >
        <Hash className="size-3.5" />
        <span className="flex-1">{channel.name.replace("#", "")}</span>
        <div className="flex items-center gap-2">
          <div className="hidden h-1 w-10 overflow-hidden rounded-full bg-secondary md:block">
            <div
              className={`h-full rounded-full ${channel.color}`}
              style={{ width: `${channel.percentage}%` }}
            />
          </div>
          <span className="text-xs tabular-nums">{channel.count.toLocaleString()}</span>
        </div>
      </button>
    )
  }

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
          <span className="text-xs tabular-nums">{totalThreads.toLocaleString()}</span>
        </button>
        {topChannels.map(renderChannelButton)}

        {remainingChannels.length > 0 && (
          <>
            <div className="mt-1 border-t border-border/60 pt-2">
              <p className="px-2 text-[11px] uppercase tracking-wide text-muted-foreground">
                More channels
              </p>
            </div>
            <div className="max-h-40 overflow-y-auto pr-1 sm:max-h-48">
              <div className="flex flex-col gap-1">{remainingChannels.map(renderChannelButton)}</div>
            </div>
          </>
        )}
      </div>
    </div>
  )
}
