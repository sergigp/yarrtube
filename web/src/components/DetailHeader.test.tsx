import { describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { DetailHeader } from './DetailHeader'
import { aVideo } from '@/test/helpers'
import { useDesktopViewport } from '@/test/viewport'

const noop = async () => {}

describe('DetailHeader', () => {
  it('summarizes the videos and unwatched count', () => {
    render(
      <DetailHeader
        name="The Channel"
        videos={[aVideo(), aVideo(), aVideo()]}
        unwatchedCount={2}
        onSync={noop}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    expect(screen.getByText('3 videos · 2 unwatched')).toBeInTheDocument()
  })

  it('drops the unwatched part when everything is watched, and singularizes one video', () => {
    render(
      <DetailHeader
        name="The Channel"
        videos={[aVideo()]}
        unwatchedCount={0}
        onSync={noop}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    expect(screen.getByText('1 video')).toBeInTheDocument()
  })

  it('shows no summary until the videos have loaded', () => {
    render(
      <DetailHeader
        name="The Channel"
        videos={undefined}
        onSync={noop}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    expect(screen.queryByText(/videos/)).not.toBeInTheDocument()
  })

  it('keeps every action, Sync included, behind the ⋮ menu on mobile', async () => {
    const onSync = vi.fn(noop)
    render(
      <DetailHeader
        name="The Channel"
        videos={[]}
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

  it('offers mark-all-watched in its menu only when supported', async () => {
    const user = userEvent.setup()
    const { rerender } = render(
      <DetailHeader
        name="The Channel"
        videos={[]}
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
        videos={[]}
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
        videos={[]}
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
        videos={[]}
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

  it('marks a single unwatched video watched without asking', async () => {
    const user = userEvent.setup()
    const onMarkWatched = vi.fn(noop)
    render(
      <DetailHeader
        name="The Channel"
        videos={[]}
        unwatchedCount={1}
        onMarkWatched={onMarkWatched}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))

    await waitFor(() => expect(onMarkWatched).toHaveBeenCalledOnce())
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument()
  })

  it('keeps the mark-all-watched confirmation open with the error when marking fails', async () => {
    const user = userEvent.setup()
    const onMarkWatched = vi.fn(() => Promise.reject(new Error('mark failed')))
    render(
      <DetailHeader
        name="The Channel"
        videos={[]}
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

  it('disables mark-all-watched while it runs', async () => {
    let resolveMarkWatched = () => {}
    const onMarkWatched = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveMarkWatched = resolve
        }),
    )
    render(
      <DetailHeader
        name="The Channel"
        videos={[]}
        onSync={noop}
        onMarkWatched={onMarkWatched}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )
    const user = userEvent.setup()

    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))
    await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))

    expect(await screen.findByRole('menuitem', { name: 'Mark all watched' })).toHaveAttribute(
      'aria-disabled',
      'true',
    )
    resolveMarkWatched()
    await waitFor(() =>
      expect(screen.getByRole('menuitem', { name: 'Mark all watched' })).not.toHaveAttribute(
        'aria-disabled',
      ),
    )
    expect(onMarkWatched).toHaveBeenCalledOnce()
  })

  it('shows the summary prefix and menu items, and no Sync without onSync', async () => {
    render(
      <DetailHeader
        name="Chess"
        videos={[aVideo(), aVideo()]}
        unwatchedCount={1}
        summaryPrefix="3 sources"
        extraActions={[{ label: 'Manage sources', icon: null, onSelect: () => {} }]}
        onDelete={noop}
        deleteDescription="desc"
      />,
    )

    await userEvent.setup().click(screen.getByRole('button', { name: 'More actions for Chess' }))
    const items = (await screen.findAllByRole('menuitem')).map((item) => item.textContent)

    expect(screen.getByText('3 sources · 2 videos · 1 unwatched')).toBeInTheDocument()
    expect(items).toEqual(['Manage sources', 'Delete'])
  })

  describe('on desktop', () => {
    useDesktopViewport()

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
          videos={[]}
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
          videos={[]}
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
          videos={[]}
          onSync={noop}
          onDelete={noop}
          deleteDescription="desc"
        />,
      )

      await userEvent.setup().hover(screen.getByRole('button', { name: 'Sync' }))

      expect(await screen.findByRole('tooltip')).toHaveTextContent('Sync')
    })

    it('lays every action but mark-all-watched and delete out as an icon button, then a ⋮ menu offering both', async () => {
      const onEditRequest = vi.fn()
      render(
        <DetailHeader
          name="The Channel"
          videos={[]}
          onSync={noop}
          onMarkWatched={noop}
          onEditRequest={onEditRequest}
          onDelete={noop}
          deleteDescription="desc"
        />,
      )

      const buttons = screen.getAllByRole('button').map((button) => button.getAttribute('aria-label'))
      const user = userEvent.setup()
      await user.click(screen.getByRole('button', { name: 'Edit settings' }))
      await user.click(screen.getByRole('button', { name: 'More actions for The Channel' }))
      const items = (await screen.findAllByRole('menuitem')).map((item) => item.textContent)
      await user.click(screen.getByRole('menuitem', { name: 'Delete' }))

      expect(buttons).toEqual(['Sync', 'Edit settings', 'More actions for The Channel'])
      expect(items).toEqual(['Mark all watched', 'Delete'])
      expect(onEditRequest).toHaveBeenCalledOnce()
      expect(screen.getByRole('dialog', { name: 'Delete "The Channel"?' })).toBeVisible()
    })
  })
})
