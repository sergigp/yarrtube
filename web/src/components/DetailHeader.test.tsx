import { describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { DetailHeader } from './DetailHeader'
import { aVideo } from '@/test/helpers'

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

    await user.click(screen.getByRole('button', { name: 'Actions for The Channel' }))
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

    await user.click(screen.getByRole('button', { name: 'Actions for The Channel' }))
    expect(await screen.findByRole('menuitem', { name: 'Delete' })).toBeInTheDocument()
    expect(screen.queryByRole('menuitem', { name: 'Mark all watched' })).not.toBeInTheDocument()
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

    await user.click(screen.getByRole('button', { name: 'Actions for The Channel' }))
    await user.click(await screen.findByRole('menuitem', { name: 'Mark all watched' }))
    await user.click(screen.getByRole('button', { name: 'Actions for The Channel' }))

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
})
