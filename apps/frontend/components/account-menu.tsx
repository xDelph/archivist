"use client"

import { FormEvent, useEffect, useMemo, useState } from "react"
import { Eye, EyeOff, LogOut, Settings2 } from "lucide-react"

import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

interface AccountMenuProps {
  onLogout: () => Promise<void>
}

interface AuthUser {
  isAnonymous?: boolean
  displayName?: string
  avatarUrl?: string
}

interface AuthMeResponse {
  user?: AuthUser
}

const PASSWORD_MIN_LENGTH = 12

export function AccountMenu({ onLogout }: AccountMenuProps) {
  const [menuOpen, setMenuOpen] = useState(false)
  const [loadingUser, setLoadingUser] = useState(true)
  const [user, setUser] = useState<AuthUser | null>(null)
  const [prefError, setPrefError] = useState<string | null>(null)
  const [passwordDialogOpen, setPasswordDialogOpen] = useState(false)
  const [currentPassword, setCurrentPassword] = useState("")
  const [newPassword, setNewPassword] = useState("")
  const [showCurrentPassword, setShowCurrentPassword] = useState(false)
  const [showNewPassword, setShowNewPassword] = useState(false)
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
            setUser(null)
          }
          return
        }
        const payload = (await response.json()) as AuthMeResponse
        if (!cancelled) {
          setUser(payload.user ?? null)
        }
      } finally {
        if (!cancelled) {
          setLoadingUser(false)
        }
      }
    }
    void loadMe()
    return () => {
      cancelled = true
    }
  }, [])

  const isAnonymous = Boolean(user?.isAnonymous)
  const displayName = user?.displayName?.trim() || "User"
  const avatarUrl = user?.avatarUrl?.trim() || "/placeholder-user.jpg"
  const initials = useMemo(() => {
    const chars = displayName
      .split(" ")
      .filter(Boolean)
      .map((chunk) => chunk[0]?.toUpperCase())
      .filter(Boolean)
      .slice(0, 2)
    return chars.join("") || "U"
  }, [displayName])

  async function updateAnonymousPreference(nextValue: boolean): Promise<void> {
    if (!user) return
    setPrefError(null)
    const previous = user
    setUser({ ...previous, isAnonymous: nextValue })
    try {
      const response = await fetch("/api/auth/preferences", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ isAnonymous: nextValue }),
      })
      const payload = (await response.json()) as AuthMeResponse & { error?: string }
      if (!response.ok) {
        setUser(previous)
        setPrefError(payload.error ?? "Unable to update preference")
        return
      }
      setUser(payload.user ?? { ...previous, isAnonymous: nextValue })
    } catch (_err) {
      setUser(previous)
      setPrefError("Unable to update preference")
    }
  }

  async function submitPassword(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault()
    setSavingPassword(true)
    setPasswordError(null)
    setPasswordDone(false)

    if (newPassword.length < PASSWORD_MIN_LENGTH) {
      setPasswordError(`Password must be at least ${PASSWORD_MIN_LENGTH} characters.`)
      setSavingPassword(false)
      return
    }

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
      setShowCurrentPassword(false)
      setShowNewPassword(false)
    } catch (_err) {
      setPasswordError("Unable to update password")
    } finally {
      setSavingPassword(false)
    }
  }

  return (
    <>
      <DropdownMenu open={menuOpen} onOpenChange={setMenuOpen}>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            className="inline-flex h-8 w-8 items-center justify-center rounded-full border border-border bg-secondary"
            aria-label={loadingUser ? "Open account menu" : `${displayName} account menu`}
          >
            <Avatar className="h-8 w-8">
              <AvatarImage src={avatarUrl} alt={displayName} />
              <AvatarFallback className="text-[11px] font-semibold">{initials}</AvatarFallback>
            </Avatar>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent
          align="end"
          sideOffset={10}
          className="w-64 rounded-xl border-border/90 bg-popover/98 shadow-[0_22px_60px_rgba(0,0,0,0.62)] ring-1 ring-black/30 backdrop-blur-md"
        >
          <DropdownMenuLabel className="space-y-0.5">
            <p className="text-sm font-medium">{loadingUser ? "Loading..." : displayName}</p>
            <p className="text-xs font-normal text-muted-foreground">
              {isAnonymous ? "Anonymous mode enabled" : "Standard profile"}
            </p>
          </DropdownMenuLabel>
          <DropdownMenuSeparator className="bg-border/80" />
          <DropdownMenuCheckboxItem
            checked={isAnonymous}
            disabled={loadingUser || !user}
            onCheckedChange={(checked) => {
              void updateAnonymousPreference(checked === true)
            }}
          >
            Appear anonymous
          </DropdownMenuCheckboxItem>
          <DropdownMenuItem
            onSelect={(event) => {
              event.preventDefault()
              setPasswordDialogOpen(true)
              setPasswordDone(false)
              setPasswordError(null)
            }}
          >
            <Settings2 className="size-4" />
            Change password
          </DropdownMenuItem>
          {prefError && <p className="px-2 pb-1 text-xs text-destructive">{prefError}</p>}
          <DropdownMenuSeparator className="bg-border/80" />
          <DropdownMenuItem
            variant="destructive"
            onSelect={(event) => {
              event.preventDefault()
              void onLogout()
            }}
          >
            <LogOut className="size-4" />
            Logout
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <Dialog open={passwordDialogOpen} onOpenChange={setPasswordDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Change password</DialogTitle>
            <DialogDescription>
              Use a strong password with at least {PASSWORD_MIN_LENGTH} characters.
            </DialogDescription>
          </DialogHeader>
          <form className="space-y-4" onSubmit={submitPassword} noValidate>
            <label className="block space-y-1">
              <span className="text-sm">Current password</span>
              <div className="relative">
                <input
                  type={showCurrentPassword ? "text" : "password"}
                  required
                  value={currentPassword}
                  onChange={(event) => setCurrentPassword(event.target.value)}
                  className="w-full rounded-md border border-border bg-background px-3 py-2 pr-10 text-sm"
                />
                <button
                  type="button"
                  onClick={() => setShowCurrentPassword((prev) => !prev)}
                  className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground"
                  aria-label={showCurrentPassword ? "Hide password" : "Show password"}
                >
                  {showCurrentPassword ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                </button>
              </div>
            </label>

            <label className="block space-y-1">
              <span className="text-sm">New password</span>
              <div className="relative">
                <input
                  type={showNewPassword ? "text" : "password"}
                  required
                  value={newPassword}
                  onChange={(event) => setNewPassword(event.target.value)}
                  className="w-full rounded-md border border-border bg-background px-3 py-2 pr-10 text-sm"
                />
                <button
                  type="button"
                  onClick={() => setShowNewPassword((prev) => !prev)}
                  className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground"
                  aria-label={showNewPassword ? "Hide password" : "Show password"}
                >
                  {showNewPassword ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
                </button>
              </div>
              <p className="text-xs text-muted-foreground">
                Requirement: at least {PASSWORD_MIN_LENGTH} characters.
              </p>
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
        </DialogContent>
      </Dialog>
    </>
  )
}
