import { describe, expect, it } from 'vitest'
import { baselineOverrides, lint, lintSource, readConfig, withoutOverride } from './oxlint'

// Each lint rule that enforces a convention, proven against the real
// .oxlintrc.json: red on a violation, green on code that follows it.
describe('oxlint rules', () => {
  describe('baselines', () => {
    it('only list files that still break the rules they are exempt from', () => {
      const config = readConfig()
      const stale = baselineOverrides(config).flatMap((override) => {
        const found = lint(override.files, withoutOverride(config, override))
        return override.files.flatMap((file) =>
          Object.keys(override.rules)
            .filter((rule) => !found.some((d) => d.file === file && d.rule.endsWith(`(${rule.split('/').pop()})`)))
            .map((rule) => `${file} no longer breaks ${rule}: delete it from its baseline in .oxlintrc.json`),
        )
      })

      expect(stale).toEqual([])
    })
  })
})
