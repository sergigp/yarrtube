import { describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { DetailHeader } from './DetailHeader'
import { MemoryRouter } from 'react-router-dom'
import { useDesktopViewport } from '@/test/viewport'

const noop = async () => {}

describe('DetailHeader', () => {
  it('shows the badge, name and meta line', () => {
    render(
      <DetailHeader
        name="The Channel"
        badge={{ kind: 'color', color: '#ff0000' }}
        summary="3 videos · 2 unwatched"
        onSync={noop}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    expect([
      screen.getByRole('heading', { name: 'The Channel' }),
      screen.getByText('3 videos · 2 unwatched'),
    ]).toHaveLength(2)
  })

  it('shows no meta line until it is known', () => {
    render(
      <DetailHeader
        name="The Channel"
        badge={{ kind: 'playlist' }}
        summary={null}
        onSync={noop}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    expect(screen.queryByText(/videos/)).not.toBeInTheDocument()
  })

  it('links the name back when given a link', () => {
    render(
      <MemoryRouter>
        <DetailHeader
          name="The Playlist"
          badge={{ kind: 'playlist' }}
          nameHref="/playlists/PL1"
          summary="1 video"
          onDelete={noop}
          deleteDescription="desc"
        />
      </MemoryRouter>,
    )

    expect(screen.getByRole('link', { name: 'The Playlist' })).toHaveAttribute(
      'href',
      '/playlists/PL1',
    )
  })

  it('keeps every action, Sync included, behind the ⋮ menu on mobile', async () => {
    const onSync = vi.fn(noop)
    render(
      <DetailHeader
        name="The Channel"
        badge={{ kind: 'playlist' }}
        summary="0 videos"
        onSync={onSync}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )
    const user = userEvent.setup()

    expect(screen.queryByRole('button', { name: 'Sync' })).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Sync' }))

    expect(onSync).toHaveBeenCalledOnce()
  })

  it('links the source on YouTube from the ⋮ menu on mobile, in a new tab', async () => {
    render(
      <DetailHeader
        name="The Channel"
        badge={{ kind: 'playlist' }}
        summary="0 videos"
        youtubeLink={{ href: 'https://www.youtube.com/@thechannel', label: 'Open channel on YouTube' }}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    await userEvent
      .setup()
      .click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    const item = await screen.findByRole('menuitem', { name: 'Open channel on YouTube' })

    expect(item).toHaveAttribute('href', 'https://www.youtube.com/@thechannel')
    expect(item).toHaveAttribute('target', '_blank')
  })

  it('offers mark-all-watched in its menu only when supported', async () => {
    const user = userEvent.setup()
    const { rerender } = render(
      <DetailHeader
        name="The Channel"
        badge={{ kind: 'playlist' }}
        summary="0 videos"
        onSync={noop}
        onMarkWatched={noop}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    expect(await screen.findByRole('menuitem', { name: 'Mark all watched' })).toBeInTheDocument()
    await user.keyboard('{Escape}')

    rerender(
      <DetailHeader
        name="The Channel"
        badge={{ kind: 'playlist' }}
        summary="0 videos"
        onSync={noop}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    expect(await screen.findByRole('menuitem', { name: 'Delete' })).toBeInTheDocument()
    expect(screen.queryByRole('menuitem', { name: 'Mark all watched' })).not.toBeInTheDocument()
  })

  it('asks before marking 2 or more unwatched videos watched', async () => {
    const user = userEvent.setup()
    const onMarkWatched = vi.fn(noop)
    render(
      <DetailHeader
        name="The Channel"
        badge={{ kind: 'playlist' }}
        summary="0 videos"
        unwatchedCount={23}
        onMarkWatched={onMarkWatched}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))

    const dialog = await screen.findByRole('dialog', {
      name: 'Mark 23 videos in "The Channel" watched?',
    })
    expect(dialog).toHaveTextContent(
      "This can't be undone in bulk: videos can only be marked unwatched one by one.",
    )
    expect(onMarkWatched).not.toHaveBeenCalled()

    await user.click(screen.getByRole('button', { name: 'Mark watched' }))

    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    expect(onMarkWatched).toHaveBeenCalledOnce()
  })

  it('marks nothing when the mark-all-watched confirmation is cancelled', async () => {
    const user = userEvent.setup()
    const onMarkWatched = vi.fn(noop)
    render(
      <DetailHeader
        name="The Channel"
        badge={{ kind: 'playlist' }}
        summary="0 videos"
        unwatchedCount={5}
        onMarkWatched={onMarkWatched}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))
    await user.click(await screen.findByRole('button', { name: 'Cancel' }))

    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument())
    expect(onMarkWatched).not.toHaveBeenCalled()
  })

  it('asks even before marking a single unwatched video watched', async () => {
    const user = userEvent.setup()
    const onMarkWatched = vi.fn(noop)
    render(
      <DetailHeader
        name="The Channel"
        badge={{ kind: 'playlist' }}
        summary="1 video"
        unwatchedCount={1}
        onMarkWatched={onMarkWatched}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))

    expect(
      await screen.findByRole('dialog', { name: 'Mark 1 video in "The Channel" watched?' }),
    ).toBeVisible()
    expect(onMarkWatched).not.toHaveBeenCalled()
  })

  it('keeps the mark-all-watched confirmation open with the error when marking fails', async () => {
    const user = userEvent.setup()
    const onMarkWatched = vi.fn(() => Promise.reject(new Error('mark failed')))
    render(
      <DetailHeader
        name="The Channel"
        badge={{ kind: 'playlist' }}
        summary="0 videos"
        unwatchedCount={3}
        onMarkWatched={onMarkWatched}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))
    await user.click(await screen.findByRole('button', { name: 'Mark watched' }))

    const dialog = await screen.findByRole('dialog', {
      name: 'Mark 3 videos in "The Channel" watched?',
    })
    expect(await within(dialog).findByText('mark failed')).toBeInTheDocument()
  })

  it('lists extra actions in the menu, and no Sync without onSync', async () => {
    render(
      <DetailHeader
        name="Chess"
        badge={{ kind: 'color', color: '#ff0000' }}
        summary="2 videos"
        extraActions={[{ label: 'Edit category', icon: null, onSelect: () => {} }]}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    await userEvent.setup().click(screen.getByRole('button', { name: 'More actions for Chess' }))
    const items = (await screen.findAllByRole('menuitem')).map((item) => item.textContent)

    expect(items).toEqual(['Edit category', 'Delete'])
  })

  describe('on desktop', () => {
    useDesktopViewport()

    it('links the source on YouTube as an icon, in a new tab', () => {
      render(
        <DetailHeader
          name="The Playlist"
          badge={{ kind: 'playlist' }}
        summary="0 videos"
          youtubeLink={{ href: 'https://www.youtube.com/playlist?list=PL1', label: 'Open playlist on YouTube' }}
          onDelete={noop}
          deleteDescription="desc"
        />,
      )

      const link = screen.getByRole('link', { name: 'Open playlist on YouTube' })
      expect(link).toHaveAttribute('href', 'https://www.youtube.com/playlist?list=PL1')
      expect(link).toHaveAttribute('target', '_blank')
    })

    it('runs the sync action and disables the button meanwhile', async () => {
      let resolveSync = () => {}
      const onSync = vi.fn(
        () =>
          new Promise<void>((resolve) => {
            resolveSync = resolve
          }),
      )
      render(
        <DetailHeader
          name="The Channel"
          badge={{ kind: 'playlist' }}
        summary="0 videos"
          onSync={onSync}
          onDelete={noop}
          deleteDescription="desc"
        />,
      )

      const sync = screen.getByRole('button', { name: 'Sync' })
      await userEvent.setup().click(sync)

      expect(onSync).toHaveBeenCalledOnce()
      expect(sync).toBeDisabled()
      resolveSync()
      await waitFor(() => expect(sync).toBeEnabled())
    })

    it('alerts when syncing fails', async () => {
      const alert = vi.fn()
      vi.stubGlobal('alert', alert)
      render(
        <DetailHeader
          name="The Channel"
          badge={{ kind: 'playlist' }}
        summary="0 videos"
          onSync={async () => {
            throw new Error('yt down')
          }}
          onDelete={noop}
          deleteDescription="desc"
        />,
      )

      await userEvent.setup().click(screen.getByRole('button', { name: 'Sync' }))

      await waitFor(() =>
        expect(alert).toHaveBeenCalledWith('Failed to sync "The Channel": yt down'),
      )
    })

    it('names an icon button in a tooltip on hover', async () => {
      render(
        <DetailHeader
          name="The Channel"
          badge={{ kind: 'playlist' }}
        summary="0 videos"
          onSync={noop}
          onDelete={noop}
          deleteDescription="desc"
        />,
      )

      await userEvent.setup().hover(screen.getByRole('button', { name: 'Sync' }))

      expect(await screen.findByRole('tooltip')).toHaveTextContent('Sync')
    })

    it('lays out Sync, YouTube and icon extras as buttons, then a ⋮ menu with the rest', async () => {
      const onEditRequest = vi.fn()
      render(
        <DetailHeader
          name="The Channel"
          badge={{ kind: 'playlist' }}
          summary="0 videos"
          primaryAction={<button type="button">Play next</button>}
          onSync={noop}
          youtubeLink={{ href: 'https://www.youtube.com/@x', label: 'Open channel on YouTube' }}
          extraActions={[
            { label: 'Edit category', icon: null, onSelect: () => {}, iconOnDesktop: true },
          ]}
          onMarkWatched={noop}
          excludedFromHome={false}
          onSetExcludedFromHome={noop}
          unwatchedCountHidden={false}
          onSetUnwatchedCountHidden={noop}
          onEditRequest={onEditRequest}
          onDelete={noop}
          deleteDescription="desc"
        />,
      )

      const controls = [...screen.getAllByRole('button'), ...screen.getAllByRole('link')].map(
        (control) => control.getAttribute('aria-label') ?? control.textContent,
      )
      const user = userEvent.setup()
      await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
      const items = (await screen.findAllByRole('menuitem')).map((item) => item.textContent)
      await user.click(screen.getByRole('menuitem', { name: 'Edit settings' }))

      expect(controls).toEqual([
        'Play next',
        'Sync',
        'Edit category',
        'More actions for The Channel',
        'Open channel on YouTube',
      ])
      expect(items).toEqual([
        'Mark all watched',
        'Exclude from home',
        'Hide unwatched count',
        'Edit settings',
        'Delete',
      ])
      expect(onEditRequest).toHaveBeenCalledOnce()
    })
  })

  it('offers Re-run AI categorization or Clear AI categories after confirmation', async () => {
    const onDelete = vi.fn(noop)
    const header = (enabled: boolean) => (
      <DetailHeader
        name="@GothamChess"
        badge={{ kind: 'playlist' }}
        summary="0 videos"
        onSync={noop}
        aiCategorization={{ enabled, onDeleteAiVerdicts: onDelete }}
        onDelete={noop}
        deleteDescription="desc"
      />
    )
    const { rerender } = render(header(true))
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'More actions for @GothamChess' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Re-run AI categorization' }))
    const rerun = await screen.findByRole('dialog', {
      name: 'Re-run AI categorization of "@GothamChess"?',
    })
    await user.click(within(rerun).getByRole('button', { name: 'Cancel' }))
    expect(onDelete).not.toHaveBeenCalled()

    rerender(header(false))
    await user.click(screen.getByRole('button', { name: 'More actions for @GothamChess' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Clear AI categories' }))
    const clear = await screen.findByRole('dialog', {
      name: 'Clear AI categories of "@GothamChess"?',
    })
    await user.click(within(clear).getByRole('button', { name: 'Clear' }))

    await waitFor(() => expect(onDelete).toHaveBeenCalledOnce())
  })
})
