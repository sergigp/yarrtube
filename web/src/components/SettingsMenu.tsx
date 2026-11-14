import { Link } from 'react-router-dom'
import { Activity, Bug, ExternalLink, Settings, SlidersHorizontal } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Button } from '@/components/ui/button'
import { useTasks } from '@/api/queries'

const NEW_ISSUE_URL = 'https://github.com/sergigp/yarrtube/issues/new'

const ITEM_CLASS = 'h-9 gap-2.5 px-2.5 font-medium'
const ICON_CLASS = 'text-muted-foreground'

export function SettingsMenu() {
  const { data: tasks } = useTasks()
  const running = tasks?.filter((task) => task.status === 'running').length ?? 0

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Settings"
          className="relative data-[state=open]:bg-secondary"
        >
          <Settings className="size-5" />
          {running > 0 && (
            <span
              aria-hidden="true"
              className="absolute top-1.5 right-1.5 size-2 rounded-full bg-primary ring-2 ring-background"
            />
          )}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="end"
        sideOffset={6}
        className="w-60 rounded-xl p-1.5 shadow-lg"
      >
        <DropdownMenuItem asChild className={ITEM_CLASS}>
          <Link to="/tasks">
            <Activity className={ICON_CLASS} />
            <span className="flex-1">Tasks</span>
            {running > 0 && (
              <span className="rounded-full bg-primary px-2 py-0.5 text-xs font-semibold text-primary-foreground">
                {running} running
              </span>
            )}
          </Link>
        </DropdownMenuItem>
        <DropdownMenuItem asChild className={ITEM_CLASS}>
          <Link to="/settings/general">
            <SlidersHorizontal className={ICON_CLASS} />
            <span className="flex-1">Settings</span>
          </Link>
        </DropdownMenuItem>
        <DropdownMenuSeparator className="-mx-1.5" />
        <DropdownMenuItem asChild className={`${ITEM_CLASS} text-muted-foreground`}>
          <a href={NEW_ISSUE_URL} target="_blank" rel="noreferrer">
            <Bug className={ICON_CLASS} />
            <span className="flex-1">Report a bug</span>
            <ExternalLink className={`${ICON_CLASS} size-3.5`} aria-hidden="true" />
          </a>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
