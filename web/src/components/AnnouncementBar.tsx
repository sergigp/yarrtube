import { useState } from 'react'
import { X } from 'lucide-react'
import { useAnnouncements } from '@/api/queries'
import { Button } from '@/components/ui/button'
import {
  firstUnread,
  readDismissedAnnouncements,
  writeDismissedAnnouncements,
} from '@/lib/announcements'
import { parseAnnouncementText } from '@/lib/announcementText'

const LINK_CLASS = 'font-medium whitespace-nowrap underline underline-offset-2'

function linkProps(url: string) {
  return { href: url, target: '_blank', rel: 'noopener noreferrer' }
}

/**
 * Shows the newest announcement this browser hasn't dismissed, above the
 * header. Renders nothing while loading, on any fetch error, or when every
 * announcement has been read.
 */
export function AnnouncementBar() {
  const { data } = useAnnouncements()
  const [dismissed, setDismissed] = useState<ReadonlySet<string>>(
    () => new Set(readDismissedAnnouncements()),
  )
  const announcement = data && firstUnread(data, dismissed)
  if (!announcement) {
    return null
  }

  const segments = parseAnnouncementText(announcement.text)

  const dismiss = () => {
    const next = new Set(dismissed).add(announcement.id)
    setDismissed(next)
    writeDismissedAnnouncements([...next])
  }

  return (
    <div
      role="region"
      aria-label="Announcement"
      className="flex shrink-0 items-center gap-3 border-b border-border bg-muted px-4 py-1.5 text-sm sm:px-6 sm:py-2"
    >
      {/* Wraps rather than truncates, so the whole note shows on phones too. */}
      <p className="min-w-0 flex-1">
        {segments.map((segment, index) =>
          segment.kind === 'link' ? (
            <a key={index} {...linkProps(segment.url)} className={LINK_CLASS}>
              {segment.label}
            </a>
          ) : (
            <span key={index}>{segment.text}</span>
          ),
        )}
      </p>
      <Button
        variant="ghost"
        size="icon"
        className="size-7 shrink-0"
        aria-label="Dismiss announcement"
        onClick={dismiss}
      >
        <X className="size-4" />
      </Button>
    </div>
  )
}
