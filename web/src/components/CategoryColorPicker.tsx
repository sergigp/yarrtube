import { useId } from 'react'
import { PALETTE } from '@/lib/categoryColors'
import { cn } from '@/lib/utils'

interface CategoryColorPickerProps {
  value: string
  onChange: (hex: string) => void
}

/** A round color swatch that is a radio option named after its color. */
function Swatch({
  name,
  group,
  color,
  checked,
  onSelect,
}: {
  name: string
  group: string
  color: string
  checked: boolean
  onSelect: () => void
}) {
  return (
    <label
      className={cn(
        'relative flex size-7 cursor-pointer items-center justify-center rounded-full border border-black/10 transition-shadow',
        'has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring has-[:focus-visible]:ring-offset-2',
        checked && 'ring-2 ring-foreground ring-offset-2',
      )}
      style={{ background: color }}
      title={name}
    >
      <input
        type="radio"
        name={group}
        aria-label={name}
        checked={checked}
        onChange={onSelect}
        className="sr-only"
      />
    </label>
  )
}

/** The palette swatches plus a custom pastel hue, as one radio group. */
export function CategoryColorPicker({ value, onChange }: CategoryColorPickerProps) {
  const group = useId()
  return (
    <div role="radiogroup" aria-label="Color" className="flex flex-wrap gap-2">
      {PALETTE.map((color) => (
        <Swatch
          key={color.hex}
          name={color.name}
          group={group}
          color={color.hex}
          checked={value === color.hex}
          onSelect={() => onChange(color.hex)}
        />
      ))}
    </div>
  )
}
