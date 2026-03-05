import { useEffect, useRef, useState } from "react"
import { useTwoPhaseNumber } from "@/hooks/use-two-phase-motion"
import { MessageSquare, FileText, Users, TrendingUp } from "lucide-react"

interface StatCardProps {
  title: string
  value: number
  change?: number
  icon: "messages" | "threads" | "files" | "users"
  syncKey?: string
}

const iconMap = {
  messages: MessageSquare,
  threads: TrendingUp,
  files: FileText,
  users: Users,
}

const STAT_MOTION = {
  downDurationMs: 350,
  holdMs: 100,
  upDurationMs: 550,
  fps: 30,
} as const

const STAT_TOTAL_DURATION_MS =
  STAT_MOTION.downDurationMs + STAT_MOTION.holdMs + STAT_MOTION.upDurationMs
const STAT_HALF_DURATION_MS = Math.round(STAT_TOTAL_DURATION_MS / 2)

type ChangePolarity = -1 | 0 | 1

function getChangePolarity(value: number): ChangePolarity {
  if (value > 0) return 1
  if (value < 0) return -1
  return 0
}

export function StatCard({ title, value, change, icon, syncKey }: StatCardProps) {
  const Icon = iconMap[icon]
  const targetChange = typeof change === "number" ? change : 0
  const targetPolarity = getChangePolarity(targetChange)
  const [displayPolarity, setDisplayPolarity] = useState<ChangePolarity>(targetPolarity)
  const mountedRef = useRef(false)
  const previousSyncKeyRef = useRef<string | undefined>(syncKey)
  const polarityRef = useRef<ChangePolarity>(targetPolarity)
  const switchTimeoutRef = useRef<number | null>(null)
  const animatedValue = useTwoPhaseNumber(value, STAT_MOTION, syncKey)
  const animatedChangeMagnitude = useTwoPhaseNumber(Math.abs(targetChange), STAT_MOTION, syncKey)
  const displayValue = Math.max(0, Math.round(animatedValue)).toLocaleString()
  const displayChangeValue = Math.max(0, Math.round(animatedChangeMagnitude)).toLocaleString()

  useEffect(() => {
    const syncChanged = previousSyncKeyRef.current !== syncKey
    previousSyncKeyRef.current = syncKey

    if (switchTimeoutRef.current != null) {
      window.clearTimeout(switchTimeoutRef.current)
      switchTimeoutRef.current = null
    }

    if (!mountedRef.current) {
      mountedRef.current = true
      polarityRef.current = targetPolarity
      setDisplayPolarity(targetPolarity)
      return
    }

    if (!syncChanged) {
      if (polarityRef.current !== targetPolarity) {
        polarityRef.current = targetPolarity
        setDisplayPolarity(targetPolarity)
      }
      return
    }

    if (polarityRef.current === targetPolarity) return

    switchTimeoutRef.current = window.setTimeout(() => {
      switchTimeoutRef.current = null
      polarityRef.current = targetPolarity
      setDisplayPolarity(targetPolarity)
    }, STAT_HALF_DURATION_MS)
  }, [syncKey, targetPolarity])

  useEffect(() => {
    return () => {
      if (switchTimeoutRef.current != null) {
        window.clearTimeout(switchTimeoutRef.current)
      }
    }
  }, [])

  return (
    <div className="flex flex-col gap-2 rounded-lg border border-border bg-card p-4">
      <div className="flex items-center justify-between">
        <span className="text-sm text-muted-foreground">{title}</span>
        <div className="flex size-8 items-center justify-center rounded-md bg-primary/10">
          <Icon className="size-4 text-primary" />
        </div>
      </div>
      <div className="flex items-end gap-2">
        <span className="text-2xl font-semibold tracking-tight text-foreground tabular-nums">
          {displayValue}
        </span>
        {typeof change === "number" && (
          <span
            className={`mb-0.5 text-xs font-medium ${
              displayPolarity > 0
                ? "text-success"
                : displayPolarity < 0
                  ? "text-destructive"
                  : "text-muted-foreground"
            }`}
          >
            {displayPolarity > 0 ? "+" : displayPolarity < 0 ? "-" : ""}
            {displayChangeValue}%
          </span>
        )}
      </div>
    </div>
  )
}
