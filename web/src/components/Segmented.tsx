/** A choice of one of a few values, shown as joined segments. */
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
    <div role="radiogroup" aria-label={name} className="flex flex-wrap gap-x-6 gap-y-2">
      {choices.map((choice) => (
        <label key={choice.value} className="flex items-center gap-2">
          <input
            type="radio"
            name={name}
            value={choice.value}
            checked={value === choice.value}
            disabled={disabled}
            onChange={() => onChange(choice.value)}
          />
          {choice.label}
        </label>
      ))}
    </div>
  )
}
