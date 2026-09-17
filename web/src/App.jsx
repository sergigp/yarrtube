import { useState } from 'react'
import { BrowserRouter, Link, Route, Routes } from 'react-router-dom'
import { Home } from './components/Home'
import { PlaylistDetail } from './components/PlaylistDetail'
import { ChannelDetail } from './components/ChannelDetail'
import { TasksView } from './components/TasksView'
import { AddDialog } from './components/AddDialog'
import { Button } from '@/components/ui/button'

export default function App() {
  const [addDialogOpen, setAddDialogOpen] = useState(false)

  return (
    <BrowserRouter>
      <div className="flex h-screen flex-col">
        <header className="flex shrink-0 items-center gap-4 border-b border-border px-4 py-3 sm:px-6">
          <Link
            to="/"
            className="font-heading text-xl font-semibold tracking-tight text-foreground no-underline"
          >
            Yarrtube
          </Link>
          <div className="ml-auto flex items-center gap-2">
            <Button asChild variant="ghost">
              <Link to="/tasks">Tasks</Link>
            </Button>
            <Button onClick={() => setAddDialogOpen(true)}>Add</Button>
          </div>
        </header>
        <main className="min-h-0 flex-1 overflow-y-auto">
          <div className="mx-auto max-w-[1400px] px-4 py-6 sm:px-6">
            <Routes>
              <Route path="/" element={<Home />} />
              <Route path="/playlists/:id" element={<PlaylistDetail />} />
              <Route path="/channels/:id" element={<ChannelDetail />} />
              <Route path="/tasks" element={<TasksView />} />
            </Routes>
          </div>
        </main>
        <AddDialog open={addDialogOpen} onOpenChange={setAddDialogOpen} />
      </div>
    </BrowserRouter>
  )
}
