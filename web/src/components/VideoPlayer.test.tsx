import { act, fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Chapter, Video } from '@/api/types'
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

  describe('chapter rail', () => {
    const CHAPTERS: Chapter[] = [
      { start_seconds: 0, end_seconds: 75, title: 'Intro' },
      { start_seconds: 75, end_seconds: 1037, title: 'Game 1' },
      { start_seconds: 1037, end_seconds: 2201, title: 'Game 2' },
    ]

    function renderWith(video: Partial<Video>) {
      return render(
        <VideoPlayer
          basePath="music"
          source={PLAYLIST}
          video={aVideo({ id: 'vid1', ...video })}
          autoplay={false}
          onVideoElement={() => {}}
        />,
      )
    }

    /** The text of each chapter card in the rail, in order. */
    function railCards(): string[] {
      const rail = screen.getByRole('navigation', { name: 'Chapters' })
      return within(rail)
        .getAllByRole('button')
        .map((card) => card.textContent ?? '')
    }

    it('shows a card per chapter with its start time and title, in order', () => {
      renderWith({ chapters: CHAPTERS })

      expect(railCards()).toEqual(['0:00 Intro', '1:15 Game 1', '17:17 Game 2'])
    })

    it('shows hour-long start times as h:mm:ss', () => {
      renderWith({ chapters: [{ start_seconds: 3725, end_seconds: 4000, title: 'Overtime' }] })

      expect(railCards()).toEqual(['1:02:05 Overtime'])
    })

    it('shows no rail for a video without chapters', () => {
      renderWith({ chapters: [] })

      expect(screen.queryByRole('navigation', { name: 'Chapters' })).not.toBeInTheDocument()
    })

    it('shows no rail for a video that is not downloaded', () => {
      renderWith({ status: 'PENDING', filename: null, chapters: CHAPTERS })

      expect(screen.queryByRole('navigation', { name: 'Chapters' })).not.toBeInTheDocument()
    })

    /** Moves the player's <video> to `seconds`, as playback would. */
    function playTo(container: HTMLElement, seconds: number) {
      const video = container.querySelector('video')!
      video.currentTime = seconds
      fireEvent.timeUpdate(video)
    }

    /** The text of the card marked as the current chapter, or null. */
    function currentCard(): string | null {
      const rail = screen.getByRole('navigation', { name: 'Chapters' })
      return within(rail).queryByRole('button', { current: true })?.textContent ?? null
    }

    it('marks the chapter containing the playback position as current', () => {
      const { container } = renderWith({ chapters: CHAPTERS })

      playTo(container, 1500)

      expect(currentCard()).toBe('17:17 Game 2')
    })
  })
})
