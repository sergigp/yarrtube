import { cn } from '@/lib/utils'
import { VIEW_WIDTH } from '@/lib/viewWidth'

/** The AI page: the Ollama server and how video summaries are generated. */
export function AiSettingsView() {
  return (
    <div className={cn(VIEW_WIDTH, 'flex flex-col gap-8')}>
      <h1 className="font-heading text-xl font-semibold text-foreground">AI</h1>
    </div>
  )
}
