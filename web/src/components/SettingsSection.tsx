import type { ReactNode } from 'react'

export function SettingsSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <fieldset className="flex flex-col gap-4">
      <legend className="mb-4 font-heading text-lg font-semibold text-foreground">{title}</legend>
      {children}
    </fieldset>
  )
}
