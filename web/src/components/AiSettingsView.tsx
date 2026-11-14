import { SettingsCard } from '@/components/SettingsCard'
import { SaveBar } from '@/components/SaveBar'
import { PageHeader } from '@/components/PageHeader'
import { PromptDisclosure } from '@/components/PromptDisclosure'
import { Segmented } from '@/components/Segmented'
import { changedSettingNames } from '@/lib/aiChanges'
import { useState, type ReactNode } from 'react'
import { Link } from 'react-router-dom'
import { ChevronRight } from 'lucide-react'
import {
  useAnthropic,
  useCheckAnthropic,
  useCheckOllama,
  useOllama,
  usePreferences,
  useSavePreferences,
} from '@/api/queries'
import type {
  AiProvider,
  AnthropicServer,
  Effort,
  OllamaServer,
  Preferences,
  PreferencesPatch,
} from '@/api/types'
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

/** The AI page: the provider AI runs on, and how videos are summarized and categorized. */
export function AiSettingsView() {
  const { data: preferences, error } = usePreferences()

  return (
    <TooltipProvider>
      <div className={cn(VIEW_WIDTH, 'flex flex-col gap-6')}>
        {!preferences && <PageHeader title="AI" description={AI_DESCRIPTION} saved={false} />}
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

const AI_DESCRIPTION =
  'Summaries and automatic categorization, run on a local Ollama server or on Anthropic.'

/** The form, filled once from the saved preferences. */
function AiSettings({ saved }: { saved: Preferences }) {
  const [form, setForm] = useState<AiSettingsForm>(() => formFrom(saved))
  const set = <K extends keyof AiSettingsForm>(key: K, value: AiSettingsForm[K]) =>
    setForm((current) => ({ ...current, [key]: value }))
  const aiOff = !form.ai_enabled
  const onAnthropic = form.ai_provider === 'anthropic'
  // Saved before the key was removed: only Ollama can be picked now.
  const keyRemoved = !saved.anthropic_available && saved.ai_provider === 'anthropic'
  // Each provider keeps its own models: the fields edit the selected one's.
  const summariesModelKey = onAnthropic ? 'anthropic_summaries_model' : 'summaries_model'
  const categorizationModelKey = onAnthropic
    ? 'anthropic_categorization_model'
    : 'categorization_model'
  // Always shown, so turning summaries on doesn't reshape the page.
  const modelDisabled = aiOff || !form.summaries_enabled
  // Context length, temperature, thinking and the prompt: their defaults come
  // from the evals, so they stay out of the way until asked for.
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [categorizationAdvancedOpen, setCategorizationAdvancedOpen] = useState(false)
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
  // Anthropic's models, checked once Anthropic is saved or tested.
  const checkAnthropic = useCheckAnthropic()
  const [anthropicModelsWanted, setAnthropicModelsWanted] = useState(
    saved.ai_provider === 'anthropic',
  )
  const { data: anthropic } = useAnthropic(saved.anthropic_available && anthropicModelsWanted)
  const models = onAnthropic
    ? anthropic?.models.map((model) => model.name)
    : server?.models.map((model) => model.name)
  const missingModel = onAnthropic ? 'not available' : 'not installed'
  // Known only when the server lists the model; unknown, the switch shows.
  const modelCannotThink = (name: string) =>
    server?.models.find((model) => model.name === name)?.thinking === false
  const summariesCannotThink = modelCannotThink(form.summaries_model)
  const categorizationCannotThink = modelCannotThink(form.categorization_model)
  // While Auto is on, what the daemon sized it to (known once saved).
  const categorizationContextLength =
    form.categorization_context_length_auto && saved.categorization_context_length_is_auto
      ? String(saved.categorization_context_length)
      : form.categorization_context_length
  const setCategorizationAuto = (auto: boolean) =>
    setForm((current) => ({
      ...current,
      categorization_context_length_auto: auto,
      categorization_context_length: categorizationContextLength,
    }))
  const patch = changedFields(saved, form, {
    summariesThinking: !summariesCannotThink,
    categorizationThinking: !categorizationCannotThink,
  })
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

  const testAnthropic = async () => {
    setConnection({ ...NO_CHECK, checking: true })
    try {
      const tested = await checkAnthropic()
      setAnthropicModelsWanted(true)
      setConnection({ ...NO_CHECK, message: anthropicConnectedMessage(tested) })
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

  const discard = () => {
    setForm(formFrom(saved))
    setSaveError(null)
  }

  const resetPrompt = async () => {
    const updated = await submit({ summaries_prompt: null })
    if (updated) {
      set('summaries_prompt', updated.summaries_prompt)
    }
  }

  const resetCategorizationPrompt = async () => {
    const updated = await submit({ categorization_prompt: null })
    if (updated) {
      set('categorization_prompt', updated.categorization_prompt)
    }
  }

  return (
    <>
      <PageHeader
        title="AI"
        description={AI_DESCRIPTION}
        saved={savedFlash.shown}
        action={
          <Switch
            aria-label="AI features"
            className="mt-1.5"
            checked={form.ai_enabled}
            onCheckedChange={(value) => set('ai_enabled', value)}
          />
        }
      />
      <SettingsCard title="Provider">
        {keyRemoved && (
          <p className="text-sm text-destructive">
            Anthropic needs an API key. Set YARRTUBE_ANTHROPIC_API_KEY and restart yarrtube.
          </p>
        )}
        {(saved.anthropic_available || keyRemoved) && (
          <ProviderChoice
            providers={saved.anthropic_available ? PROVIDERS : OLLAMA_ONLY}
            value={form.ai_provider}
            onChange={(value) => set('ai_provider', value)}
          />
        )}
        {!onAnthropic && (
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
            <ConnectionMessage connection={connection} />
          </Field>
        )}
        {onAnthropic && (
          <div className="flex flex-col gap-1.5">
            <Button
              variant="outline"
              className="self-start"
              disabled={connection.checking}
              onClick={() => void testAnthropic()}
            >
              Test connection
            </Button>
            <ConnectionMessage connection={connection} />
          </div>
        )}
      </SettingsCard>
      <SettingsCard
        title="Summaries"
        description="Summarize videos from channels and playlists that have summaries on. The summary shows on the video's page."
        action={
          <Switch
            aria-label="Summaries"
            checked={form.summaries_enabled}
            disabled={aiOff}
            onCheckedChange={(value) => set('summaries_enabled', value)}
          />
        }
      >
        <ModelField
          id="summaries-model"
          models={models}
          missing={missingModel}
          value={form[summariesModelKey]}
          disabled={modelDisabled}
          onChange={(value) => set(summariesModelKey, value)}
        />
        {form.summaries_enabled && (
          <>
            {onAnthropic ? (
              <EffortField
                name="Summaries effort"
                value={form.anthropic_summaries_effort}
                disabled={aiOff}
                onChange={(value) => set('anthropic_summaries_effort', value)}
              />
            ) : (
              <>
                <AdvancedToggle open={advancedOpen} onToggle={setAdvancedOpen} />
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
                        disabled={aiOff}
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
                        disabled={aiOff}
                        onChange={(event) => set('summaries_temperature', event.target.value)}
                      />
                    </Field>
                    <ThinkingField
                      id="summaries-thinking"
                      model={form.summaries_model}
                      cannotThink={summariesCannotThink}
                      checked={form.summaries_thinking}
                      disabled={aiOff}
                      onChange={(value) => set('summaries_thinking', value)}
                    />
                  </>
                )}
              </>
            )}
            <PromptDisclosure
              isDefault={
                saved.summaries_prompt_is_default &&
                form.summaries_prompt === saved.summaries_prompt
              }
            >
              <PromptField
                id="summaries-prompt"
                help={<PromptHelp />}
                rows={20}
                value={form.summaries_prompt}
                disabled={aiOff}
                canReset={!saved.summaries_prompt_is_default}
                resetDisabled={saving || aiOff}
                onChange={(value) => set('summaries_prompt', value)}
                onReset={() => void resetPrompt()}
              />
            </PromptDisclosure>
          </>
        )}
      </SettingsCard>
      {saved.categories_enabled && (
        <SettingsCard
          title="Categorization"
          description={
            <>
              Suggest categories for new videos from channels and playlists that have
              categorization on.{' '}
              <Link to="/settings/general" className="text-primary underline-offset-4 hover:underline">
                Uses your categories
              </Link>
              .
            </>
          }
          action={
            <Switch
              aria-label="Categorization"
              checked={form.categorization_enabled}
              disabled={aiOff}
              onCheckedChange={(value) => set('categorization_enabled', value)}
            />
          }
        >
          <ModelField
            id="categorization-model"
            name="Categorization model"
            models={models}
            missing={missingModel}
            value={form[categorizationModelKey]}
            disabled={aiOff || !form.categorization_enabled}
            onChange={(value) => set(categorizationModelKey, value)}
          />
          {form.categorization_enabled && (
            <>
              {onAnthropic ? (
                <EffortField
                  name="Categorization effort"
                  value={form.anthropic_categorization_effort}
                  disabled={aiOff}
                  onChange={(value) => set('anthropic_categorization_effort', value)}
                />
              ) : (
                <>
                  <AdvancedToggle
                    name="Advanced categorization settings"
                    open={categorizationAdvancedOpen}
                    onToggle={setCategorizationAdvancedOpen}
                  />
                  {categorizationAdvancedOpen && (
                    <>
                      <Field id="categorization-context-length" label="Context length">
                        <Input
                          id="categorization-context-length"
                          aria-label="Categorization context length"
                          type="number"
                          min={1024}
                          max={131072}
                          step={1024}
                          value={categorizationContextLength}
                          readOnly={form.categorization_context_length_auto}
                          disabled={aiOff}
                          onChange={(event) =>
                            set('categorization_context_length', event.target.value)
                          }
                        />
                        <p className="text-sm text-muted-foreground">
                          Auto: sized to fit your categories, about 1,800 tokens each. If it&apos;s
                          too small, videos are checked one category at a time, which is slower.
                        </p>
                      </Field>
                      <SwitchRow
                        id="categorization-context-length-auto"
                        label="Auto"
                        name="Auto context length"
                        checked={form.categorization_context_length_auto}
                        disabled={aiOff}
                        onChange={setCategorizationAuto}
                      />
                      <Field id="categorization-temperature" label="Temperature">
                        <Input
                          id="categorization-temperature"
                          aria-label="Categorization temperature"
                          type="number"
                          min={0}
                          max={2}
                          step={0.1}
                          value={form.categorization_temperature}
                          disabled={aiOff}
                          onChange={(event) => set('categorization_temperature', event.target.value)}
                        />
                      </Field>
                      <ThinkingField
                        id="categorization-thinking"
                        name="Categorization thinking"
                        model={form.categorization_model}
                        cannotThink={categorizationCannotThink}
                        checked={form.categorization_thinking}
                        disabled={aiOff}
                        onChange={(value) => set('categorization_thinking', value)}
                      />
                    </>
                  )}
                </>
              )}
              <PromptDisclosure
                isDefault={
                  saved.categorization_prompt_is_default &&
                  form.categorization_prompt === saved.categorization_prompt
                }
              >
                <PromptField
                  id="categorization-prompt"
                  name="Categorization prompt"
                  rows={8}
                  value={form.categorization_prompt}
                  disabled={aiOff}
                  canReset={!saved.categorization_prompt_is_default}
                  resetDisabled={saving || aiOff}
                  onChange={(value) => set('categorization_prompt', value)}
                  onReset={() => void resetCategorizationPrompt()}
                />
              </PromptDisclosure>
            </>
          )}
        </SettingsCard>
      )}
      <SaveBar
        changes={changedSettingNames(patch)}
        error={saveError}
        saving={saving}
        onDiscard={discard}
        onSave={save}
      />
    </>
  )
}

/** The provider every AI feature runs on, as one option card each. */
function ProviderChoice({
  providers,
  value,
  onChange,
}: {
  providers: Provider[]
  value: AiProvider
  onChange: (value: AiProvider) => void
}) {
  return (
    <div role="radiogroup" aria-label="Provider" className="flex flex-wrap gap-3">
      {providers.map((provider) => {
        const selected = value === provider.value
        return (
          <label
            key={provider.value}
            className={cn(
              'flex min-w-56 flex-1 cursor-pointer items-start gap-3 rounded-lg border p-4 text-sm',
              selected ? 'border-primary bg-accent ring-1 ring-primary' : 'border-border',
            )}
          >
            <input
              type="radio"
              name="ai-provider"
              value={provider.value}
              checked={selected}
              aria-labelledby={`provider-${provider.value}-label`}
              aria-describedby={`provider-${provider.value}-description`}
              onChange={() => onChange(provider.value)}
              className="mt-0.5 accent-primary"
            />
            <span className="flex flex-col gap-0.5">
              <span id={`provider-${provider.value}-label`} className="font-medium text-foreground">
                {provider.label}
              </span>
              <span
                id={`provider-${provider.value}-description`}
                className="text-muted-foreground"
              >
                {provider.description}
              </span>
            </span>
          </label>
        )
      })}
    </div>
  )
}

interface Provider {
  value: AiProvider
  label: string
  description: string
}

const PROVIDERS: Provider[] = [
  {
    value: 'ollama',
    label: 'Ollama',
    description: 'Runs on your own hardware. Free, private, slower on small machines.',
  },
  {
    value: 'anthropic',
    label: 'Anthropic',
    description: 'Claude models in the cloud. Fast and accurate, billed per token to your API key.',
  },
]

const OLLAMA_ONLY = PROVIDERS.filter((provider) => provider.value === 'ollama')

/** "Model": the server's models when known, otherwise a text field. */
function ModelField({
  id,
  name,
  models,
  missing,
  value,
  disabled,
  onChange,
}: {
  id: string
  /** The field's accessible name, when the label alone is ambiguous. */
  name?: string
  models: string[] | undefined
  /** How a selected model the provider doesn't have is marked. */
  missing: string
  value: string
  disabled: boolean
  onChange: (value: string) => void
}) {
  return (
    <Field id={id} label="Model">
      {models ? (
        <select
          id={id}
          aria-label={name}
          value={value}
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
          className={SELECT_CLASS}
        >
          {modelOptions(models, value, missing).map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      ) : (
        <Input
          id={id}
          aria-label={name}
          value={value}
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
        />
      )}
    </Field>
  )
}

function AdvancedToggle({
  name,
  open,
  onToggle,
}: {
  /** The button's accessible name, when "Advanced" alone is ambiguous. */
  name?: string
  open: boolean
  onToggle: (open: boolean) => void
}) {
  return (
    <button
      type="button"
      aria-label={name}
      aria-expanded={open}
      onClick={() => onToggle(!open)}
      className="flex items-center gap-1 self-start text-sm font-medium text-muted-foreground hover:text-foreground"
    >
      <ChevronRight
        className={cn('size-4 transition-transform', open && 'rotate-90')}
        aria-hidden="true"
      />
      Advanced
    </button>
  )
}

/** "Effort": how much an Anthropic model reasons before answering. */
function EffortField({
  name,
  value,
  disabled,
  onChange,
}: {
  /** The choice's accessible name, e.g. "Summaries effort". */
  name: string
  value: Effort
  disabled: boolean
  onChange: (value: Effort) => void
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-sm font-medium" aria-hidden="true">
        Effort
      </span>
      <Segmented name={name} choices={EFFORTS} value={value} disabled={disabled} onChange={onChange} />
    </div>
  )
}

const EFFORTS: { value: Effort; label: string }[] = [
  { value: 'low', label: 'Low' },
  { value: 'medium', label: 'Medium' },
  { value: 'high', label: 'High' },
]

/** The "Thinking" switch, or a note when `model` is known not to support it. */
function ThinkingField({
  id,
  name,
  model,
  cannotThink,
  checked,
  disabled,
  onChange,
}: {
  id: string
  /** The switch's accessible name, when the label alone is ambiguous. */
  name?: string
  model: string
  cannotThink: boolean
  checked: boolean
  disabled: boolean
  onChange: (value: boolean) => void
}) {
  if (cannotThink) {
    return <p className="text-sm text-muted-foreground">{model} doesn't support thinking</p>
  }
  return (
    <SwitchRow
      id={id}
      label="Thinking"
      name={name}
      help={THINKING_HELP}
      checked={checked}
      disabled={disabled}
      onChange={onChange}
    />
  )
}

/** "Prompt": the whole prompt, saying when it is the default one or resetting it. */
function PromptField({
  id,
  name,
  help,
  rows,
  value,
  disabled,
  canReset,
  resetDisabled,
  onChange,
  onReset,
}: {
  id: string
  /** The field's accessible name, when the label alone is ambiguous. */
  name?: string
  help?: ReactNode
  rows: number
  value: string
  disabled: boolean
  canReset: boolean
  resetDisabled: boolean
  onChange: (value: string) => void
  onReset: () => void
}) {
  return (
    <Field id={id} label="Prompt" help={help}>
      <textarea
        id={id}
        aria-label={name}
        rows={rows}
        spellCheck={false}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
        className="w-full rounded-lg border border-input bg-transparent px-2.5 py-2 font-mono text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-input/30"
      />
      <div className="flex items-center justify-end gap-4">
        {canReset && (
          <Button
            variant="outline"
            size="sm"
            disabled={resetDisabled}
            onClick={onReset}
          >
            Reset to default
          </Button>
        )}
      </div>
    </Field>
  )
}

const SELECT_CLASS =
  'h-8 w-full rounded-lg border border-input bg-transparent px-2 text-sm outline-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50 dark:bg-input/30'

const THINKING_HELP =
  'Lets the model reason before answering: slower, but more accurate. Some models, such as qwen3:4b, think even when it is off.'

interface ConnectionCheck {
  checking: boolean
  failed: boolean
  message: string | null
}

const NO_CHECK: ConnectionCheck = { checking: false, failed: false, message: null }

function connectedMessage(server: OllamaServer): string {
  return `Connected to Ollama ${server.version}, ${modelCount(server.models.length)}`
}

function anthropicConnectedMessage(server: AnthropicServer): string {
  return `Connected to Anthropic, ${modelCount(server.models.length)}`
}

function modelCount(count: number): string {
  return `${count} ${count === 1 ? 'model' : 'models'}`
}

/** What the last "Test connection" found, if it ran. */
function ConnectionMessage({ connection }: { connection: ConnectionCheck }) {
  if (!connection.message) {
    return null
  }
  return (
    <p className={cn('text-sm', connection.failed ? 'text-destructive' : 'text-muted-foreground')}>
      {connection.message}
    </p>
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
