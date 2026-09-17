import { cn } from '@/lib/utils'

const VARIANT_CLASSES = {
  live: 'bg-primary shadow-[0_0_6px_1px_var(--primary)]',
  idle: 'bg-muted-foreground/40',
  warning: 'bg-amber-400 shadow-[0_0_6px_1px_theme(colors.amber.400)]',
  danger: 'bg-destructive shadow-[0_0_6px_1px_var(--destructive)]',
}

/**
 * The app's signature signal light: a small glowing dot used everywhere
 * something is "live" — an in-progress download, a running task, the
 * header's system-online indicator. `pulse` drives the animation and is
 * automatically disabled under prefers-reduced-motion.
 */
export function Beacon({ variant = 'live', pulse = true, label, className }) {
  return (
    <span
      role={label ? 'img' : undefined}
      aria-label={label}
      title={label}
      className={cn(
        'relative inline-flex size-2 shrink-0 rounded-full',
        VARIANT_CLASSES[variant] ?? VARIANT_CLASSES.live,
        pulse && 'animate-beacon-pulse motion-reduce:animate-none',
        className,
      )}
    />
  )
}
