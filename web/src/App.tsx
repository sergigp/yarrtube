import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from 'react'
import { BrowserRouter, Link, Route, Routes, useLocation } from 'react-router-dom'
import { Home } from './components/Home'
import { PlaylistDetail } from './components/PlaylistDetail'
import { ChannelDetail } from './components/ChannelDetail'
import { TasksView } from './components/TasksView'
import { CategoryDetail } from './components/CategoryDetail'
import { PreferencesView } from './components/PreferencesView'
import { AddChannelDialog } from './components/AddChannelDialog'
import { AddPlaylistDialog } from './components/AddPlaylistDialog'
import { AddCategoryDialog } from './components/AddCategoryDialog'
import { SIDEBAR_ID, Sidebar } from './components/Sidebar'
import { SettingsMenu } from './components/SettingsMenu'
import { ThemeToggle } from './components/ThemeToggle'
import { AnnouncementBar } from './components/AnnouncementBar'
import { MobileNavToggle } from './components/MobileNavToggle'
import { VersionLabel } from './components/VersionLabel'
import { SearchDialog } from './components/SearchDialog'
import { SearchTrigger } from './components/SearchTrigger'
import { useScrollDirection } from './hooks/useScrollDirection'
import { useSavedTheme } from './hooks/useTheme'
import { cn } from './lib/utils'

export default function App() {
  return (
    <BrowserRouter>
      <AppShell />
    </BrowserRouter>
  )
}

/** The routed layout; split from `App` so tests can render it in a memory router. */
export function AppShell() {
  useSavedTheme()
  const [addDialog, setAddDialog] = useState<'channel' | 'playlist' | 'category' | null>(null)
  const [sidebarOpen, setSidebarOpen] = useState(false)
  const closeSidebar = () => setSidebarOpen(false)
  // The mobile sidebar opens right below the header, wherever it sits: an
  // announcement bar above it pushes it down until scrolled away. The page
  // can't scroll while the sidebar is open, but the bar can still appear
  // (it loads asynchronously), reflow or be dismissed, so the header's bottom
  // is re-measured whenever the bar's slot changes size.
  const headerRef = useRef<HTMLElement>(null)
  const announcementRef = useRef<HTMLDivElement>(null)
  const [sidebarTop, setSidebarTop] = useState(0)
  useLayoutEffect(() => {
    if (!sidebarOpen) {
      return
    }
    const measure = () => setSidebarTop(headerRef.current?.getBoundingClientRect().bottom ?? 0)
    measure()
    const observer = new ResizeObserver(measure)
    if (announcementRef.current) {
      observer.observe(announcementRef.current)
    }
    return () => observer.disconnect()
  }, [sidebarOpen])
  const toggleSidebar = () => setSidebarOpen(!sidebarOpen)

  // Below `sm` the header slides away while scrolling down, unless the
  // sidebar is open or a video player is pinned below it.
  const scrollDirection = useScrollDirection()
  const headerHidden =
    scrollDirection === 'down' &&
    !sidebarOpen &&
    !document.querySelector('[data-pinned-player]')

  // ⌘K / Ctrl+K opens search from anywhere, unless another dialog is open.
  const [searchOpen, setSearchOpen] = useState(false)
  useEffect(() => {
    const openSearch = (event: KeyboardEvent) => {
      if (
        event.key.toLowerCase() === 'k' &&
        (event.metaKey || event.ctrlKey) &&
        !document.querySelector('[role=dialog]')
      ) {
        event.preventDefault()
        setSidebarOpen(false)
        setSearchOpen(true)
      }
    }
    document.addEventListener('keydown', openSearch)
    return () => document.removeEventListener('keydown', openSearch)
  }, [])

  // Any navigation closes the mobile sidebar, whatever triggered it.
  const { pathname } = useLocation()
  const [shownPathname, setShownPathname] = useState(pathname)
  if (pathname !== shownPathname) {
    setShownPathname(pathname)
    setSidebarOpen(false)
  }

  return (
    <div
      className="flex min-h-dvh flex-col md:h-dvh"
      style={headerHidden ? ({ '--header-offset': '0px' } as CSSProperties) : undefined}
    >
      <div ref={announcementRef} className="shrink-0">
        <AnnouncementBar />
      </div>
      <header
        ref={headerRef}
        className={cn(
          'sticky top-0 z-20 flex h-(--header-height) shrink-0 items-center gap-4 border-b border-border bg-background px-4 transition-transform duration-200 sm:px-6',
          headerHidden && 'max-sm:-translate-y-full',
        )}
      >
        <MobileNavToggle open={sidebarOpen} onToggle={toggleSidebar} controls={SIDEBAR_ID} />
        <Link
          to="/"
          onClick={closeSidebar}
          className="flex items-center no-underline"
          aria-label="Yarrtube"
        >
          <img
            src="/logo.png"
            alt="Yarrtube"
            // The intrinsic size reserves the logo's box before the image loads.
            width={2048}
            height={768}
            className="h-10 w-auto sm:h-12"
          />
        </Link>
        <VersionLabel />
        <div className="ml-auto flex items-center gap-2">
          <SearchTrigger
            onOpen={() => {
              closeSidebar()
              setSearchOpen(true)
            }}
          />
          <ThemeToggle />
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
          onAddCategory={() => setAddDialog('category')}
        />
        <main className="min-w-0 flex-1 md:min-h-0 md:overflow-y-auto">
          {/* The container video grids size their columns by. */}
          <div className="@container h-full px-4 pt-4 pb-6 sm:px-6 md:pt-6">
            <Routes>
              <Route path="/" element={<Home />} />
              <Route path="/playlists/:id" element={<PlaylistDetail />} />
              <Route path="/channels/:id" element={<ChannelDetail />} />
              <Route path="/tasks" element={<TasksView />} />
              <Route path="/preferences" element={<PreferencesView />} />
              <Route path="/categories/:id" element={<CategoryDetail />} />
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
      <AddCategoryDialog
        open={addDialog === 'category'}
        onOpenChange={(open) => !open && setAddDialog(null)}
      />
      <SearchDialog open={searchOpen} onOpenChange={setSearchOpen} />
    </div>
  )
}
