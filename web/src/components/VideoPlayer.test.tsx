import { act, fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, onTestFinished, vi } from 'vitest'
import type { Chapter, Video } from '@/api/types'
import { aVideo } from '@/test/helpers'
import { rotatePhone, useDesktopViewport } from '@/test/viewport'
import { VideoPlayer } from './VideoPlayer'


function renderPlayer(subtitleLanguage: string | null) {
  return render(
    <VideoPlayer
      basePath="music"
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
        src: '/api/videos/vid1/subtitles.vtt',
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

  describe('autoplay', () => {
    afterEach(() => {
      vi.restoreAllMocks()
    })

    function player(video: Video | null, autoplay: boolean) {
      return (
        <VideoPlayer basePath="music" video={video} autoplay={autoplay} onVideoElement={() => {}} />
      )
    }

    it('starts playing a downloaded video selected with autoplay', () => {
      const play = vi.spyOn(HTMLMediaElement.prototype, 'play')

      const { container } = render(player(aVideo({ id: 'vid1' }), true))

      expect(play).toHaveBeenCalledTimes(1)
      expect(container.querySelector('video')!.hasAttribute('autoplay')).toBe(false)
    })

    it('does not start a video selected without autoplay', () => {
      const play = vi.spyOn(HTMLMediaElement.prototype, 'play')

      render(player(aVideo({ id: 'vid1' }), false))

      expect(play).not.toHaveBeenCalled()
    })

    it('starts each newly picked video in the same video element', () => {
      const play = vi.spyOn(HTMLMediaElement.prototype, 'play')
      const { container, rerender } = render(player(aVideo({ id: 'vid1' }), true))
      const first = container.querySelector('video')

      rerender(player(aVideo({ id: 'vid2', filename: 'vid2.mp4' }), true))

      expect(play.mock.contexts).toEqual([first, first])
      expect(container.querySelector('video')).toBe(first)
    })

    it('does not start the video again when it re-renders', () => {
      const play = vi.spyOn(HTMLMediaElement.prototype, 'play')
      const { rerender } = render(player(aVideo({ id: 'vid1' }), true))

      rerender(player(aVideo({ id: 'vid1' }), true))

      expect(play).toHaveBeenCalledTimes(1)
    })

    it('does not start a video that finishes downloading while selected', () => {
      const play = vi.spyOn(HTMLMediaElement.prototype, 'play')
      const { rerender } = render(
        player(aVideo({ id: 'vid1', status: 'IN_PROGRESS', filename: null }), true),
      )

      rerender(player(aVideo({ id: 'vid1' }), true))

      expect(play).not.toHaveBeenCalled()
    })

    it('stays paused without an error when the browser refuses to play', async () => {
      const refused = new DOMException('needs a user gesture', 'NotAllowedError')
      // Not a spy: one marks the promise it returns as handled.
      const originalPlay = HTMLMediaElement.prototype.play
      let plays = 0
      HTMLMediaElement.prototype.play = () => {
        plays += 1
        return Promise.reject(refused)
      }
      const unhandled = vi.fn()
      process.on('unhandledRejection', unhandled)
      onTestFinished(() => {
        HTMLMediaElement.prototype.play = originalPlay
        process.off('unhandledRejection', unhandled)
      })

      const { container } = render(player(aVideo({ id: 'vid1' }), true))
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(plays).toBe(1)
      expect(unhandled).not.toHaveBeenCalled()
      expect(container.querySelector('video')).toBeInTheDocument()
      expect(container).not.toHaveTextContent(/error/i)
    })
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

  describe('landscape takeover', () => {
    function player(video: Video | null) {
      return (
        <VideoPlayer basePath="music" video={video} autoplay={false} onVideoElement={() => {}} />
      )
    }

    /** The player's root, which takes over the screen. */
    function playerRoot(container: HTMLElement): HTMLElement {
      return container.querySelector<HTMLElement>('[data-pinned-player]')!
    }

    it('takes over the screen with a downloaded video on a phone in landscape', () => {
      rotatePhone('landscape')

      const { container } = render(player(aVideo({ id: 'vid1' })))

      expect(playerRoot(container)).toHaveAttribute('data-landscape-takeover')
      expect(playerRoot(container)).toHaveClass('fixed', 'inset-0', 'bg-black')
      expect(container.querySelector('video')).toHaveClass('object-contain')
    })

    it('hides the chapter rail while taking over', () => {
      const chapters: Chapter[] = [
        { start_seconds: 0, end_seconds: 75, title: 'Intro' },
        { start_seconds: 75, end_seconds: 200, title: 'Game 1' },
      ]
      render(player(aVideo({ id: 'vid1', chapters })))

      act(() => rotatePhone('landscape'))
      const railInLandscape = screen.queryByRole('navigation', { name: 'Chapters' })
      act(() => rotatePhone('portrait'))

      expect(railInLandscape).not.toBeInTheDocument()
      expect(screen.getByRole('navigation', { name: 'Chapters' })).toBeInTheDocument()
    })

    it('does not take over without a downloaded video', () => {
      rotatePhone('landscape')

      const pending = render(player(aVideo({ status: 'PENDING', filename: null })))
      const pendingTakesOver = playerRoot(pending.container).hasAttribute('data-landscape-takeover')
      pending.unmount()
      const none = render(player(null))

      expect(pendingTakesOver).toBe(false)
      expect(playerRoot(none.container)).not.toHaveAttribute('data-landscape-takeover')
    })
  })
})
