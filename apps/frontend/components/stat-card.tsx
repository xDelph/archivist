import { MessageSquare, FileText, Users, TrendingUp } from "lucide-react"

interface StatCardProps {
  title: string
  value: string
  change: number
  icon: "messages" | "threads" | "files" | "users"
}

const iconMap = {
  messages: MessageSquare,
  threads: TrendingUp,
  files: FileText,
  users: Users,
}

export function StatCard({ title, value, change, icon }: StatCardProps) {
  const Icon = iconMap[icon]
  const isPositive = change > 0

  return (
    <div className="flex flex-col gap-2 rounded-lg border border-border bg-card p-4">
      <div className="flex items-center justify-between">
        <span className="text-sm text-muted-foreground">{title}</span>
        <div className="flex size-8 items-center justify-center rounded-md bg-primary/10">
          <Icon className="size-4 text-primary" />
        </div>
      </div>
      <div className="flex items-end gap-2">
        <span className="text-2xl font-semibold tracking-tight text-foreground">
          {value}
        </span>
        <span
          className={`mb-0.5 text-xs font-medium ${
            isPositive ? "text-success" : "text-destructive"
          }`}
        >
          {isPositive ? "+" : ""}
          {change}%
        </span>
      </div>
    </div>
  )
}
