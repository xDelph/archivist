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
}

export function SearchBar({
  query,
  onQueryChange,
  sortBy,
  onSortChange,
}: SearchBarProps) {
  return (
    <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
      <div className="relative flex-1">
        <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          placeholder="Search messages, users, channels..."
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
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
      </div>
    </div>
  )
}
