/** The test files with no source next to them: `foo.test.ts(x)` needs `foo.ts(x)`. */
export function testsWithoutSource(paths: string[]): string[] {
  const existing = new Set(paths)
  return paths
    .filter((path) => /\.test\.tsx?$/.test(path))
    .filter((path) => {
      const base = path.replace(/\.test\.tsx?$/, '')
      return !existing.has(`${base}.ts`) && !existing.has(`${base}.tsx`)
    })
    .sort()
}
