"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { FormEvent, useEffect, useState } from "react"

export default function ResetPasswordPage() {
  const router = useRouter()
  const [token, setToken] = useState("")
  const [newPassword, setNewPassword] = useState("")
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [done, setDone] = useState(false)

  useEffect(() => {
    const tokenFromUrl = new URLSearchParams(window.location.search).get("token") ?? ""
    setToken(tokenFromUrl)
  }, [])

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setLoading(true)
    setError(null)
    setDone(false)
    try {
      const response = await fetch("/api/auth/password/reset", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ token, newPassword }),
      })
      const payload = (await response.json()) as { error?: string }
      if (!response.ok) {
        setError(payload.error ?? "Unable to reset password")
        return
      }
      setDone(true)
      window.setTimeout(() => {
        router.replace("/login")
      }, 1200)
    } catch (_err) {
      setError("Unable to reset password")
    } finally {
      setLoading(false)
    }
  }

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-md items-center px-6">
      <form
        className="w-full space-y-4 rounded-xl border border-border bg-card p-6"
        onSubmit={onSubmit}
      >
        <h1 className="text-2xl font-semibold">Reset password</h1>
        <p className="text-sm text-muted-foreground">
          Enter your reset token and a new password.
        </p>

        <label className="block space-y-1">
          <span className="text-sm">Reset token</span>
          <input
            type="text"
            required
            value={token}
            onChange={(event) => setToken(event.target.value)}
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

        {error && <p className="text-sm text-destructive">{error}</p>}
        {done && <p className="text-sm text-emerald-500">Password reset. Redirecting...</p>}

        <button
          type="submit"
          disabled={loading}
          className="w-full rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground disabled:opacity-60"
        >
          {loading ? "Updating..." : "Reset password"}
        </button>

        <div className="text-xs text-muted-foreground">
          <Link href="/login" className="hover:underline">
            Back to sign in
          </Link>
        </div>
      </form>
    </main>
  )
}
