/** A variable a prompt section can use, and what it is replaced by. */
export interface PromptVariable {
  name: string
  description: string
}

/** A section of the summary prompt, as the daemon accepts it. */
export interface PromptSectionGuide {
  heading: string
  purpose: string
  variables: PromptVariable[]
}

/** Every section of the summary prompt, in document order. */
export const PROMPT_SECTIONS: PromptSectionGuide[] = []
