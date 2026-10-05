import { useRef, useState } from 'react'
import { BrowserRouter, Link, Route, Routes, useLocation } from 'react-router-dom'
import { Home } from './components/Home'
import { PlaylistDetail } from './components/PlaylistDetail'
import { ChannelDetail } from './components/ChannelDetail'
import { TasksView } from './components/TasksView'
import { AddChannelDialog } from './components/AddChannelDialog'
import { AddPlaylistDialog } from './components/AddPlaylistDialog'
import { SIDEBAR_ID, Sidebar } from './components/Sidebar'
import { SettingsMenu } from './components/SettingsMenu'
import { AnnouncementBar } from './components/AnnouncementBar'
import { MobileNavToggle } from './components/MobileNavToggle'

export default function App() {
  return (
    <BrowserRouter>
      <AppShell />
    </BrowserRouter>
  )
}

/** The routed layout; split from `App` so tests can render it in a memory router. */
export function AppShell() {
  const [addDialog, setAddDialog] = useState<'channel' | 'playlist' | null>(null)
  const [sidebarOpen, setSidebarOpen] = useState(false)
  const closeSidebar = () => setSidebarOpen(false)
  // The mobile sidebar opens right below the header, wherever it sits: an
  // announcement bar above it pushes it down until scrolled away. The page
  // can't scroll while the sidebar is open, so measuring once holds.
  const headerRef = useRef<HTMLElement>(null)
  const [sidebarTop, setSidebarTop] = useState(0)
  const toggleSidebar = () => {
    if (!sidebarOpen) {
      setSidebarTop(headerRef.current?.getBoundingClientRect().bottom ?? 0)
    }
    setSidebarOpen(!sidebarOpen)
  }

  // Any navigation closes the mobile sidebar, whatever triggered it.
  const { pathname } = useLocation()
  const [shownPathname, setShownPathname] = useState(pathname)
  if (pathname !== shownPathname) {
    setShownPathname(pathname)
    setSidebarOpen(false)
  }

  return (
    <div className="flex min-h-dvh flex-col md:h-dvh">
      <AnnouncementBar />
      <header
        ref={headerRef}
        className="sticky top-0 z-20 flex h-(--header-height) shrink-0 items-center gap-4 border-b border-border bg-background px-4 sm:px-6"
      >
        <MobileNavToggle open={sidebarOpen} onToggle={toggleSidebar} controls={SIDEBAR_ID} />
        <Link
          to="/"
          onClick={closeSidebar}
          className="flex items-center no-underline"
          aria-label="Yarrtube"
        >
          <img src="/logo.png" alt="Yarrtube" className="h-10 w-auto sm:h-12" />
        </Link>
        <div className="ml-auto flex items-center gap-2">
          <SettingsMenu />
        </div>
      </header>
      <div className="flex min-h-0 flex-1 flex-col md:flex-row">
        <Sidebar
          open={sidebarOpen}
          top={sidebarTop}
          onClose={closeSidebar}
          onAddChannel={() => setAddDialog('channel')}
          onAddPlaylist={() => setAddDialog('playlist')}
        />
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
      <AddChannelDialog
        open={addDialog === 'channel'}
        onOpenChange={(open) => !open && setAddDialog(null)}
      />
      <AddPlaylistDialog
        open={addDialog === 'playlist'}
        onOpenChange={(open) => !open && setAddDialog(null)}
      />
    </div>
  )
}
