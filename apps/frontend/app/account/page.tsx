"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { FormEvent, useEffect, useState } from "react"

interface AuthMePayload {
  user?: {
    isAnonymous?: boolean
  }
  error?: string
}

export default function AccountPage() {
  const router = useRouter()
  const [isAnonymous, setIsAnonymous] = useState(false)
  const [loadingState, setLoadingState] = useState(true)
  const [savingPref, setSavingPref] = useState(false)
  const [prefError, setPrefError] = useState<string | null>(null)
  const [prefDone, setPrefDone] = useState(false)
  const [currentPassword, setCurrentPassword] = useState("")
  const [newPassword, setNewPassword] = useState("")
  const [savingPassword, setSavingPassword] = useState(false)
  const [passwordError, setPasswordError] = useState<string | null>(null)
  const [passwordDone, setPasswordDone] = useState(false)

  useEffect(() => {
    let cancelled = false
    async function loadMe() {
      try {
        const response = await fetch("/api/auth/me", { cache: "no-store" })
        if (!response.ok) {
          if (!cancelled) {
            router.replace("/login")
          }
          return
        }
        const payload = (await response.json()) as AuthMePayload
        if (!cancelled) {
          setIsAnonymous(Boolean(payload.user?.isAnonymous))
        }
      } finally {
        if (!cancelled) {
          setLoadingState(false)
        }
      }
    }
    void loadMe()
    return () => {
      cancelled = true
    }
  }, [router])

  async function submitPreferences(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setSavingPref(true)
    setPrefError(null)
    setPrefDone(false)
    try {
      const response = await fetch("/api/auth/preferences", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ isAnonymous }),
      })
      const payload = (await response.json()) as { error?: string }
      if (!response.ok) {
        setPrefError(payload.error ?? "Unable to update preference")
        return
      }
      setPrefDone(true)
    } catch (_err) {
      setPrefError("Unable to update preference")
    } finally {
      setSavingPref(false)
    }
  }

  async function submitPassword(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setSavingPassword(true)
    setPasswordError(null)
    setPasswordDone(false)
    try {
      const response = await fetch("/api/auth/change-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ currentPassword, newPassword }),
      })
      const payload = (await response.json()) as { error?: string }
      if (!response.ok) {
        setPasswordError(payload.error ?? "Unable to update password")
        return
      }
      setPasswordDone(true)
      setCurrentPassword("")
      setNewPassword("")
    } catch (_err) {
      setPasswordError("Unable to update password")
    } finally {
      setSavingPassword(false)
    }
  }

  async function logout() {
    await fetch("/api/auth/logout", { method: "POST" })
    router.replace("/login")
    router.refresh()
  }

  if (loadingState) {
    return (
      <main className="mx-auto flex min-h-screen w-full max-w-2xl items-center px-6">
        <p className="text-sm text-muted-foreground">Loading account...</p>
      </main>
    )
  }

  return (
    <main className="mx-auto min-h-screen w-full max-w-2xl px-6 py-10">
      <div className="mb-6 flex items-center justify-between">
        <h1 className="text-2xl font-semibold">Account</h1>
        <div className="flex items-center gap-3">
          <Link href="/" className="text-sm text-muted-foreground hover:underline">
            Back to dashboard
          </Link>
          <button
            type="button"
            onClick={logout}
            className="rounded-md border border-border px-3 py-1.5 text-sm"
          >
            Logout
          </button>
        </div>
      </div>

      <section className="mb-6 rounded-xl border border-border bg-card p-6">
        <h2 className="mb-2 text-lg font-semibold">Privacy</h2>
        <form className="space-y-4" onSubmit={submitPreferences}>
          <label className="flex items-start gap-3 text-sm">
            <input
              type="checkbox"
              checked={isAnonymous}
              onChange={(event) => setIsAnonymous(event.target.checked)}
              className="mt-0.5 h-4 w-4 rounded border-border"
            />
            <span>
              Appear anonymous in Archivist.
              <br />
              Your messages will be rendered as Anonymous with default avatar.
            </span>
          </label>
          {prefError && <p className="text-sm text-destructive">{prefError}</p>}
          {prefDone && <p className="text-sm text-emerald-500">Preference updated.</p>}
          <button
            type="submit"
            disabled={savingPref}
            className="rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground disabled:opacity-60"
          >
            {savingPref ? "Saving..." : "Save preference"}
          </button>
        </form>
      </section>

      <section className="rounded-xl border border-border bg-card p-6">
        <h2 className="mb-2 text-lg font-semibold">Password</h2>
        <form className="space-y-4" onSubmit={submitPassword}>
          <label className="block space-y-1">
            <span className="text-sm">Current password</span>
            <input
              type="password"
              required
              value={currentPassword}
              onChange={(event) => setCurrentPassword(event.target.value)}
              className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
            />
          </label>

          <label className="block space-y-1">
            <span className="text-sm">New password</span>
            <input
              type="password"
              minLength={12}
              required
              value={newPassword}
              onChange={(event) => setNewPassword(event.target.value)}
              className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
            />
          </label>

          {passwordError && <p className="text-sm text-destructive">{passwordError}</p>}
          {passwordDone && <p className="text-sm text-emerald-500">Password updated.</p>}
          <button
            type="submit"
            disabled={savingPassword}
            className="rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground disabled:opacity-60"
          >
            {savingPassword ? "Updating..." : "Update password"}
          </button>
        </form>
      </section>
    </main>
  )
}
