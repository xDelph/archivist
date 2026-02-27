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
          <span className="text-xs tabular-nums">
            {channels.reduce((sum, c) => sum + c.count, 0).toLocaleString()}
          </span>
        </button>
        {channels.map((channel) => (
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
              <span className="text-xs tabular-nums">
                {channel.count.toLocaleString()}
              </span>
            </div>
          </button>
        ))}
      </div>
    </div>
  )
}
