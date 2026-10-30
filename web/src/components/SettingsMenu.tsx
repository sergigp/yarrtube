import { Link } from 'react-router-dom'
import { Settings } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Button } from '@/components/ui/button'

const NEW_ISSUE_URL = 'https://github.com/sergigp/yarrtube/issues/new'

export function SettingsMenu() {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label="Settings">
          <Settings className="size-5" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-auto min-w-40">
        <DropdownMenuItem asChild>
          <Link to="/tasks">Tasks</Link>
        </DropdownMenuItem>
        <DropdownMenuItem asChild>
          <Link to="/preferences">Preferences</Link>
        </DropdownMenuItem>
        <DropdownMenuItem asChild>
          <a href={NEW_ISSUE_URL} target="_blank" rel="noreferrer">
            Report a bug
          </a>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
