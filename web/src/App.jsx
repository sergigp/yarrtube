import { useState } from 'react'
import { BrowserRouter, Link, Route, Routes } from 'react-router-dom'
import './App.css'
import { Home } from './components/Home'
import { PlaylistDetail } from './components/PlaylistDetail'
import { ChannelDetail } from './components/ChannelDetail'
import { TasksView } from './components/TasksView'
import { AddDialog } from './components/AddDialog'

export default function App() {
  const [addDialogOpen, setAddDialogOpen] = useState(false)

  return (
    <BrowserRouter>
      <div className="app">
        <header className="app-header">
          <Link to="/" className="app-logo">
            <h1>Yarrtube</h1>
          </Link>
          <div className="header-actions">
            <Link to="/tasks" className="secondary-button">
              Tasks
            </Link>
            <button className="primary-button" onClick={() => setAddDialogOpen(true)}>
              Add
            </button>
          </div>
        </header>
        <main className="app-main">
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/playlists/:id" element={<PlaylistDetail />} />
            <Route path="/channels/:id" element={<ChannelDetail />} />
            <Route path="/tasks" element={<TasksView />} />
          </Routes>
        </main>
        <AddDialog open={addDialogOpen} onOpenChange={setAddDialogOpen} />
      </div>
    </BrowserRouter>
  )
}
