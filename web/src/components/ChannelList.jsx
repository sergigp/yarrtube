import { usePolling } from '../usePolling'
import { fetchChannels } from '../api'
import { ChannelActionsMenu } from './ChannelActionsMenu'

export function ChannelList() {
  const { data: channels, error } = usePolling(fetchChannels, [])

  if (error) {
    return <p className="error">Failed to load channels: {error.message}</p>
  }

  if (!channels) {
    return <p className="muted">Loading channels…</p>
  }

  if (channels.length === 0) {
    return <p className="muted">No channels tracked yet.</p>
  }

  return (
    <ul className="list">
      {channels.map((channel) => (
        <li key={channel.id} className="list-row">
          <span className="list-item">
            <span className="list-item-title">{channel.name}</span>
            <span className="chip-group">
              <span className="status-badge">{channel.quality}</span>
              <span className="status-badge">limit {channel.video_limit}</span>
            </span>
          </span>
          <ChannelActionsMenu channel={channel} />
        </li>
      ))}
    </ul>
  )
}
