import { useEffect, useState } from 'react'
import './App.css'
import { fetchPlaylists, fetchChannels } from './api'
import { Home } from './components/Home'
import { PlaylistList } from './components/PlaylistList'
import { PlaylistDetail } from './components/PlaylistDetail'
import { ChannelList } from './components/ChannelList'
import { ChannelDetail } from './components/ChannelDetail'
import { TasksView } from './components/TasksView'
import { AddPlaylistDialog } from './components/AddPlaylistDialog'
import { AddChannelDialog } from './components/AddChannelDialog'

function PlaylistsTab({ deepLink, onDeepLinkHandled }) {
  const [selectedPlaylist, setSelectedPlaylist] = useState(null)
  const [initialVideoId, setInitialVideoId] = useState(null)

  useEffect(() => {
    if (!deepLink) {
      return undefined
    }

    let cancelled = false
    fetchPlaylists().then((playlists) => {
      if (cancelled) {
        return
      }
      const match = playlists.find((playlist) => playlist.id === deepLink.id)
      if (match) {
        setSelectedPlaylist(match)
        setInitialVideoId(deepLink.videoId)
      }
      onDeepLinkHandled()
    })
    return () => {
      cancelled = true
    }
  }, [deepLink, onDeepLinkHandled])

  if (selectedPlaylist) {
    return (
      <PlaylistDetail
        playlist={selectedPlaylist}
        initialVideoId={initialVideoId}
        onBack={() => {
          setSelectedPlaylist(null)
          setInitialVideoId(null)
        }}
        onDeleted={() => {
          setSelectedPlaylist(null)
          setInitialVideoId(null)
        }}
      />
    )
  }

  return <PlaylistList onSelect={setSelectedPlaylist} />
}

function ChannelsTab({ deepLink, onDeepLinkHandled }) {
  const [selectedChannel, setSelectedChannel] = useState(null)
  const [initialVideoId, setInitialVideoId] = useState(null)

  useEffect(() => {
    if (!deepLink) {
      return undefined
    }

    let cancelled = false
    fetchChannels().then((channels) => {
      if (cancelled) {
        return
      }
      const match = channels.find((channel) => channel.id === deepLink.id)
      if (match) {
        setSelectedChannel(match)
        setInitialVideoId(deepLink.videoId)
      }
      onDeepLinkHandled()
    })
    return () => {
      cancelled = true
    }
  }, [deepLink, onDeepLinkHandled])

  if (selectedChannel) {
    return (
      <ChannelDetail
        channel={selectedChannel}
        initialVideoId={initialVideoId}
        onBack={() => {
          setSelectedChannel(null)
          setInitialVideoId(null)
        }}
        onDeleted={() => {
          setSelectedChannel(null)
          setInitialVideoId(null)
        }}
      />
    )
  }

  return <ChannelList onSelect={setSelectedChannel} />
}

export default function App() {
  const [tab, setTab] = useState('home')
  const [addPlaylistOpen, setAddPlaylistOpen] = useState(false)
  const [addChannelOpen, setAddChannelOpen] = useState(false)
  const [deepLink, setDeepLink] = useState(null)

  const handleHomeSelect = (source, videoId) => {
    setDeepLink({ kind: source.kind, id: source.id, videoId })
    setTab(source.kind === 'channel' ? 'channels' : 'playlists')
  }

  return (
    <div className="app">
      <header className="app-header">
        <h1>Yarrtube</h1>
        <nav className="tabs">
          <button
            className={tab === 'home' ? 'tab active' : 'tab'}
            onClick={() => setTab('home')}
          >
            Home
          </button>
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
        {tab === 'home' && <Home onSelect={handleHomeSelect} />}
        {tab === 'playlists' && (
          <PlaylistsTab
            deepLink={deepLink?.kind === 'playlist' ? deepLink : null}
            onDeepLinkHandled={() => setDeepLink(null)}
          />
        )}
        {tab === 'channels' && (
          <ChannelsTab
            deepLink={deepLink?.kind === 'channel' ? deepLink : null}
            onDeepLinkHandled={() => setDeepLink(null)}
          />
        )}
        {tab === 'tasks' && <TasksView />}
      </main>
      <AddPlaylistDialog open={addPlaylistOpen} onOpenChange={setAddPlaylistOpen} />
      <AddChannelDialog open={addChannelOpen} onOpenChange={setAddChannelOpen} />
    </div>
  )
}
