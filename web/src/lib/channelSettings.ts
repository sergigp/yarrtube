import type { UpdateChannelRequest } from '@/api/client'
import type { ChannelListItem } from '@/api/types'
import { intervalMinutes, type IntervalForm } from './reconcileInterval'

/** The edit channel dialog's fields, as entered. */
export interface ChannelSettingsForm {
  quality: string
  video_limit: string
  interval: IntervalForm
}

/** The settings the form changes from the channel's current ones; `{}` when none. */
export function channelSettingsChanges(
  current: Pick<ChannelListItem, 'quality' | 'video_limit' | 'reconcile_interval_minutes'>,
  form: ChannelSettingsForm,
): UpdateChannelRequest {
  const videoLimit = Number(form.video_limit)
  const interval = intervalMinutes(form.interval)
  return {
    ...(form.quality !== current.quality && { quality: form.quality }),
    ...(videoLimit !== current.video_limit && { video_limit: videoLimit }),
    ...(interval !== null &&
      interval !== current.reconcile_interval_minutes && { reconcile_interval_minutes: interval }),
  }
}

/** Whether the entered video limit is below the channel's current one. */
export function lowersVideoLimit(currentLimit: number, entered: string): boolean {
  return entered.trim() !== '' && Number(entered) < currentLimit
}
