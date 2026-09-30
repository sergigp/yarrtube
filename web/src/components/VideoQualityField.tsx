import { Info } from 'lucide-react'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'

interface VideoQualityFieldProps {
  id: string
  value: string
  /** Event-shaped so the dialogs' shared `setField` handler fits unchanged. */
  onChange: (event: { target: { value: string } }) => void
}

export function VideoQualityField({ id, value, onChange }: VideoQualityFieldProps) {
  return (
    <div className="flex flex-col gap-1.5">
      <Label htmlFor={id}>
        Video quality
        <TooltipProvider>
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                className="inline-flex size-4 items-center justify-center rounded-full text-muted-foreground"
                aria-label="About video quality"
              >
                <Info className="size-3.5" />
              </button>
            </TooltipTrigger>
            <TooltipContent>
              Controls the resolution videos are downloaded at. A lower resolution reduces
              storage use.
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </Label>
      <Select value={value} onValueChange={(next) => onChange({ target: { value: next } })}>
        <SelectTrigger id={id} className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="high">High</SelectItem>
          <SelectItem value="mid">Mid</SelectItem>
          <SelectItem value="low">Low</SelectItem>
        </SelectContent>
      </Select>
    </div>
  )
}
