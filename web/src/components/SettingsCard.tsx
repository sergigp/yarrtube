import type { ReactNode } from 'react'

/** One group of settings: a heading, an optional description and action, and its fields. */
export function SettingsCard({
  title,
  description,
  action,
  children,
}: {
  title: string
  description?: ReactNode
  action?: ReactNode
  children?: ReactNode
}) {
  return (
    <fieldset className="flex flex-col gap-4">
      <legend className="mb-4 font-heading text-lg font-semibold text-foreground">{title}</legend>
      {description}
      {action}
      {children}
    </fieldset>
  )
}
