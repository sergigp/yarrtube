import { useState } from 'react'
import './App.css'
import { PlaylistList } from './components/PlaylistList'
import { PlaylistDetail } from './components/PlaylistDetail'
import { TasksView } from './components/TasksView'
import { AddPlaylistDialog } from './components/AddPlaylistDialog'

function PlaylistsTab() {
  const [selectedPlaylist, setSelectedPlaylist] = useState(null)

  if (selectedPlaylist) {
    return (
      <PlaylistDetail
        playlist={selectedPlaylist}
        onBack={() => setSelectedPlaylist(null)}
        onDeleted={() => setSelectedPlaylist(null)}
      />
    )
  }

  return <PlaylistList onSelect={setSelectedPlaylist} />
}

export default function App() {
  const [tab, setTab] = useState('playlists')
  const [addPlaylistOpen, setAddPlaylistOpen] = useState(false)

  return (
    <div className="app">
      <header className="app-header">
        <h1>Yarrtube</h1>
        <nav className="tabs">
          <button
            className={tab === 'playlists' ? 'tab active' : 'tab'}
            onClick={() => setTab('playlists')}
          >
            Playlists
          </button>
          <button
            className={tab === 'tasks' ? 'tab active' : 'tab'}
            onClick={() => setTab('tasks')}
          >
            Tasks
          </button>
        </nav>
        <div className="header-actions">
          <button className="primary-button" onClick={() => setAddPlaylistOpen(true)}>
            Add Playlist
          </button>
        </div>
      </header>
      <main className="app-main">
        {tab === 'playlists' ? <PlaylistsTab /> : <TasksView />}
      </main>
      <AddPlaylistDialog open={addPlaylistOpen} onOpenChange={setAddPlaylistOpen} />
    </div>
  )
}
