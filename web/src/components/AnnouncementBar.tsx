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
import { cn } from '@/lib/utils'

const LINK_CLASS = 'font-medium underline underline-offset-2'

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
  const links = segments.flatMap((segment) => (segment.kind === 'link' ? [segment] : []))

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
      {/* Below `sm`: one line, the text truncated and its links after it. */}
      <p className="flex min-w-0 flex-1 items-center gap-2">
        <span className="min-w-0 max-sm:truncate">
          {segments.map((segment, index) =>
            segment.kind === 'link' ? (
              <a
                key={index}
                {...linkProps(segment.url)}
                className={cn(LINK_CLASS, 'max-sm:hidden')}
              >
                {segment.label}
              </a>
            ) : (
              <span key={index}>{segment.text}</span>
            ),
          )}
        </span>
        {links.length > 0 && (
          <span className="flex shrink-0 gap-2 sm:hidden">
            {links.map((link, index) => (
              <a key={index} {...linkProps(link.url)} className={LINK_CLASS}>
                {link.label}
              </a>
            ))}
          </span>
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
