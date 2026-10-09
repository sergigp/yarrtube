import type { UpdateChannelRequest } from '@/api/client'
import type { ChannelListItem } from '@/api/types'

/** The edit channel dialog's fields, as entered. */
export interface ChannelSettingsForm {
  quality: string
  video_limit: string
}

/** The settings the form changes from the channel's current ones; `{}` when none. */
export function channelSettingsChanges(
  current: Pick<ChannelListItem, 'quality' | 'video_limit'>,
  form: ChannelSettingsForm,
): UpdateChannelRequest {
  const videoLimit = Number(form.video_limit)
  return {
    ...(form.quality !== current.quality && { quality: form.quality }),
    ...(videoLimit !== current.video_limit && { video_limit: videoLimit }),
  }
}

/** Whether the entered video limit is below the channel's current one. */
export function lowersVideoLimit(currentLimit: number, entered: string): boolean {
  return entered.trim() !== '' && Number(entered) < currentLimit
}
