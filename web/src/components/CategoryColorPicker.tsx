import { useId, useState } from 'react'
import { PALETTE, hueOf, solidFromHue } from '@/lib/categoryColors'
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
        className="absolute inset-0 cursor-pointer appearance-none rounded-full opacity-0"
      />
    </label>
  )
}

const HUE_STOPS = [0, 60, 120, 180, 240, 300, 360].map((hue) => solidFromHue(hue)).join(', ')

/** Every solid hue around a circle, the Custom swatch's face. */
const CUSTOM_FACE = `conic-gradient(${HUE_STOPS})`

/** Every solid hue left to right, the hue slider's track. */
const HUE_TRACK = `linear-gradient(to right, ${HUE_STOPS})`

/**
 * The palette swatches plus "Custom", as one radio group. Custom shows a hue
 * slider whose colors keep the palette's solid lightness and chroma.
 */
export function CategoryColorPicker({ value, onChange }: CategoryColorPickerProps) {
  const group = useId()
  const custom = !PALETTE.some((color) => color.hex === value)
  // Held apart from `value` so the slider stays where it was put, even though
  // a hex only approximates its hue.
  const [hue, setHue] = useState(() => Math.round(hueOf(value)))
  const chooseHue = (next: number) => {
    setHue(next)
    onChange(solidFromHue(next))
  }

  return (
    <div className="flex flex-col gap-3">
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
        <Swatch
          name="Custom"
          group={group}
          color={custom ? value : CUSTOM_FACE}
          checked={custom}
          onSelect={() => chooseHue(hue)}
        />
      </div>
      {custom && (
        <input
          type="range"
          aria-label="Hue"
          min={0}
          max={359}
          value={hue}
          onChange={(event) => chooseHue(Number(event.target.value))}
          className="h-2 w-full cursor-pointer appearance-none rounded-full"
          style={{ background: HUE_TRACK }}
        />
      )}
    </div>
  )
}
