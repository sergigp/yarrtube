import type { ReactNode } from 'react'

/** "Prompt": a disclosure saying whether the prompt is the default one. */
export function PromptDisclosure({ children }: { isDefault: boolean; children: ReactNode }) {
  return <>{children}</>
}
