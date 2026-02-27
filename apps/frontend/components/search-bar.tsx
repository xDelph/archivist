"use client"

import { Search, SlidersHorizontal } from "lucide-react"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"

interface SearchBarProps {
  query: string
  onQueryChange: (q: string) => void
  sortBy: string
  onSortChange: (sort: string) => void
  period: string
  onPeriodChange: (period: string) => void
  users: string[]
  selectedUser: string | null
  onUserChange: (user: string | null) => void
}

export function SearchBar({
  query,
  onQueryChange,
  sortBy,
  onSortChange,
  period,
  onPeriodChange,
  users,
  selectedUser,
  onUserChange,
}: SearchBarProps) {
  const normalizedSelectedUser = selectedUser?.trim() ? selectedUser.trim() : null
  const normalizedUsers = Array.from(
    new Set(
      users
        .map((user) => user.trim())
        .filter((user) => user.length > 0)
    )
  )
  const userOptions =
    normalizedSelectedUser && !normalizedUsers.includes(normalizedSelectedUser)
      ? [normalizedSelectedUser, ...normalizedUsers]
      : normalizedUsers

  return (
    <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
      <div className="relative flex-1">
        <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          placeholder="Search messages, users, channels..."
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          onInput={(e) => onQueryChange((e.target as HTMLInputElement).value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault()
            }
          }}
          className="h-9 bg-secondary pl-9 text-sm text-foreground placeholder:text-muted-foreground"
        />
      </div>
      <div className="flex items-center gap-2">
        <SlidersHorizontal className="size-4 text-muted-foreground" />
        <Select value={sortBy} onValueChange={onSortChange}>
          <SelectTrigger className="h-9 w-32 bg-secondary text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="score">Score</SelectItem>
            <SelectItem value="replies">Replies</SelectItem>
            <SelectItem value="reactions">Reactions</SelectItem>
            <SelectItem value="date">Date</SelectItem>
          </SelectContent>
        </Select>
        <Select value={period} onValueChange={onPeriodChange}>
          <SelectTrigger className="h-9 w-32 bg-secondary text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All time</SelectItem>
            <SelectItem value="30d">30 days</SelectItem>
            <SelectItem value="7d">7 days</SelectItem>
          </SelectContent>
        </Select>
        <Select
          value={normalizedSelectedUser ?? "__all__"}
          onValueChange={(value) => onUserChange(value === "__all__" ? null : value)}
        >
          <SelectTrigger className="h-9 w-40 bg-secondary text-sm">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="__all__">All users</SelectItem>
            {userOptions.map((user) => (
              <SelectItem key={user} value={user}>
                {user}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  )
}
