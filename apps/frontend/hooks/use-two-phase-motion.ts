"use client"

import { useEffect, useRef, useState } from "react"

interface TwoPhaseNumberOptions {
  downDurationMs?: number
  upDurationMs?: number
  holdMs?: number
  fps?: number
}

interface MorphingTextOptions {
  downDurationMs?: number
  upDurationMs?: number
  holdMs?: number
  fps?: number
}

type SyncKey = string | number | null | undefined

export const TWO_PHASE_DEFAULTS = {
  downDurationMs: 350,
  holdMs: 100,
  upDurationMs: 550,
  fps: 30,
} as const

function easeInOutCubic(progress: number): number {
  if (progress < 0.5) {
    return 4 * progress * progress * progress
  }
  return 1 - Math.pow(-2 * progress + 2, 3) / 2
}

function clampFps(value: number): number {
  if (!Number.isFinite(value)) return TWO_PHASE_DEFAULTS.fps
  return Math.min(60, Math.max(8, Math.round(value)))
}

export function useTwoPhaseNumber(
  targetValue: number,
  {
    downDurationMs = TWO_PHASE_DEFAULTS.downDurationMs,
    upDurationMs = TWO_PHASE_DEFAULTS.upDurationMs,
    holdMs = TWO_PHASE_DEFAULTS.holdMs,
    fps = TWO_PHASE_DEFAULTS.fps,
  }: TwoPhaseNumberOptions = {},
  syncKey?: SyncKey
): number {
  const [displayValue, setDisplayValue] = useState(Math.max(0, Math.round(targetValue)))
  const mountedRef = useRef(false)
  const displayRef = useRef(targetValue)
  const roundedRef = useRef(Math.max(0, Math.round(targetValue)))
  const syncRef = useRef<SyncKey>(syncKey)
  const rafRef = useRef<number | null>(null)
  const holdTimeoutRef = useRef<number | null>(null)

  function stopAnimations(): void {
    if (rafRef.current != null) {
      window.cancelAnimationFrame(rafRef.current)
      rafRef.current = null
    }
    if (holdTimeoutRef.current != null) {
      window.clearTimeout(holdTimeoutRef.current)
      holdTimeoutRef.current = null
    }
  }

  function setDisplay(nextValue: number, force = false): void {
    displayRef.current = nextValue
    const roundedValue = Math.max(0, Math.round(nextValue))
    if (!force && roundedRef.current === roundedValue) return
    roundedRef.current = roundedValue
    setDisplayValue(roundedValue)
  }

  function animate(from: number, to: number, durationMs: number, onDone: () => void): void {
    if (durationMs <= 0) {
      setDisplay(to)
      onDone()
      return
    }

    const startedAt = performance.now()
    const frameInterval = 1000 / clampFps(fps)
    let lastPaint = startedAt - frameInterval

    const step = (now: number) => {
      const progress = Math.min((now - startedAt) / durationMs, 1)
      if (now - lastPaint >= frameInterval || progress === 1) {
        lastPaint = now
        const eased = easeInOutCubic(progress)
        const interpolated = from + (to - from) * eased
        const shouldSnapToTarget = Math.abs(to - from) >= 1 && progress >= 0.985
        setDisplay(shouldSnapToTarget ? to : interpolated, shouldSnapToTarget)
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

  useEffect(() => {
    const syncChanged = syncRef.current !== syncKey
    syncRef.current = syncKey

    if (!mountedRef.current) {
      mountedRef.current = true
      setDisplay(targetValue, true)
      return
    }

    if (!syncChanged && displayRef.current === targetValue) return

    stopAnimations()

    const startValue = displayRef.current
    animate(startValue, 0, downDurationMs, () => {
      holdTimeoutRef.current = window.setTimeout(() => {
        holdTimeoutRef.current = null
        animate(0, targetValue, upDurationMs, () => {
          setDisplay(targetValue, true)
        })
      }, holdMs)
    })

    return stopAnimations
  }, [targetValue, downDurationMs, upDurationMs, holdMs, fps, syncKey])

  useEffect(() => () => stopAnimations(), [])

  return displayValue
}

export function useMorphingText(
  targetText: string,
  {
    downDurationMs = TWO_PHASE_DEFAULTS.downDurationMs,
    upDurationMs = TWO_PHASE_DEFAULTS.upDurationMs,
    holdMs = TWO_PHASE_DEFAULTS.holdMs,
    fps = TWO_PHASE_DEFAULTS.fps,
  }: MorphingTextOptions = {},
  syncKey?: SyncKey
): string {
  const [displayText, setDisplayText] = useState(targetText)
  const mountedRef = useRef(false)
  const textRef = useRef(targetText)
  const syncRef = useRef<SyncKey>(syncKey)
  const rafRef = useRef<number | null>(null)
  const holdTimeoutRef = useRef<number | null>(null)

  function stopAnimations(): void {
    if (rafRef.current != null) {
      window.cancelAnimationFrame(rafRef.current)
      rafRef.current = null
    }
    if (holdTimeoutRef.current != null) {
      window.clearTimeout(holdTimeoutRef.current)
      holdTimeoutRef.current = null
    }
  }

  function setText(nextText: string, force = false): void {
    if (!force && textRef.current === nextText) return
    textRef.current = nextText
    setDisplayText(nextText)
  }

  function animateText(
    fromText: string,
    toText: string,
    durationMs: number,
    phase: "down" | "up",
    onDone: () => void
  ): void {
    if (durationMs <= 0) {
      setText(phase === "down" ? "" : toText, true)
      onDone()
      return
    }

    const startedAt = performance.now()
    const frameInterval = 1000 / clampFps(fps)
    let lastPaint = startedAt - frameInterval

    const step = (now: number) => {
      const progress = Math.min((now - startedAt) / durationMs, 1)
      if (now - lastPaint >= frameInterval || progress === 1) {
        lastPaint = now
        const eased = easeInOutCubic(progress)
        if (phase === "down") {
          const nextLength = Math.max(0, Math.round(fromText.length * (1 - eased)))
          setText(fromText.slice(0, nextLength))
        } else {
          const nextLength = Math.min(toText.length, Math.round(toText.length * eased))
          setText(toText.slice(0, nextLength))
        }
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

  useEffect(() => {
    const syncChanged = syncRef.current !== syncKey
    syncRef.current = syncKey

    if (!mountedRef.current) {
      mountedRef.current = true
      setText(targetText, true)
      return
    }

    if (!syncChanged && textRef.current === targetText) return

    stopAnimations()

    const previousText = textRef.current
    animateText(previousText, targetText, downDurationMs, "down", () => {
      holdTimeoutRef.current = window.setTimeout(() => {
        holdTimeoutRef.current = null
        animateText(previousText, targetText, upDurationMs, "up", () => {
          setText(targetText, true)
        })
      }, holdMs)
    })

    return stopAnimations
  }, [targetText, downDurationMs, upDurationMs, holdMs, fps, syncKey])

  useEffect(() => () => stopAnimations(), [])

  return displayText
}
