"use client"

import Link from "next/link"
import { FormEvent, useState } from "react"

export default function ForgotPasswordPage() {
  const [email, setEmail] = useState("")
  const [loading, setLoading] = useState(false)
  const [done, setDone] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [devToken, setDevToken] = useState<string | null>(null)

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setLoading(true)
    setError(null)
    setDevToken(null)
    try {
      const response = await fetch("/api/auth/password/forgot", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email }),
      })
      const payload = (await response.json()) as { error?: string; resetToken?: string }
      if (!response.ok) {
        setError(payload.error ?? "Unable to request password reset")
        return
      }
      setDone(true)
      if (payload.resetToken) {
        setDevToken(payload.resetToken)
      }
    } catch (_err) {
      setError("Unable to request password reset")
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
        <h1 className="text-2xl font-semibold">Forgot password</h1>
        <p className="text-sm text-muted-foreground">
          We will send reset instructions if the account exists.
        </p>

        <label className="block space-y-1">
          <span className="text-sm">Email</span>
          <input
            type="email"
            required
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            className="w-full rounded-md border border-border bg-background px-3 py-2 text-base md:text-sm"
          />
        </label>

        {error && <p className="text-sm text-destructive">{error}</p>}
        {done && (
          <p className="text-sm text-emerald-500">
            Reset request submitted. Check your email.
          </p>
        )}
        {devToken && (
          <p className="break-all rounded-md bg-secondary p-2 text-xs text-muted-foreground">
            Dev reset token: {devToken}
          </p>
        )}

        <button
          type="submit"
          disabled={loading}
          className="w-full rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground disabled:opacity-60"
        >
          {loading ? "Submitting..." : "Request reset"}
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
