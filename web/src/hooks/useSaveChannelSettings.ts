import { useCallback } from 'react'
import { useLibraryAction, useUpdateChannelSettings } from '@/api/queries'
import { reconcileChannel, type UpdateChannelRequest } from '@/api/client'
import type { ChannelListItem } from '@/api/types'
import { errorMessage } from '@/lib/errorMessage'

/**
 * Saves a channel's changed settings, the edit channel dialog's `onSave`.
 * A new limit applies at the next sync, so it runs one now rather than leave
 * the channel short of (or past) its limit until the recurring pass; a
 * failed sync is alerted, the settings staying saved.
 */
export function useSaveChannelSettings(): (
  channel: ChannelListItem,
  changes: UpdateChannelRequest,
) => Promise<void> {
  const updateChannelSettings = useUpdateChannelSettings()
  const refreshing = useLibraryAction()
  return useCallback(
    async (channel: ChannelListItem, changes: UpdateChannelRequest) => {
      await updateChannelSettings(channel.id, changes)
      if (changes.video_limit !== undefined) {
        refreshing(() => reconcileChannel(channel.id))().catch((err: unknown) =>
          window.alert(
            `Saved the settings of "${channel.name}", but failed to sync it: ${errorMessage(err)}`,
          ),
        )
      }
    },
    [updateChannelSettings, refreshing],
  )
}
