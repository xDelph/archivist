"use client"

import { useEffect, useRef, useState } from "react"
import {
  Area,
  AreaChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts"
import { TWO_PHASE_DEFAULTS } from "@/hooks/use-two-phase-motion"
import type { ActivityPoint } from "@/lib/types"

interface ActivityChartProps {
  data: ActivityPoint[]
  tab: "top" | "week" | "month" | "recent"
  syncKey?: string
}

type SyncKey = string | number | null | undefined

function easeOutCubic(progress: number): number {
  return 1 - Math.pow(1 - progress, 3)
}

function clampFps(value: number): number {
  if (!Number.isFinite(value)) return TWO_PHASE_DEFAULTS.fps
  return Math.min(60, Math.max(8, Math.round(value)))
}

function normalizeSeries(series: ActivityPoint[]): ActivityPoint[] {
  return series.map((point) => ({
    date: point.date,
    messages: Math.max(0, point.messages),
    threads: Math.max(0, point.threads),
  }))
}

function getSeriesMax(series: ActivityPoint[]): number {
  return series.reduce((max, point) => {
    return Math.max(max, point.messages, point.threads)
  }, 0)
}

function buildFrame(
  fromData: ActivityPoint[],
  toData: ActivityPoint[],
  progress: number
): ActivityPoint[] {
  return toData.map((targetPoint, index) => {
    const sourcePoint = fromData[index] ?? {
      date: targetPoint.date,
      messages: 0,
      threads: 0,
    }
    return {
      date: targetPoint.date,
      messages: Math.max(0, sourcePoint.messages + (targetPoint.messages - sourcePoint.messages) * progress),
      threads: Math.max(0, sourcePoint.threads + (targetPoint.threads - sourcePoint.threads) * progress),
    }
  })
}

function areActivitySeriesEqual(left: ActivityPoint[], right: ActivityPoint[]): boolean {
  if (left.length !== right.length) return false
  return left.every((point, index) => {
    const other = right[index]
    if (!other) return false
    return (
      point.date === other.date &&
      point.messages === other.messages &&
      point.threads === other.threads
    )
  })
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
          <span className="font-medium text-foreground">
            {Math.max(0, Math.round(entry.value)).toLocaleString()}
          </span>
        </p>
      ))}
    </div>
  )
}

function getChartHeading(tab: "top" | "week" | "month" | "recent"): { title: string; subtitle: string } {
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
  if (tab === "recent") {
    return {
      title: "Recent Activity",
      subtitle: "Messages and threads by weekday for the latest 50 threads",
    }
  }
  return {
    title: "Top Threads Activity",
    subtitle: "Messages and threads by weekday for current top threads",
  }
}

export function ActivityChart({ data, tab, syncKey }: ActivityChartProps) {
  const heading = getChartHeading(tab)
  const [animatedData, setAnimatedData] = useState<ActivityPoint[]>(normalizeSeries(data))
  const [yDomainMax, setYDomainMax] = useState<number>(
    Math.max(1, getSeriesMax(normalizeSeries(data)))
  )
  const mountedRef = useRef(false)
  const displayRef = useRef<ActivityPoint[]>(normalizeSeries(data))
  const syncRef = useRef<SyncKey>(syncKey)
  const rafRef = useRef<number | null>(null)
  const holdTimeoutRef = useRef<number | null>(null)

  useEffect(() => {
    const downDurationMs = TWO_PHASE_DEFAULTS.downDurationMs
    const holdMs = TWO_PHASE_DEFAULTS.holdMs
    const upDurationMs = TWO_PHASE_DEFAULTS.upDurationMs
    const fps = clampFps(TWO_PHASE_DEFAULTS.fps)

    const stopAnimations = () => {
      if (rafRef.current != null) {
        window.cancelAnimationFrame(rafRef.current)
        rafRef.current = null
      }
      if (holdTimeoutRef.current != null) {
        window.clearTimeout(holdTimeoutRef.current)
        holdTimeoutRef.current = null
      }
    }

    const setDisplay = (nextData: ActivityPoint[]) => {
      const normalized = normalizeSeries(nextData)
      displayRef.current = normalized
      setAnimatedData((previous) => {
        if (areActivitySeriesEqual(previous, normalized)) return previous
        return normalized
      })
    }

    const animateSeries = (
      fromData: ActivityPoint[],
      toData: ActivityPoint[],
      durationMs: number,
      onDone: () => void
    ) => {
      if (durationMs <= 0) {
        setDisplay(toData)
        onDone()
        return
      }

      const startedAt = performance.now()
      const frameInterval = 1000 / fps
      let lastPaint = startedAt - frameInterval
      const step = (now: number) => {
        const progress = Math.min((now - startedAt) / durationMs, 1)
        if (now - lastPaint >= frameInterval || progress === 1) {
          lastPaint = now
          const eased = easeOutCubic(progress)
          setDisplay(buildFrame(fromData, toData, eased))
        }

        if (progress < 1) {
          rafRef.current = window.requestAnimationFrame(step)
        } else {
          rafRef.current = null
          onDone()
        }
      }

      rafRef.current = window.requestAnimationFrame(step)
    }

    const normalizedTarget = normalizeSeries(data)
    const syncChanged = syncRef.current !== syncKey
    syncRef.current = syncKey

    if (!mountedRef.current) {
      mountedRef.current = true
      setDisplay(normalizedTarget)
      setYDomainMax(Math.max(1, getSeriesMax(normalizedTarget)))
      return
    }

    if (!syncChanged && areActivitySeriesEqual(displayRef.current, normalizedTarget)) return

    stopAnimations()

    const sourceByDate = new Map(displayRef.current.map((point) => [point.date, point]))
    const startData = normalizedTarget.map((point) => ({
      date: point.date,
      messages: sourceByDate.get(point.date)?.messages ?? point.messages,
      threads: sourceByDate.get(point.date)?.threads ?? point.threads,
    }))
    const zeroData = normalizedTarget.map((point) => ({
      date: point.date,
      messages: 0,
      threads: 0,
    }))
    const startDomainMax = Math.max(1, getSeriesMax(startData))
    const targetDomainMax = Math.max(1, getSeriesMax(normalizedTarget))
    setYDomainMax(startDomainMax)

    animateSeries(startData, zeroData, downDurationMs, () => {
      holdTimeoutRef.current = window.setTimeout(() => {
        holdTimeoutRef.current = null
        setYDomainMax(targetDomainMax)
        animateSeries(zeroData, normalizedTarget, upDurationMs, () => {
          setDisplay(normalizedTarget)
        })
      }, holdMs)
    })

    return stopAnimations
  }, [data, syncKey])

  useEffect(() => {
    return () => {
      if (rafRef.current != null) {
        window.cancelAnimationFrame(rafRef.current)
      }
      if (holdTimeoutRef.current != null) {
        window.clearTimeout(holdTimeoutRef.current)
      }
    }
  }, [])

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
        <AreaChart data={animatedData}>
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
            domain={[0, yDomainMax]}
            allowDecimals={false}
          />
          <Tooltip content={<CustomTooltip />} />
          <Area
            type="monotone"
            dataKey="messages"
            stroke="oklch(0.65 0.2 145)"
            strokeWidth={2}
            fill="url(#fillMessages)"
            isAnimationActive={false}
          />
          <Area
            type="monotone"
            dataKey="threads"
            stroke="oklch(0.65 0.15 220)"
            strokeWidth={2}
            fill="url(#fillThreads)"
            isAnimationActive={false}
          />
        </AreaChart>
      </ResponsiveContainer>
    </div>
  )
}
