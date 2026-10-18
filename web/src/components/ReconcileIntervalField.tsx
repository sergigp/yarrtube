import { MAX_INTERVAL_MINUTES, type IntervalForm, type IntervalUnit } from '@/lib/reconcileInterval'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

interface ReconcileIntervalFieldProps {
  id: string
  value: IntervalForm
  onChange: (value: IntervalForm) => void
}

export function ReconcileIntervalField({ id, value, onChange }: ReconcileIntervalFieldProps) {
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>Check for new videos every</Label>
      <div className="flex gap-2">
        <Input
          id={id}
          type="number"
          min="1"
          max={value.unit === 'hours' ? MAX_INTERVAL_MINUTES / 60 : MAX_INTERVAL_MINUTES}
          step="1"
          value={value.amount}
          onChange={(event) => onChange({ ...value, amount: event.target.value })}
          required
        />
        <Select
          value={value.unit}
          onValueChange={(unit) => onChange({ ...value, unit: unit as IntervalUnit })}
        >
          <SelectTrigger aria-label="Interval unit" className="w-32">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="minutes">minutes</SelectItem>
            <SelectItem value="hours">hours</SelectItem>
          </SelectContent>
        </Select>
      </div>
    </div>
  )
}
