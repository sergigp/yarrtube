import { act, fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Chapter, Video } from '@/api/types'
import { aVideo } from '@/test/helpers'
import { useDesktopViewport } from '@/test/viewport'
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

  it('plays inline instead of going fullscreen on phones', () => {
    const { container } = renderPlayer(null)

    expect(container.querySelector('video')!.hasAttribute('playsinline')).toBe(true)
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
      return within(within(rail).getByRole('list'))
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

    it('moves the current mark when playback enters the next chapter', () => {
      const { container } = renderWith({ chapters: CHAPTERS })

      playTo(container, 1000)
      const before = currentCard()
      playTo(container, 1100)

      expect(before).toBe('1:15 Game 1')
      expect(currentCard()).toBe('17:17 Game 2')
    })

    it('marks no chapter before the first chapter starts', () => {
      const { container } = renderWith({
        chapters: [{ start_seconds: 10, end_seconds: 60, title: 'Late start' }],
      })

      playTo(container, 5)

      expect(currentCard()).toBeNull()
    })

    it('seeks to a chapter start and plays when its card is clicked', async () => {
      const play = vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined)
      const { container } = renderWith({ chapters: CHAPTERS })
      playTo(container, 30)
      const user = userEvent.setup()

      await user.click(screen.getByRole('button', { name: '17:17 Game 2' }))

      expect(container.querySelector('video')!.currentTime).toBe(1037)
      expect(play).toHaveBeenCalled()
      play.mockRestore()
    })

    it('scrolls the current card into view when the current chapter changes', () => {
      const scrolled: Element[] = []
      const scrollTo = vi
        .spyOn(Element.prototype, 'scrollTo')
        .mockImplementation(function (this: Element) {
          scrolled.push(this)
        })
      const { container } = renderWith({ chapters: CHAPTERS })
      const rail = within(screen.getByRole('navigation', { name: 'Chapters' })).getByRole('list')
      playTo(container, 1000)
      const scrollsBefore = scrolled.length

      playTo(container, 1010)
      const scrollsWithinChapter = scrolled.length
      playTo(container, 1100)

      expect(scrollsWithinChapter).toBe(scrollsBefore)
      expect(scrolled.length).toBe(scrollsBefore + 1)
      expect(scrolled.at(-1)).toBe(rail)
      scrollTo.mockRestore()
    })

    describe('on desktop', () => {
      useDesktopViewport()

      it('offers no "All chapters" control on desktop', () => {
        renderWith({ chapters: CHAPTERS })

        expect(screen.queryByRole('button', { name: 'All chapters' })).not.toBeInTheDocument()
      })
    })

    it('lists every chapter in a sheet from "All chapters" on mobile, marking the current one', async () => {
      const { container } = renderWith({ chapters: CHAPTERS })
      playTo(container, 1000)
      const user = userEvent.setup()

      await user.click(screen.getByRole('button', { name: 'All chapters' }))

      const sheet = screen.getByRole('dialog', { name: 'Chapters' })
      const rows = within(sheet).getAllByRole('button', { name: /^\d/ })
      expect(rows.map((row) => row.textContent)).toEqual([
        '0:00 Intro',
        '1:15 Game 1',
        '17:17 Game 2',
      ])
      expect(within(sheet).getByRole('button', { current: true })).toHaveTextContent('1:15 Game 1')
    })

    it('seeks, plays and closes the sheet when a chapter is picked from it', async () => {
      const play = vi.spyOn(HTMLMediaElement.prototype, 'play').mockResolvedValue(undefined)
      const { container } = renderWith({ chapters: CHAPTERS })
      const user = userEvent.setup()
      await user.click(screen.getByRole('button', { name: 'All chapters' }))

      const sheet = screen.getByRole('dialog', { name: 'Chapters' })
      await user.click(within(sheet).getByRole('button', { name: '17:17 Game 2' }))

      expect(container.querySelector('video')!.currentTime).toBe(1037)
      expect(play).toHaveBeenCalled()
      expect(screen.queryByRole('dialog', { name: 'Chapters' })).not.toBeInTheDocument()
      play.mockRestore()
    })
  })
})
