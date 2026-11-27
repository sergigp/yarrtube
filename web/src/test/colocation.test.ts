/// <reference types="node" />
import { readdirSync } from 'node:fs'
import { join, relative } from 'node:path'
import { describe, expect, it } from 'vitest'
import { testsWithoutSource } from './colocation'
import { WEB_ROOT } from './oxlint'

// Test files that had no source next to them when the rule arrived. This
// baseline only shrinks: colocate one, then delete its line.
const BASELINE = ['src/AppShell.test.tsx']

const MESSAGE =
  'Colocate each test with what it tests: `foo.ts` → `foo.test.ts`, in the same folder. ' +
  'Convention: CLAUDE.md (Frontend testing (web/))'

// oxlint.test.ts writes its fixtures in src/__lint_fixture_*/ while it runs.
function sourceFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true })
    .filter((entry) => !entry.name.startsWith('__lint_fixture_'))
    .flatMap((entry) =>
      entry.isDirectory()
      ? sourceFiles(join(dir, entry.name))
      : [relative(WEB_ROOT, join(dir, entry.name))],
  )
}

describe('testsWithoutSource', () => {
  it('holds on the repository, outside its baseline', () => {
    expect(testsWithoutSource(sourceFiles(join(WEB_ROOT, 'src'))), MESSAGE).toEqual(BASELINE)
  })

  it('reports a test with no source next to it', () => {
    expect(
      testsWithoutSource(['src/lib/slugify.ts', 'src/lib/slugify.test.ts', 'src/components/Orphan.test.tsx']),
    ).toEqual(['src/components/Orphan.test.tsx'])
  })

  it('accepts tests next to a .ts or .tsx source', () => {
    expect(
      testsWithoutSource([
        'src/lib/slugify.ts',
        'src/lib/slugify.test.ts',
        'src/components/Home.tsx',
        'src/components/Home.test.tsx',
        'src/hooks/useLookup.ts',
        'src/hooks/useLookup.test.tsx',
      ]),
    ).toEqual([])
  })
})
