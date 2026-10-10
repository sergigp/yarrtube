/**
 * The folder holding a video's file, as the detail pane shows it: the
 * source's storage path joined with the file's own folder (its season
 * folder), never the file name itself; the storage path alone when the
 * video has no file yet.
 */
export function storageLocation(basePath: string, filename: string | null): string {
  const separator = filename?.lastIndexOf('/') ?? -1
  return filename && separator >= 0 ? `${basePath}/${filename.slice(0, separator)}` : basePath
}
