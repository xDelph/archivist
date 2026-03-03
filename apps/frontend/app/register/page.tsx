"use client"

import Link from "next/link"
import { useRouter } from "next/navigation"
import { Eye, EyeOff } from "lucide-react"
import { FormEvent, useEffect, useState } from "react"

const PASSWORD_MIN_LENGTH = 12

export default function RegisterPage() {
  const router = useRouter()
  const [email, setEmail] = useState("")
  const [password, setPassword] = useState("")
  const [showPassword, setShowPassword] = useState(false)
  const [connected, setConnected] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false

    async function checkSession() {
      try {
        const response = await fetch("/api/auth/me", { cache: "no-store" })
        if (!response.ok || cancelled) return
        setConnected(true)
        router.replace("/")
        router.refresh()
      } catch (_err) {}
    }

    void checkSession()
    return () => {
      cancelled = true
    }
  }, [router])

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setError(null)
    if (password.length < PASSWORD_MIN_LENGTH) {
      setError(`Password must be at least ${PASSWORD_MIN_LENGTH} characters.`)
      return
    }
    setLoading(true)
    try {
      const response = await fetch("/api/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password }),
      })
      const payload = (await response.json()) as { error?: string }
      if (!response.ok) {
        setError(payload.error ?? "Unable to create account")
        return
      }
      router.replace("/")
      router.refresh()
    } catch (_err) {
      setError("Unable to create account")
    } finally {
      setLoading(false)
    }
  }

  return (
    <main className="flex min-h-screen items-center justify-center bg-background px-6">
      <div className="w-full max-w-md space-y-6">
        <div className="select-none text-center">
          <h1 className="text-5xl font-semibold tracking-tight text-foreground sm:text-6xl">
            Archivist
          </h1>
          <p className="mt-2 text-base text-muted-foreground sm:text-lg">Slack message archiver</p>
        </div>

        {connected ? (
          <div className="w-full rounded-xl border border-border bg-card p-6">
            <div className="inline-flex items-center gap-2 text-sm text-muted-foreground">
              <span className="size-2.5 animate-pulse rounded-full bg-orange-400" />
              loading
            </div>
          </div>
        ) : (
          <form
            className="w-full space-y-4 rounded-xl border border-border bg-card p-6"
            onSubmit={onSubmit}
            noValidate
          >
            <h2 className="text-2xl font-semibold">Create account</h2>
            <p className="text-sm text-muted-foreground">
              Registration is restricted to Slack workspace users.
            </p>

            <label className="block space-y-1">
              <span className="text-sm">Email</span>
              <input
                type="email"
                required
                value={email}
                onChange={(event) => setEmail(event.target.value)}
                className="w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
              />
            </label>

            <label className="block space-y-1">
              <span className="text-sm">Password</span>
              <div className="relative">
                <input
                  type={showPassword ? "text" : "password"}
                  required
                  value={password}
                  onChange={(event) => setPassword(event.target.value)}
                  className="w-full rounded-md border border-border bg-background px-3 py-2 pr-10 text-sm"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword((prev) => !prev)}
                  className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground"
                  aria-label={showPassword ? "Hide password" : "Show password"}
                >
                  {showPassword ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                </button>
              </div>
              <p className="text-xs text-muted-foreground">
                Requirement: at least {PASSWORD_MIN_LENGTH} characters.
              </p>
            </label>

            {error && <p className="text-sm text-destructive">{error}</p>}

            <button
              type="submit"
              disabled={loading}
              className="w-full rounded-md bg-primary px-3 py-2 text-sm font-medium text-primary-foreground disabled:opacity-60"
            >
              {loading ? "Creating..." : "Create account"}
            </button>

            <div className="text-xs text-muted-foreground">
              Already have an account?{" "}
              <Link href="/login" className="hover:underline">
                Sign in
              </Link>
            </div>
          </form>
        )}
      </div>
    </main>
  )
}
