import { useState } from 'react'
import './App.css'
import { PlaylistList } from './components/PlaylistList'
import { PlaylistDetail } from './components/PlaylistDetail'
import { TasksView } from './components/TasksView'

function PlaylistsTab() {
  const [selectedPlaylist, setSelectedPlaylist] = useState(null)

  if (selectedPlaylist) {
    return (
      <PlaylistDetail
        playlist={selectedPlaylist}
        onBack={() => setSelectedPlaylist(null)}
      />
    )
  }

  return <PlaylistList onSelect={setSelectedPlaylist} />
}

export default function App() {
  const [tab, setTab] = useState('playlists')

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
      </header>
      <main className="app-main">
        {tab === 'playlists' ? <PlaylistsTab /> : <TasksView />}
      </main>
    </div>
  )
}
