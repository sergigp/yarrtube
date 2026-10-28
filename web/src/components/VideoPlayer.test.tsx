import { act, fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
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

  it('starts captions off again when another video is selected', () => {
    const player = (id: string) => (
      <VideoPlayer
        basePath="music"
        source={PLAYLIST}
        video={aVideo({ id, subtitle_language: 'en' })}
        autoplay={false}
        onVideoElement={() => {}}
      />
    )
    const { container, rerender } = render(player('vid1'))
    const firstTrack = container.querySelector('track')

    rerender(player('vid2'))

    // A reused <track> would keep the captions mode the user picked for the previous video.
    expect(container.querySelector('track')).not.toBe(firstTrack)
  })

  it('offers no captions track for a video without subtitles', () => {
    const { container } = renderPlayer(null)

    expect(tracks(container)).toEqual([])
  })

  describe('arrow key seeking', () => {
    afterEach(() => {
      vi.useRealTimers()
    })

    /** The player's <video>, 100 s long and 30 s in. */
    function playerVideo(container: HTMLElement): HTMLVideoElement {
      const video = container.querySelector('video')!
      Object.defineProperty(video, 'duration', { value: 100 })
      video.currentTime = 30
      return video
    }

    it('seeks 10 s forward on → and back on ←', async () => {
      const video = playerVideo(renderPlayer(null).container)
      const user = userEvent.setup()

      await user.keyboard('{ArrowRight}')
      const afterForward = video.currentTime
      await user.keyboard('{ArrowLeft}{ArrowLeft}')

      expect(afterForward).toBe(40)
      expect(video.currentTime).toBe(20)
    })

    it('shows the seconds sought, adding up quick presses, then fades out', async () => {
      vi.useFakeTimers()
      playerVideo(renderPlayer(null).container)
      const press = (key: string) => fireEvent.keyDown(document.body, { key })

      press('ArrowRight')
      press('ArrowRight')
      const shown = screen.getByRole('status').textContent
      press('ArrowLeft')
      const switched = screen.getByRole('status').textContent
      act(() => {
        vi.advanceTimersByTime(700)
      })

      expect(shown).toBe('20 seconds')
      expect(switched).toBe('10 seconds')
      expect(screen.queryByRole('status')).not.toBeInTheDocument()
    })

    it('leaves arrows typed into a field alone', async () => {
      const { container } = render(
        <>
          <input aria-label="Search" />
          <VideoPlayer
            basePath="music"
            source={PLAYLIST}
            video={aVideo()}
            autoplay={false}
            onVideoElement={() => {}}
          />
        </>,
      )
      const video = playerVideo(container)
      const user = userEvent.setup()

      await user.click(screen.getByRole('textbox', { name: 'Search' }))
      await user.keyboard('{ArrowLeft}')

      expect(video.currentTime).toBe(30)
      expect(screen.queryByRole('status')).not.toBeInTheDocument()
    })
  })
})
