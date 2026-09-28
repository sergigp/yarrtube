import { useState } from 'react'
import { BrowserRouter, Link, Route, Routes } from 'react-router-dom'
import { Menu } from 'lucide-react'
import { Home } from './components/Home'
import { PlaylistDetail } from './components/PlaylistDetail'
import { ChannelDetail } from './components/ChannelDetail'
import { TasksView } from './components/TasksView'
import { AddDialog } from './components/AddDialog'
import { Sidebar } from './components/Sidebar'
import { Button } from '@/components/ui/button'

export default function App() {
  const [addDialogOpen, setAddDialogOpen] = useState(false)
  const [sidebarOpen, setSidebarOpen] = useState(false)

  return (
    <BrowserRouter>
      <div className="flex min-h-dvh flex-col md:h-dvh">
        <header className="sticky top-0 z-20 flex h-(--header-height) shrink-0 items-center gap-4 border-b border-border bg-background px-4 sm:px-6">
          <Button
            variant="ghost"
            size="icon"
            className="md:hidden"
            onClick={() => setSidebarOpen(true)}
            aria-label="Open menu"
          >
            <Menu className="size-5" />
          </Button>
          <Link to="/" className="flex items-center no-underline" aria-label="Yarrtube">
            <img src="/logo.png" alt="Yarrtube" className="h-10 w-auto sm:h-12" />
          </Link>
          <div className="ml-auto flex items-center gap-2">
            <Button asChild variant="ghost">
              <Link to="/tasks">Tasks</Link>
            </Button>
            <Button onClick={() => setAddDialogOpen(true)}>Add</Button>
          </div>
        </header>
        <div className="flex min-h-0 flex-1 flex-col md:flex-row">
          <Sidebar open={sidebarOpen} onClose={() => setSidebarOpen(false)} />
          <main className="min-w-0 flex-1 md:min-h-0 md:overflow-y-auto">
            <div className="h-full px-4 pt-4 pb-6 sm:px-6 md:pt-6">
              <Routes>
                <Route path="/" element={<Home />} />
                <Route path="/playlists/:id" element={<PlaylistDetail />} />
                <Route path="/channels/:id" element={<ChannelDetail />} />
                <Route path="/tasks" element={<TasksView />} />
              </Routes>
            </div>
          </main>
        </div>
        <AddDialog open={addDialogOpen} onOpenChange={setAddDialogOpen} />
      </div>
    </BrowserRouter>
  )
}
