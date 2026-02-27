"use client"

import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts"
import type { ActivityPoint } from "@/lib/types"

interface ActivityChartProps {
  data: ActivityPoint[]
  tab: "top" | "week" | "month"
}

function CustomTooltip({
  active,
  payload,
  label,
}: {
  active?: boolean
  payload?: Array<{ value: number; dataKey: string; color: string }>
  label?: string
}) {
  if (!active || !payload) return null
  return (
    <div className="rounded-md border border-border bg-popover px-3 py-2 shadow-lg">
      <p className="mb-1 text-xs font-medium text-foreground">{label}</p>
      {payload.map((entry) => (
        <p
          key={entry.dataKey}
          className="text-xs text-muted-foreground"
        >
          <span
            className="mr-1.5 inline-block size-2 rounded-full"
            style={{ backgroundColor: entry.color }}
          />
          {entry.dataKey === "messages" ? "Messages" : "Threads"}:{" "}
          <span className="font-medium text-foreground">{entry.value}</span>
        </p>
      ))}
    </div>
  )
}

function getChartHeading(tab: "top" | "week" | "month"): { title: string; subtitle: string } {
  if (tab === "week") {
    return {
      title: "Weekly Activity",
      subtitle: "Messages and threads by weekday for this week's ranking",
    }
  }
  if (tab === "month") {
    return {
      title: "Monthly Activity",
      subtitle: "Messages and threads by weekday for this month's ranking",
    }
  }
  return {
    title: "Top Threads Activity",
    subtitle: "Messages and threads by weekday for current top threads",
  }
}

export function ActivityChart({ data, tab }: ActivityChartProps) {
  const heading = getChartHeading(tab)

  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium text-foreground">{heading.title}</h3>
          <p className="text-xs text-muted-foreground">
            {heading.subtitle}
          </p>
        </div>
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-1.5">
            <span className="size-2 rounded-full bg-chart-1" />
            <span className="text-xs text-muted-foreground">Messages</span>
          </div>
          <div className="flex items-center gap-1.5">
            <span className="size-2 rounded-full bg-chart-2" />
            <span className="text-xs text-muted-foreground">Threads</span>
          </div>
        </div>
      </div>
      <ResponsiveContainer width="100%" height={180}>
        <AreaChart data={data}>
          <defs>
            <linearGradient id="fillMessages" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="oklch(0.65 0.2 145)" stopOpacity={0.3} />
              <stop offset="100%" stopColor="oklch(0.65 0.2 145)" stopOpacity={0} />
            </linearGradient>
            <linearGradient id="fillThreads" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="oklch(0.65 0.15 220)" stopOpacity={0.3} />
              <stop offset="100%" stopColor="oklch(0.65 0.15 220)" stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid
            strokeDasharray="3 3"
            stroke="oklch(0.25 0.005 260)"
            vertical={false}
          />
          <XAxis
            dataKey="date"
            tick={{ fill: "oklch(0.6 0.01 260)", fontSize: 12 }}
            axisLine={false}
            tickLine={false}
          />
          <YAxis
            tick={{ fill: "oklch(0.6 0.01 260)", fontSize: 12 }}
            axisLine={false}
            tickLine={false}
            width={36}
          />
          <Tooltip content={<CustomTooltip />} />
          <Area
            type="monotone"
            dataKey="messages"
            stroke="oklch(0.65 0.2 145)"
            strokeWidth={2}
            fill="url(#fillMessages)"
          />
          <Area
            type="monotone"
            dataKey="threads"
            stroke="oklch(0.65 0.15 220)"
            strokeWidth={2}
            fill="url(#fillThreads)"
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  )
}
