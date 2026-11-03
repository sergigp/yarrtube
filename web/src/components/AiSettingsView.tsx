import { useState, type ReactNode } from 'react'
import { Check, ChevronRight } from 'lucide-react'
import { useCheckOllama, useOllama, usePreferences, useSavePreferences } from '@/api/queries'
import type { OllamaServer, Preferences, PreferencesPatch } from '@/api/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { TooltipProvider } from '@/components/ui/tooltip'
import { PreferenceHelp } from '@/components/PreferenceHelp'
import { PromptHelp } from '@/components/PromptHelp'
import {
  changedFields,
  formFrom,
  hasInvalidNumber,
  modelOptions,
  type AiSettingsForm,
} from '@/lib/aiSettingsForm'
import { errorMessage } from '@/lib/errorMessage'
import { useSavedFlash } from '@/hooks/useSavedFlash'
import { cn } from '@/lib/utils'
import { VIEW_WIDTH } from '@/lib/viewWidth'

/** The AI page: the Ollama server and how video summaries are generated. */
export function AiSettingsView() {
  const { data: preferences, error } = usePreferences()

  return (
    <TooltipProvider>
      <div className={cn(VIEW_WIDTH, 'flex flex-col gap-8')}>
        <h1 className="font-heading text-xl font-semibold text-foreground">AI</h1>
        {error && (
          <p className="text-sm text-destructive">Failed to load preferences: {error.message}</p>
        )}
        {!error && !preferences && (
          <p className="text-sm text-muted-foreground">Loading preferences…</p>
        )}
        {preferences && <AiSettings saved={preferences} />}
      </div>
    </TooltipProvider>
  )
}

