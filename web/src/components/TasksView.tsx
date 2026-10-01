import { useState } from 'react'
import { Download, RotateCw, Trash2, Wrench, type LucideIcon } from 'lucide-react'
import { useTasks } from '@/api/queries'
import { formatRelativeTime } from '@/lib/formatDateTime'
import {
  describeTask,
  matchesTask,
  tabCounts,
  tasksForTab,
  taskCategory,
  taskFamily,
  TASK_SEARCH_THRESHOLD,
  TASK_TABS,
  type TaskFamily,
  type TaskTab,
} from '@/lib/tasks'
import { Beacon } from './Beacon'
import { Badge } from '@/components/ui/badge'
import { Input } from '@/components/ui/input'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { cn } from '@/lib/utils'

const CATEGORY_BADGE_VARIANT: Record<string, 'default' | 'outline' | 'secondary'> = {
  running: 'default',
  queued: 'outline',
  pending: 'secondary',
}

const TAB_LABELS: Record<TaskTab, string> = {
  active: 'Active',
  downloads: 'Downloads',
  syncs: 'Syncs',
  cleanup: 'Cleanup',
  all: 'All',
}

const FAMILY_ICON: Record<TaskFamily, LucideIcon> = {
  downloads: Download,
  syncs: RotateCw,
  cleanup: Trash2,
  maintenance: Wrench,
}

export function TasksView() {
  const { data: tasks, error } = useTasks()
  const [tab, setTab] = useState<TaskTab>('active')
  const [search, setSearch] = useState('')

  if (error) {
    return <p className="text-sm text-destructive">Failed to load tasks: {error.message}</p>
  }

  if (!tasks) {
    return <p className="text-sm text-muted-foreground">Loading tasks…</p>
  }

  if (tasks.length === 0) {
    return <p className="text-sm text-muted-foreground">No pending or in-progress tasks.</p>
  }

  const counts = tabCounts(tasks)
  const tabTasks = tasksForTab(tasks, tab)
  const showSearch = tabTasks.length > TASK_SEARCH_THRESHOLD
  const query = showSearch ? search.trim() : ''
  const rows = query ? tabTasks.filter((task) => matchesTask(task, query)) : tabTasks

  return (
    <TooltipProvider>
      <Tabs
        value={tab}
        onValueChange={(value) => {
          setTab(value as TaskTab)
          setSearch('')
        }}
      >
        <TabsList className="w-full">
          {TASK_TABS.map((value) => (
            <TabsTrigger key={value} value={value}>
              {TAB_LABELS[value]}
              <Badge variant="secondary">{counts[value]}</Badge>
            </TabsTrigger>
          ))}
        </TabsList>
        <TabsContent value={tab} className="flex flex-col gap-2">
          {showSearch && (
            <Input
              type="search"
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search"
              aria-label="Search tasks"
            />
          )}
          {rows.length === 0 ? (
            <p className="px-2 text-sm text-muted-foreground">
              {query
                ? 'Nothing matches'
                : tab === 'active'
                  ? 'Nothing running right now'
                  : 'No tasks here'}
            </p>
          ) : (
            <ul className="flex flex-col divide-y divide-border">
              {rows.map((task, index) => {
                const category = taskCategory(task)
                const Icon = FAMILY_ICON[taskFamily(task)]
                return (
                  <li
                    key={task.id}
                    className={cn(
                      'flex items-center gap-3 px-2 py-3 animate-enter',
                      category === 'running' && 'bg-accent',
                    )}
                    style={{ animationDelay: `${Math.min(index, 12) * 20}ms` }}
                  >
                    {category === 'running' && <Beacon variant="live" label="Running" />}
                    <Icon className="size-4 shrink-0 text-muted-foreground" aria-hidden />
                    <span className="flex min-w-0 flex-1 flex-col">
                      <span className="truncate text-sm text-foreground">{describeTask(task)}</span>
                      {task.last_error && (
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <span className="truncate text-xs text-muted-foreground">
                              {task.last_error}
                            </span>
                          </TooltipTrigger>
                          <TooltipContent>{task.last_error}</TooltipContent>
                        </Tooltip>
                      )}
                    </span>
                    <span className="flex shrink-0 items-center gap-1.5">
                      <Badge variant={CATEGORY_BADGE_VARIANT[category] ?? 'secondary'}>
                        {category}
                      </Badge>
                      {task.retries > 0 && <Badge variant="outline">{task.retries} retries</Badge>}
                      <span className="w-16 text-right text-xs text-muted-foreground">
                        {category === 'pending' ? formatRelativeTime(task.run_at) : ''}
                      </span>
                    </span>
                  </li>
                )
              })}
            </ul>
          )}
        </TabsContent>
      </Tabs>
    </TooltipProvider>
  )
}
