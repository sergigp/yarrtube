/// <reference types="node" />
// Runs the repository's oxlint config from the lint rules' own tests
// (oxlint.test.ts): on fixtures written for a test, and on the files a
// baseline exempts, to prove each rule still fires.
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'

export const WEB_ROOT = join(dirname(fileURLToPath(import.meta.url)), '../..')
const OXLINT = join(WEB_ROOT, 'node_modules/.bin/oxlint')

export interface Override {
  files: string[]
  rules: Record<string, unknown>
}

export interface OxlintConfig {
  overrides?: Override[]
  [key: string]: unknown
}

export interface Diagnostic {
  file: string
  rule: string
}

/** The repository's `.oxlintrc.json`, its `//` comments dropped. */
export function readConfig(): OxlintConfig {
  const text = readFileSync(join(WEB_ROOT, '.oxlintrc.json'), 'utf8')
  return JSON.parse(
    text
      .split('\n')
      .filter((line) => !line.trim().startsWith('//'))
      .join('\n'),
  ) as OxlintConfig
}

/** Lints `files` (relative to web/) with `config`, written next to the real one. */
export function lint(files: string[], config: OxlintConfig = readConfig()): Diagnostic[] {
  const configPath = join(WEB_ROOT, `.oxlintrc.rule-test-${process.pid}-${Date.now()}.json`)
  writeFileSync(configPath, JSON.stringify(config))
  try {
    const run = spawnSync(OXLINT, ['-c', configPath, '--format', 'json', ...files], {
      cwd: WEB_ROOT,
      encoding: 'utf8',
    })
    const report = JSON.parse(run.stdout) as { diagnostics: { filename: string; code: string }[] }
    return report.diagnostics
      .map((d) => ({ file: d.filename, rule: d.code }))
      .sort((a, b) => `${a.file} ${a.rule}`.localeCompare(`${b.file} ${b.rule}`))
  } finally {
    rmSync(configPath)
  }
}

/** The rules the real config reports on `source`, saved as `src/<tmp>/<name>`. */
export function lintSource(name: string, source: string): string[] {
  const dir = mkdtempSync(join(WEB_ROOT, 'src', '__lint_fixture_'))
  try {
    const path = join(dir, name)
    mkdirSync(dirname(path), { recursive: true })
    writeFileSync(path, source)
    return lint([relative(WEB_ROOT, path)]).map((d) => d.rule)
  } finally {
    rmSync(dir, { recursive: true, force: true })
  }
}

/** The overrides that exempt a baseline: exact file paths, no globs. */
export function baselineOverrides(config: OxlintConfig): Override[] {
  return (config.overrides ?? []).filter((o) => o.files.every((file) => !file.includes('*')))
}

export function withoutOverride(config: OxlintConfig, override: Override): OxlintConfig {
  return { ...config, overrides: (config.overrides ?? []).filter((o) => o !== override) }
}