/** The form, filled once from the saved preferences. */
function AiSettings({ saved }: { saved: Preferences }) {
  const [form, setForm] = useState<AiSettingsForm>(() => formFrom(saved))
  const set = <K extends keyof AiSettingsForm>(key: K, value: AiSettingsForm[K]) =>
    setForm((current) => ({ ...current, [key]: value }))
  const summariesDisabled = !form.ai_enabled
  // Always shown, so turning summaries on doesn't reshape the page.
  const modelDisabled = summariesDisabled || !form.summaries_enabled
  // Context length, temperature, thinking and the prompt: their defaults come
  // from the evals, so they stay out of the way until asked for.
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const savePreferences = useSavePreferences()
  const savedFlash = useSavedFlash()
  const [saving, setSaving] = useState(false)
  const [saveError, setSaveError] = useState<string | null>(null)

  const checkOllama = useCheckOllama()
  const [connection, setConnection] = useState<ConnectionCheck>(NO_CHECK)
  // The server whose models "Model" lists: the saved one, or the last one
  // tested successfully, whichever came last.
  const [modelsUrl, setModelsUrl] = useState(saved.ollama_url)
  const { data: server } = useOllama(modelsUrl)
  // Known only when the server lists the model; unknown, the switch shows.
  const cannotThink =
    server?.models.find((model) => model.name === form.summaries_model)?.thinking === false
  const patch = changedFields(saved, form, { thinking: !cannotThink })
  const testConnection = async () => {
    const url = form.ollama_url.trim()
    setConnection({ ...NO_CHECK, checking: true })
    try {
      const tested = await checkOllama(url)
      setModelsUrl(url)
      setConnection({ ...NO_CHECK, message: connectedMessage(tested) })
    } catch (err) {
      setConnection({ ...NO_CHECK, failed: true, message: errorMessage(err) })
    }
  }

  /** Saves `patch`, showing "Saved" or why it failed. */
  const submit = async (patch: PreferencesPatch): Promise<Preferences | null> => {
    setSaveError(null)
    savedFlash.clear()
    setSaving(true)
    try {
      const updated = await savePreferences(patch)
      savedFlash.flash()
      if (patch.ollama_url !== undefined) {
        // "Model" lists the saved server's models.
        setModelsUrl(updated.ollama_url)
      }
      return updated
    } catch (err) {
      setSaveError(errorMessage(err))
      return null
    } finally {
      setSaving(false)
    }
  }

  const save = () => {
    if (hasInvalidNumber(patch)) {
      setSaveError('Context length and temperature must be numbers')
      savedFlash.clear()
      return
    }
    void submit(patch)
  }

  const resetPrompt = async () => {
    const updated = await submit({ summaries_prompt: null })
    if (updated) {
      set('summaries_prompt', updated.summaries_prompt)
    }
  }

  return (
    <>
      <Section title="Ollama">
        <SwitchRow
          id="ai-enabled"
          label="Enable AI"
          checked={form.ai_enabled}
          onChange={(value) => set('ai_enabled', value)}
        />
        <Field id="ollama-url" label="Ollama URL">
          <div className="flex gap-2">
            <Input
              id="ollama-url"
              type="url"
              placeholder="http://192.168.1.10:11434"
              value={form.ollama_url}
              onChange={(event) => set('ollama_url', event.target.value)}
            />
            <Button
              variant="outline"
              disabled={form.ollama_url.trim() === '' || connection.checking}
              onClick={() => void testConnection()}
            >
              Test connection
            </Button>
          </div>
          {connection.message && (
            <p
              className={cn(
                'text-sm',
                connection.failed ? 'text-destructive' : 'text-muted-foreground',
              )}
            >
              {connection.message}
            </p>
          )}
        </Field>
      </Section>
      <Section title="Summaries">
        <SwitchRow
          id="summaries-enabled"
          label="Enabled"
          name="Summaries enabled"
          checked={form.summaries_enabled}
          disabled={summariesDisabled}
          onChange={(value) => set('summaries_enabled', value)}
        />
        <Field id="summaries-model" label="Model">
          {server ? (
            <select
              id="summaries-model"
              value={form.summaries_model}
              disabled={modelDisabled}
              onChange={(event) => set('summaries_model', event.target.value)}
              className="h-8 w-full rounded-lg border border-input bg-transparent px-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-input/30"
            >
              {modelOptions(server.models, form.summaries_model).map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          ) : (
            <Input
              id="summaries-model"
              value={form.summaries_model}
              disabled={modelDisabled}
              onChange={(event) => set('summaries_model', event.target.value)}
            />
          )}
        </Field>
        {form.summaries_enabled && (
          <>
            <button
              type="button"
              aria-expanded={advancedOpen}
              onClick={() => setAdvancedOpen((open) => !open)}
              className="flex items-center gap-1 self-start text-sm font-medium text-muted-foreground hover:text-foreground"
            >
              <ChevronRight
                className={cn('size-4 transition-transform', advancedOpen && 'rotate-90')}
                aria-hidden="true"
              />
              Advanced
            </button>
            {advancedOpen && (
              <>
                <Field id="summaries-context-length" label="Context length">
                  <Input
                    id="summaries-context-length"
                    type="number"
                    min={1024}
                    max={131072}
                    step={1024}
                    value={form.summaries_context_length}
                    disabled={summariesDisabled}
                    onChange={(event) => set('summaries_context_length', event.target.value)}
                  />
                </Field>
                <Field id="summaries-temperature" label="Temperature">
                  <Input
                    id="summaries-temperature"
                    type="number"
                    min={0}
                    max={2}
                    step={0.1}
                    value={form.summaries_temperature}
                    disabled={summariesDisabled}
                    onChange={(event) => set('summaries_temperature', event.target.value)}
                  />
                </Field>
                {cannotThink ? (
                  <p className="text-sm text-muted-foreground">
                    {form.summaries_model} doesn't support thinking
                  </p>
                ) : (
                  <SwitchRow
                    id="summaries-thinking"
                    label="Thinking"
                    help={THINKING_HELP}
                    checked={form.summaries_thinking}
                    disabled={summariesDisabled}
                    onChange={(value) => set('summaries_thinking', value)}
                  />
                )}
                <Field id="summaries-prompt" label="Prompt" help={<PromptHelp />}>
                  <textarea
                    id="summaries-prompt"
                    rows={20}
                    spellCheck={false}
                    value={form.summaries_prompt}
                    disabled={summariesDisabled}
                    onChange={(event) => set('summaries_prompt', event.target.value)}
                    className="w-full rounded-lg border border-input bg-transparent px-2.5 py-2 font-mono text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-input/30"
                  />
                  <div className="flex items-center justify-between gap-4">
                    {saved.summaries_prompt_is_default && form.summaries_prompt === saved.summaries_prompt ? (
                      <p className="text-sm text-muted-foreground">Using the default prompt</p>
                    ) : (
                      <span />
                    )}
                    {!saved.summaries_prompt_is_default && (
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={saving || summariesDisabled}
                        onClick={() => void resetPrompt()}
                      >
                        Reset to default
                      </Button>
                    )}
                  </div>
                </Field>
              </>
            )}
          </>
        )}
      </Section>
      <div className="flex items-center justify-end gap-4">
        {saveError && (
          <p role="alert" className="mr-auto text-sm text-destructive">
            {saveError}
          </p>
        )}
        <p role="status" className="flex items-center gap-1 text-sm text-muted-foreground">
          {savedFlash.shown && (
            <>
              <Check className="size-4" aria-hidden="true" />
              Saved
            </>
          )}
        </p>
        <Button disabled={saving || Object.keys(patch).length === 0} onClick={save}>
          Save
        </Button>
      </div>
    </>
  )
}

const THINKING_HELP =
  'Lets the model reason before answering: slower, but more accurate. Some models, such as qwen3:4b, think even when it is off.'

interface ConnectionCheck {
  checking: boolean
  failed: boolean
  message: string | null
}

const NO_CHECK: ConnectionCheck = { checking: false, failed: false, message: null }

function connectedMessage(server: OllamaServer): string {
  const count = server.models.length
  return `Connected to Ollama ${server.version}, ${count} ${count === 1 ? 'model' : 'models'}`
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <fieldset className="flex flex-col gap-4">
      <legend className="mb-4 font-heading text-lg font-semibold text-foreground">{title}</legend>
      {children}
    </fieldset>
  )
}

function Field({
  id,
  label,
  help,
  children,
}: {
  id: string
  label: string
  help?: ReactNode
  children: ReactNode
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center gap-1.5">
        <Label htmlFor={id}>{label}</Label>
        {help}
      </div>
      {children}
    </div>
  )
}

function SwitchRow({
  id,
  label,
  name,
  help,
  checked,
  disabled,
  onChange,
}: {
  id: string
  label: string
  /** The switch's accessible name, when the label alone is ambiguous. */
  name?: string
  help?: string
  checked: boolean
  disabled?: boolean
  onChange: (value: boolean) => void
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div className="flex items-center gap-1.5">
        <Label htmlFor={id}>{label}</Label>
        {help && <PreferenceHelp label={label} help={help} />}
      </div>
      <Switch
        id={id}
        aria-label={name}
        checked={checked}
        disabled={disabled}
        onCheckedChange={onChange}
      />
    </div>
  )
}
