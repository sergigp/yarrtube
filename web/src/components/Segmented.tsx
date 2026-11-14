import { cn } from '@/lib/utils'

/**
 * A choice of one of a few values, shown as joined segments: radio inputs
 * underneath, so it is a radiogroup of radios to assistive technology.
 */
export function Segmented<T extends string>({
  name,
  choices,
  value,
  disabled,
  onChange,
}: {
  name: string
  choices: { value: T; label: string }[]
  value: T
  disabled?: boolean
  onChange: (value: T) => void
}) {
  return (
    <div
      role="radiogroup"
      aria-label={name}
      aria-disabled={disabled || undefined}
      className={cn(
        'inline-flex self-start rounded-lg border border-border bg-muted p-0.5',
        disabled && 'opacity-50',
      )}
    >
      {choices.map((choice) => {
        const selected = value === choice.value
        return (
          <label
            key={choice.value}
            className={cn(
              'relative rounded-md px-3 py-1.5 text-sm transition-colors has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring',
              selected
                ? 'bg-background font-semibold text-foreground shadow-sm'
                : 'text-muted-foreground',
              disabled ? 'cursor-not-allowed' : 'cursor-pointer hover:text-foreground',
            )}
          >
            <input
              type="radio"
              name={name}
              value={choice.value}
              checked={selected}
              disabled={disabled}
              onChange={() => onChange(choice.value)}
              className="sr-only"
            />
            {choice.label}
          </label>
        )
      })}
    </div>
  )
}
