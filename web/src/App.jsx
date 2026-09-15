import { useState } from 'react'
import './App.css'
import { PlaylistList } from './components/PlaylistList'
import { PlaylistDetail } from './components/PlaylistDetail'
import { ChannelList } from './components/ChannelList'
import { ChannelDetail } from './components/ChannelDetail'
import { TasksView } from './components/TasksView'
import { AddPlaylistDialog } from './components/AddPlaylistDialog'
import { AddChannelDialog } from './components/AddChannelDialog'

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

function ChannelsTab() {
  const [selectedChannel, setSelectedChannel] = useState(null)

  if (selectedChannel) {
    return (
      <ChannelDetail
        channel={selectedChannel}
        onBack={() => setSelectedChannel(null)}
        onDeleted={() => setSelectedChannel(null)}
      />
    )
  }

  return <ChannelList onSelect={setSelectedChannel} />
}

export default function App() {
  const [tab, setTab] = useState('playlists')
  const [addPlaylistOpen, setAddPlaylistOpen] = useState(false)
  const [addChannelOpen, setAddChannelOpen] = useState(false)

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
            className={tab === 'channels' ? 'tab active' : 'tab'}
            onClick={() => setTab('channels')}
          >
            Channels
          </button>
          <button
            className={tab === 'tasks' ? 'tab active' : 'tab'}
            onClick={() => setTab('tasks')}
          >
            Tasks
          </button>
        </nav>
        <div className="header-actions">
          {tab === 'channels' ? (
            <button className="primary-button" onClick={() => setAddChannelOpen(true)}>
              Add Channel
            </button>
          ) : (
            <button className="primary-button" onClick={() => setAddPlaylistOpen(true)}>
              Add Playlist
            </button>
          )}
        </div>
      </header>
      <main className="app-main">
        {tab === 'playlists' && <PlaylistsTab />}
        {tab === 'channels' && <ChannelsTab />}
        {tab === 'tasks' && <TasksView />}
      </main>
      <AddPlaylistDialog open={addPlaylistOpen} onOpenChange={setAddPlaylistOpen} />
      <AddChannelDialog open={addChannelOpen} onOpenChange={setAddChannelOpen} />
    </div>
  )
}
