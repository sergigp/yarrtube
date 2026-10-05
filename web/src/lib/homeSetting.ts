/** Describes changing a playlist's home setting to `excluded`, e.g. for a failure alert. */
export function homeSettingAction(excluded: boolean, name: string): string {
  return excluded ? `exclude "${name}" from home` : `include "${name}" in home`
}
