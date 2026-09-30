import type { SaveLocation } from '@/hooks/useSaveLocation'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'

/** The folder-name override under "Advanced options". */
export function FolderNameField({ location, id }: { location: SaveLocation; id: string }) {
  return (
    <div className="flex min-w-0 flex-col gap-1.5">
      <Label htmlFor={id}>Folder name</Label>
      <Input
        id={id}
        type="text"
        value={location.folderName}
        onChange={(event) => location.setFolderName(event.target.value)}
        aria-invalid={Boolean(location.folderNameError)}
      />
      {location.folderNameError && (
        <p className="text-sm text-destructive">{location.folderNameError}</p>
      )}
    </div>
  )
}
