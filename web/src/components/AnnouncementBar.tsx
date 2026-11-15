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

const LINK_CLASS =
  'font-semibold whitespace-nowrap text-primary no-underline underline-offset-2 hover:underline'

function linkProps(url: string) {
  return { href: url, target: '_blank', rel: 'noopener noreferrer' }
}

/**
 * Shows the newest announcement this browser hasn't dismissed: a slim one-line
 * strip above the header from the small breakpoint up, a compact card below
 * the header on phones (where the shell places it). Renders nothing while loading, on any fetch error, or when every
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
      className="mx-4 mt-3 flex shrink-0 items-center gap-2 rounded-xl border border-border bg-card py-2 pr-1.5 pl-3.5 text-[0.8125rem] text-muted-foreground sm:m-0 sm:h-8 sm:justify-center sm:rounded-none sm:border-x-0 sm:border-t-0 sm:bg-muted sm:py-0 sm:pr-2 sm:pl-4"
    >
      {/* Wraps on phones, so the whole note shows; one line from `sm` up. */}
      <p className="min-w-0 flex-1 sm:flex-initial sm:truncate">
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
