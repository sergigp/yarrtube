import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { aVideo } from '@/test/helpers'
import { VideoPlayer } from './VideoPlayer'

const PLAYLIST = { kind: 'playlist', id: 'PL1' } as const

function renderPlayer(subtitleLanguage: string | null) {
  return render(
    <VideoPlayer
      basePath="music"
      source={PLAYLIST}
      video={aVideo({ id: 'vid1', subtitle_language: subtitleLanguage })}
      autoplay={false}
      onVideoElement={() => {}}
    />,
  )
}

/** Each captions track the player offers, as its attributes. */
function tracks(container: HTMLElement) {
  return [...container.querySelectorAll('track')].map((track) => ({
    kind: track.getAttribute('kind'),
    src: track.getAttribute('src'),
    srclang: track.getAttribute('srclang'),
    label: track.getAttribute('label'),
    default: track.hasAttribute('default'),
  }))
}

describe('VideoPlayer', () => {
  it('offers the video subtitles as a captions track that starts off', () => {
    const { container } = renderPlayer('en')

    expect(tracks(container)).toEqual([
      {
        kind: 'subtitles',
        src: '/api/playlists/PL1/videos/vid1/subtitles.vtt',
        srclang: 'en',
        label: 'English',
        default: false,
      },
    ])
  })

  it('offers no captions track for a video without subtitles', () => {
    const { container } = renderPlayer(null)

    expect(tracks(container)).toEqual([])
  })
})
