import type { ReactNode } from 'react'
import {
  type VideoGridLayout,
  videoGridItemClassName,
  videoGridListClassName,
} from '@/lib/videoGridLayout'
import { Skeleton } from './ui/skeleton'

/** A video card's placeholder: thumbnail, two title lines and a meta line. */
export function VideoCardSkeleton() {
  return (
    <div className="flex flex-col gap-2">
      <Skeleton className="aspect-video w-full rounded-lg" />
      <div className="flex items-start gap-2">
        <Skeleton className="mt-0.5 size-6 shrink-0 rounded-full" />
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          <Skeleton className="h-3.5 w-full" />
          <Skeleton className="h-3.5 w-2/3" />
          <Skeleton className="h-3 w-1/2" />
        </div>
      </div>
    </div>
  )
}

/** A video list row's placeholder: thumbnail beside two title lines. */
export function VideoRowSkeleton() {
  return (
    <div className="flex items-start gap-3 py-2.5 pl-2">
      <Skeleton className="aspect-video w-24 shrink-0 rounded-md" />
      <div className="flex min-w-0 flex-1 flex-col gap-1.5">
        <Skeleton className="h-3.5 w-full" />
        <Skeleton className="h-3.5 w-1/2" />
      </div>
    </div>
  )
}

/** `count` placeholder cards laid out as `layout`, announced as loading. */
export function VideoGridSkeleton({
  count,
  layout = 'grid',
}: {
  count: number
  layout?: VideoGridLayout
}) {
  return (
    <Busy>
      <ul className={videoGridListClassName(layout)}>
        {Array.from({ length: count }, (_, index) => (
          <li key={index} className={videoGridItemClassName(layout, index)}>
            <VideoCardSkeleton />
          </li>
        ))}
      </ul>
    </Busy>
  )
}

/** `count` placeholder video list rows, announced as loading. */
export function VideoRowsSkeleton({ count }: { count: number }) {
  return (
    <Busy>
      <ul className="flex flex-col divide-y divide-border">
        {Array.from({ length: count }, (_, index) => (
          <li key={index}>
            <VideoRowSkeleton />
          </li>
        ))}
      </ul>
    </Busy>
  )
}

/** A loading region: marked busy, announced as "Loading…" to assistive technology. */
export function Busy({ children }: { children: ReactNode }) {
  return (
    <div aria-busy="true">
      <span role="status" className="sr-only">
        Loading…
      </span>
      {children}
    </div>
  )
}
