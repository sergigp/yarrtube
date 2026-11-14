import { describe, expect, it } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AiSettingsView } from './AiSettingsView'
import { fetchChannelVideos } from '@/api/client'
import { queryKeys } from '@/api/queries'
import type { Preferences } from '@/api/types'
import {
  aPreferences,
  aVideo,
  anAnthropicServer,
  anOllamaServer,
  mockApi,
  renderWithProviders,
} from '@/test/helpers'

describe('AiSettingsView', () => {
  it('shows the AI features switch beside the title', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })
    renderWithProviders(<AiSettingsView />)

    const toggle = await screen.findByRole('switch', { name: 'AI features' })
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('AI')
    expect(
      screen.getByText(
        'Summaries and automatic categorization, run on a local Ollama server or on Anthropic.',
      ),
    ).toBeInTheDocument()
    expect(
      within(screen.getByRole('group', { name: 'Provider' })).queryByRole('switch'),
    ).not.toBeInTheDocument()
    expect(toggle).not.toBeChecked()
  })

  it('shows the defaults, with the model disabled and the advanced settings hidden', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })
    renderWithProviders(<AiSettingsView />)

    expect(await screen.findByRole('switch', { name: 'AI features' })).not.toBeChecked()
    expect(screen.getByRole('textbox', { name: 'Ollama URL' })).toHaveValue('')
    expect(screen.getByRole('switch', { name: 'Summaries' })).not.toBeChecked()
    expect(screen.getByRole('switch', { name: 'Summaries' })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: 'Model' })).toHaveValue('qwen3:4b')
    expect(screen.getByRole('textbox', { name: 'Model' })).toBeDisabled()
    expect(screen.queryByRole('button', { name: 'Advanced' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Prompt/ })).not.toBeInTheDocument()
  })

  it('shows no save bar until a field changes', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await screen.findByRole('switch', { name: 'AI features' })
    expect(screen.queryByRole('region', { name: 'Unsaved changes' })).not.toBeInTheDocument()

    await user.type(screen.getByRole('textbox', { name: 'Ollama URL' }), 'http://mac.local:11434')

    expect(screen.getByRole('region', { name: 'Unsaved changes' })).toBeInTheDocument()
  })

  it('lists the unsaved changes in the save bar', async () => {
    mockApi({
      'GET /api/preferences': { ...aiOn(), categorization_enabled: true },
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()
    await openAdvanced()
    await user.click(screen.getByRole('button', { name: 'Advanced categorization settings' }))

    await user.type(screen.getByRole('spinbutton', { name: 'Temperature' }), '5')
    await user.type(screen.getByRole('spinbutton', { name: 'Categorization temperature' }), '5')

    expect(screen.getByRole('region', { name: 'Unsaved changes' })).toHaveTextContent(
      '2 unsaved changes · Summaries temperature, Categorization temperature',
    )
  })

  it('discards changes without a request', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': { status: 400, error: 'Ollama URL must be set to enable AI' },
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()
    await user.click(await screen.findByRole('switch', { name: 'AI features' }))
    await user.click(screen.getByRole('button', { name: 'Save changes' }))
    await screen.findByRole('alert')
    const callsBefore = fetchMock.mock.calls.length

    await user.click(screen.getByRole('button', { name: 'Discard' }))

    expect(screen.getByRole('switch', { name: 'AI features' })).not.toBeChecked()
    expect(screen.queryByRole('region', { name: 'Unsaved changes' })).not.toBeInTheDocument()
    expect(fetchMock.mock.calls).toHaveLength(callsBefore)
    await user.click(screen.getByRole('switch', { name: 'AI features' }))
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('describes each provider option', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ anthropic_available: true }) })
    renderWithProviders(<AiSettingsView />)

    expect(await screen.findByRole('radio', { name: 'Ollama' })).toHaveAccessibleDescription(
      'Runs on your own hardware. Free, private, slower on small machines.',
    )
    expect(screen.getByRole('radio', { name: 'Anthropic' })).toHaveAccessibleDescription(
      'Claude models in the cloud. Fast and accurate, billed per token to your API key.',
    )
  })

  it('hides the provider choice without an Anthropic key', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ anthropic_available: false }) })
    renderWithProviders(<AiSettingsView />)

    const section = within(await screen.findByRole('group', { name: 'Provider' }))
    expect(section.queryByRole('radiogroup', { name: 'Provider' })).not.toBeInTheDocument()
    expect(section.getByRole('textbox', { name: 'Ollama URL' })).toHaveValue('')
    expect(section.getByRole('button', { name: 'Test connection' })).toBeDisabled()
  })

  it('saves only ai_provider when switching to Anthropic and shows its settings', async () => {
    const saved = { ...aiOn(), anthropic_available: true }
    const fetchMock = mockApi({
      'GET /api/preferences': saved,
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
      'GET /api/llm/anthropic': { status: 502, error: 'down' },
      'PATCH /api/preferences': { ...saved, ai_provider: 'anthropic' },
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('radio', { name: 'Anthropic' }))

    expect(screen.queryByRole('textbox', { name: 'Ollama URL' })).not.toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Model' })).toHaveValue('claude-haiku-5-5')
    await user.click(screen.getByRole('button', { name: 'Save changes' }))
    expect(await screen.findByText('Saved')).toBeInTheDocument()
    expect(patchBodies(fetchMock)).toEqual([{ ai_provider: 'anthropic' }])
  })

  it('lists Anthropic models and marks a saved one not listed as (not available)', async () => {
    mockApi({
      'GET /api/preferences': onAnthropic({ anthropic_summaries_model: 'claude-haiku-4-5' }),
      'GET /api/llm/anthropic': anAnthropicServer(),
    })
    renderWithProviders(<AiSettingsView />)

    const model = await screen.findByRole('combobox', { name: 'Model' })
    expect(optionTexts(model)).toEqual([
      'claude-sonnet-5-5',
      'claude-haiku-5-5',
      'claude-haiku-4-5 (not available)',
    ])
    expect(model).toHaveValue('claude-haiku-4-5')
    expect(screen.getByRole('combobox', { name: 'Categorization model' })).toHaveValue(
      'claude-haiku-5-5',
    )
  })

  it('shows Anthropic effort as a segmented choice outside Advanced', async () => {
    const saved = onAnthropic()
    const fetchMock = mockApi({
      'GET /api/preferences': saved,
      'GET /api/llm/anthropic': anAnthropicServer(),
      'PATCH /api/preferences': { ...saved, anthropic_summaries_effort: 'high' },
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    const effort = await screen.findByRole('radiogroup', { name: 'Summaries effort' })
    expect(within(effort).getByRole('radio', { name: 'Low' })).toBeChecked()
    expect(
      within(screen.getByRole('radiogroup', { name: 'Categorization effort' })).getByRole(
        'radio',
        { name: 'Low' },
      ),
    ).toBeChecked()
    expect(screen.getAllByText('Higher is slower and uses more tokens.')).toHaveLength(2)
    expect(screen.queryByRole('button', { name: /advanced/i })).not.toBeInTheDocument()
    expect(screen.queryByRole('spinbutton')).not.toBeInTheDocument()
    expect(screen.queryByRole('switch', { name: /thinking/i })).not.toBeInTheDocument()
    await user.click(within(effort).getByRole('radio', { name: 'High' }))
    await user.click(screen.getByRole('button', { name: 'Save changes' }))
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Saved'))
    expect(patchBodies(fetchMock)).toEqual([{ anthropic_summaries_effort: 'high' }])
  })

  it('shows "Connected to Anthropic, <n> models" after testing the connection', async () => {
    mockApi({
      'GET /api/preferences': aPreferences({ anthropic_available: true }),
      'GET /api/llm/anthropic': anAnthropicServer(),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('radio', { name: 'Anthropic' }))
    await user.click(screen.getByRole('button', { name: 'Test connection' }))

    expect(await screen.findByText('Connected to Anthropic, 2 models')).toBeInTheDocument()
    expect(optionTexts(screen.getByRole('combobox', { name: 'Model' }))).toEqual([
      'claude-sonnet-5-5',
      'claude-haiku-5-5',
    ])
  })

  it("says the key isn't set when Anthropic is saved without one", async () => {
    mockApi({
      'GET /api/preferences': onAnthropic({ anthropic_available: false }),
    })
    renderWithProviders(<AiSettingsView />)

    const section = within(await screen.findByRole('group', { name: 'Provider' }))
    expect(
      section.getByText("Anthropic is selected but YARRTUBE_ANTHROPIC_API_KEY isn't set"),
    ).toBeInTheDocument()
    expect(
      within(section.getByRole('radiogroup', { name: 'Provider' }))
        .getAllByRole('radio')
        .map((radio) => radio.getAttribute('value')),
    ).toEqual(['ollama'])
  })

  it('shows the Categorization section disabled on a fresh install', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })
    renderWithProviders(<AiSettingsView />)

    const section = within(await screen.findByRole('group', { name: 'Categorization' }))
    expect(section.getByRole('switch', { name: 'Categorization' })).not.toBeChecked()
    expect(section.getByRole('switch', { name: 'Categorization' })).toBeDisabled()
    expect(section.getByText('Model')).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Categorization model' })).toHaveValue('qwen3:4b')
    expect(screen.getByRole('textbox', { name: 'Categorization model' })).toBeDisabled()
    expect(section.queryByRole('button', { name: /advanced/i })).not.toBeInTheDocument()
  })

  it('hides the Categorization section while categories are off', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ categories_enabled: false }) })
    renderWithProviders(<AiSettingsView />)

    expect(await screen.findByRole('group', { name: 'Summaries' })).toBeInTheDocument()
    expect(screen.queryByRole('group', { name: 'Categorization' })).not.toBeInTheDocument()
  })

  it('shows the shipped categorization prompt under its Prompt', async () => {
    mockApi({
      'GET /api/preferences': aPreferences({ ai_enabled: true, categorization_enabled: true }),
    })
    renderWithProviders(<AiSettingsView />)

    const card = within(await screen.findByRole('group', { name: 'Categorization' }))
    expect(card.getByRole('button', { name: /^Prompt/ })).toHaveAttribute('aria-expanded', 'false')
    expect(screen.queryByRole('textbox', { name: 'Categorization prompt' })).not.toBeInTheDocument()

    await openPrompt('Categorization')

    expect(screen.getByRole('textbox', { name: 'Categorization prompt' })).toHaveValue(
      aPreferences().categorization_prompt,
    )
    expect(screen.queryByRole('button', { name: /reset/i })).not.toBeInTheDocument()
  })

  it('shows the categorization generation settings under its Advanced', async () => {
    mockApi({
      'GET /api/preferences': aPreferences({ ai_enabled: true, categorization_enabled: true }),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(
      await screen.findByRole('button', { name: 'Advanced categorization settings' }),
    )

    expect(screen.getByRole('spinbutton', { name: 'Categorization context length' })).toHaveValue(4096)
    expect(screen.getByRole('spinbutton', { name: 'Categorization temperature' })).toHaveValue(0)
    expect(screen.getByRole('switch', { name: 'Categorization thinking' })).toBeChecked()
  })

  it('saves only the changed categorization fields', async () => {
    const saved = aPreferences({ ai_enabled: true, ollama_url: 'http://mac.local:11434' })
    const fetchMock = mockApi({
      'GET /api/preferences': saved,
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': { status: 502, error: 'down' },
      'PATCH /api/preferences': {
        ...saved,
        categorization_enabled: true,
        categorization_model: 'gemma3:4b',
      },
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Categorization' }))
    const model = screen.getByRole('textbox', { name: 'Categorization model' })
    await user.clear(model)
    await user.type(model, 'gemma3:4b')
    await user.click(screen.getByRole('button', { name: 'Save changes' }))

    expect(await screen.findByText('Saved')).toBeInTheDocument()
    expect(patchBodies(fetchMock)).toEqual([{ categorization_enabled: true, categorization_model: 'gemma3:4b' }])
  })

  it('saves only the changed categorization advanced setting', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': { ...aiOn(), categorization_enabled: true },
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
      'PATCH /api/preferences': aiOn(),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(
      await screen.findByRole('button', { name: 'Advanced categorization settings' }),
    )
    await user.click(screen.getByRole('switch', { name: 'Auto context length' }))
    const contextLength = screen.getByRole('spinbutton', { name: 'Categorization context length' })
    await user.clear(contextLength)
    await user.type(contextLength, '8192')
    await user.click(screen.getByRole('button', { name: 'Save changes' }))

    await screen.findByText('Saved')
    expect(patchBodies(fetchMock)).toEqual([{ categorization_context_length: 8192 }])
  })

  it('shows the Auto context length read-only, with how it scales', async () => {
    mockApi({
      'GET /api/preferences': aPreferences({
        ai_enabled: true,
        categorization_enabled: true,
        categorization_context_length: 16384,
        categorization_context_length_is_auto: true,
      }),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(
      await screen.findByRole('button', { name: 'Advanced categorization settings' }),
    )

    expect(screen.getByRole('switch', { name: 'Auto context length' })).toBeChecked()
    const contextLength = screen.getByRole('spinbutton', { name: 'Categorization context length' })
    expect(contextLength).toHaveValue(16384)
    expect(contextLength).toHaveAttribute('readonly')
    expect(
      screen.getByText(
        'Auto sizes it from your categories with an AI prompt, about 1,800 tokens each. ' +
          'Too small and videos are asked about one category at a time, which is slower.',
      ),
    ).toBeInTheDocument()
  })

  it('makes the context length editable when Auto is turned off', async () => {
    mockApi({
      'GET /api/preferences': aPreferences({
        ai_enabled: true,
        categorization_enabled: true,
        categorization_context_length: 16384,
        categorization_context_length_is_auto: true,
      }),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(
      await screen.findByRole('button', { name: 'Advanced categorization settings' }),
    )
    await user.click(screen.getByRole('switch', { name: 'Auto context length' }))

    const contextLength = screen.getByRole('spinbutton', { name: 'Categorization context length' })
    expect(contextLength).toHaveValue(16384)
    expect(contextLength).not.toHaveAttribute('readonly')
  })

  it('goes back to the Auto context length', async () => {
    const saved = aPreferences({
      ai_enabled: true,
      categorization_enabled: true,
      categorization_context_length: 8192,
      categorization_context_length_is_auto: false,
    })
    const fetchMock = mockApi({
      'GET /api/preferences': saved,
      'PATCH /api/preferences': {
        ...saved,
        categorization_context_length: 16384,
        categorization_context_length_is_auto: true,
      },
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(
      await screen.findByRole('button', { name: 'Advanced categorization settings' }),
    )
    await user.click(screen.getByRole('switch', { name: 'Auto context length' }))
    await user.click(screen.getByRole('button', { name: 'Save changes' }))

    await screen.findByText('Saved')
    expect(patchBodies(fetchMock)).toEqual([{ categorization_context_length: null }])
    const contextLength = screen.getByRole('spinbutton', { name: 'Categorization context length' })
    expect(contextLength).toHaveValue(16384)
    expect(contextLength).toHaveAttribute('readonly')
  })

  it('resets a saved categorization prompt to the default', async () => {
    const shipped = aPreferences().categorization_prompt
    const saved = aPreferences({
      ai_enabled: true,
      categorization_enabled: true,
      categorization_prompt: 'my prompt',
      categorization_prompt_is_default: false,
    })
    const fetchMock = mockApi({
      'GET /api/preferences': saved,
      'PATCH /api/preferences': {
        ...saved,
        categorization_prompt: shipped,
        categorization_prompt_is_default: true,
      },
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await openPrompt('Categorization')
    const card = within(screen.getByRole('group', { name: 'Categorization' }))
    const prompt = screen.getByRole('textbox', { name: 'Categorization prompt' })
    expect(prompt).toHaveValue('my prompt')
    expect(card.getByRole('button', { name: /^Prompt/ })).toHaveTextContent('Prompt · edited')
    await user.click(screen.getByRole('button', { name: 'Reset to default' }))

    await waitFor(() =>
      expect(card.getByRole('button', { name: /^Prompt/ })).toHaveTextContent('Prompt · default'),
    )
    expect(prompt).toHaveValue(shipped)
    expect(patchBodies(fetchMock)).toEqual([{ categorization_prompt: null }])
  })

  it('keeps the model disabled while summaries are off', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ ai_enabled: true }) })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    const model = await screen.findByRole('textbox', { name: 'Model' })
    expect(model).toBeDisabled()

    await user.click(screen.getByRole('switch', { name: 'Summaries' }))

    expect(model).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Advanced' })).toBeInTheDocument()
  })

  it('shows the summary settings once summaries are on', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'AI features' }))
    await user.click(screen.getByRole('switch', { name: 'Summaries' }))
    await openAdvanced()

    expect(screen.getByRole('textbox', { name: 'Model' })).toHaveValue('qwen3:4b')
    expect(screen.getByRole('spinbutton', { name: 'Context length' })).toHaveValue(12288)
    expect(screen.getByRole('spinbutton', { name: 'Temperature' })).toHaveValue(0.3)
    expect(screen.getByRole('switch', { name: 'Thinking' })).toBeChecked()
    await openPrompt('Summaries')
    expect(screen.getByRole('textbox', { name: 'Prompt' })).toHaveValue(
      aPreferences().summaries_prompt,
    )
  })

  it('keeps the advanced summary settings closed until asked for', async () => {
    mockApi({ 'GET /api/preferences': aiOn() })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    const advanced = await screen.findByRole('button', { name: 'Advanced' })
    expect(advanced).toHaveAttribute('aria-expanded', 'false')
    expect(screen.getByRole('textbox', { name: 'Model' })).toBeInTheDocument()
    expect(screen.queryByRole('spinbutton', { name: 'Context length' })).not.toBeInTheDocument()
    expect(screen.queryByRole('textbox', { name: 'Prompt' })).not.toBeInTheDocument()

    await user.click(advanced)

    expect(advanced).toHaveAttribute('aria-expanded', 'true')
    expect(screen.getByRole('spinbutton', { name: 'Context length' })).toBeInTheDocument()
    expect(screen.queryByRole('textbox', { name: 'Prompt' })).not.toBeInTheDocument()
  })

  it('disables the summary settings while AI is off', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ summaries_enabled: true }) })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()
    await openPrompt('Summaries')

    expect(await screen.findByRole('switch', { name: 'Summaries' })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: 'Model' })).toBeDisabled()
    expect(screen.getByRole('spinbutton', { name: 'Context length' })).toBeDisabled()
    expect(screen.getByRole('spinbutton', { name: 'Temperature' })).toBeDisabled()
    expect(screen.getByRole('switch', { name: 'Thinking' })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: 'Prompt' })).toBeDisabled()
  })

  it('saves only the changed fields and shows Saved by the title', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': aPreferences({
        ai_enabled: true,
        ollama_url: 'http://mac.local:11434',
      }),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.type(
      await screen.findByRole('textbox', { name: 'Ollama URL' }),
      'http://mac.local:11434',
    )
    await user.click(screen.getByRole('switch', { name: 'AI features' }))
    await user.click(screen.getByRole('button', { name: 'Save changes' }))

    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Saved'))
    expect(screen.getByRole('heading', { level: 1 }).parentElement).toContainElement(
      screen.getByRole('status'),
    )
    expect(patchBodies(fetchMock)).toEqual([
      { ai_enabled: true, ollama_url: 'http://mac.local:11434' },
    ])
    await waitFor(() =>
      expect(screen.queryByRole('region', { name: 'Unsaved changes' })).not.toBeInTheDocument(),
    )
  })

  it('refetches video lists after saving', async () => {
    let videoListRequests = 0
    mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
      'PATCH /api/preferences': aPreferences(),
      'GET /api/channels/handle/videos': () => {
        videoListRequests += 1
        return [aVideo()]
      },
    })
    const { queryClient } = renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()
    await queryClient.prefetchQuery({
      queryKey: queryKeys.channelVideos('handle'),
      queryFn: () => fetchChannelVideos('handle'),
    })

    await user.click(await screen.findByRole('switch', { name: 'AI features' }))
    await user.click(screen.getByRole('button', { name: 'Save changes' }))

    expect(await screen.findByText('Saved')).toBeInTheDocument()
    expect(videoListRequests).toBe(2)
  })

  it("shows a rejected save's error in the save bar", async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': { status: 400, error: 'Ollama URL must be set to enable AI' },
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'AI features' }))
    await user.click(screen.getByRole('button', { name: 'Save changes' }))

    await screen.findByRole('alert')
    expect(
      within(screen.getByRole('region', { name: 'Unsaved changes' })).getByRole('alert'),
    ).toHaveTextContent('Ollama URL must be set to enable AI')
    expect(screen.getByRole('switch', { name: 'AI features' })).toBeChecked()
  })

  it('refuses to save a number field that holds no number', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences({
        ai_enabled: true,
        ollama_url: 'http://mac.local:11434',
        summaries_enabled: true,
      }),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()
    const user = userEvent.setup()

    await user.clear(await screen.findByRole('spinbutton', { name: 'Temperature' }))
    await user.click(screen.getByRole('button', { name: 'Save changes' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Context length and temperature must be numbers',
    )
    expect(patchBodies(fetchMock)).toEqual([])
  })

  it('refuses to save an emptied categorization number', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': { ...aiOn(), categorization_enabled: true },
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(
      await screen.findByRole('button', { name: 'Advanced categorization settings' }),
    )
    await user.clear(screen.getByRole('spinbutton', { name: 'Categorization temperature' }))
    await user.click(screen.getByRole('button', { name: 'Save changes' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Context length and temperature must be numbers',
    )
    expect(patchBodies(fetchMock)).toEqual([])
  })

  it('tests the connection to the typed URL', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    const test = await screen.findByRole('button', { name: 'Test connection' })
    expect(test).toBeDisabled()
    await user.type(screen.getByRole('textbox', { name: 'Ollama URL' }), 'http://mac.local:11434')
    await user.click(test)

    expect(await screen.findByText('Connected to Ollama 0.40.13, 2 models')).toBeInTheDocument()
  })

  it('shows why the connection test failed', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'GET /api/llm/ollama?url=http%3A%2F%2Fnowhere%3A11434': {
        status: 502,
        error: 'Could not reach Ollama at http://nowhere:11434: connection refused',
      },
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.type(await screen.findByRole('textbox', { name: 'Ollama URL' }), 'http://nowhere:11434')
    await user.click(screen.getByRole('button', { name: 'Test connection' }))

    expect(
      await screen.findByText('Could not reach Ollama at http://nowhere:11434: connection refused'),
    ).toBeInTheDocument()
  })

  it("lists the saved server's models to pick from", async () => {
    mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)

    const model = await screen.findByRole('combobox', { name: 'Model' })

    expect(optionTexts(model)).toEqual(['gemma3:4b', 'qwen3:4b'])
    expect(model).toHaveValue('qwen3:4b')
  })

  it("lists the saved server's models in the categorization Model independently", async () => {
    mockApi({
      'GET /api/preferences': { ...aiOn(), categorization_enabled: true },
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    const model = await screen.findByRole('combobox', { name: 'Categorization model' })
    await user.selectOptions(model, 'gemma3:4b')

    expect(optionTexts(model)).toEqual(['gemma3:4b', 'qwen3:4b'])
    expect(model).toHaveValue('gemma3:4b')
    expect(screen.getByRole('combobox', { name: 'Model' })).toHaveValue('qwen3:4b')
  })

  it('keeps a saved model the server no longer has', async () => {
    mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer({
        models: [{ name: 'gemma3:4b', size_bytes: 1, thinking: false }],
      }),
    })
    renderWithProviders(<AiSettingsView />)

    const model = await screen.findByRole('combobox', { name: 'Model' })

    expect(optionTexts(model)).toEqual(['gemma3:4b', 'qwen3:4b (not installed)'])
    expect(model).toHaveValue('qwen3:4b')
  })

  it('lets the model be typed when the server is down', async () => {
    mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': { status: 502, error: 'down' },
    })
    renderWithProviders(<AiSettingsView />)

    expect(await screen.findByRole('textbox', { name: 'Model' })).toHaveValue('qwen3:4b')
  })

  it('lists the models of a newly saved server', async () => {
    mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
      'PATCH /api/preferences': aPreferences({ ai_enabled: true, ollama_url: 'http://nas:11434' }),
      'GET /api/llm/ollama?url=http%3A%2F%2Fnas%3A11434': anOllamaServer({
        models: [{ name: 'llama3:8b', size_bytes: 1, thinking: false }],
      }),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await screen.findByRole('combobox', { name: 'Model' })
    const url = screen.getByRole('textbox', { name: 'Ollama URL' })
    await user.clear(url)
    await user.type(url, 'http://nas:11434')
    await user.click(screen.getByRole('button', { name: 'Save changes' }))

    await waitFor(() =>
      expect(optionTexts(screen.getByRole('combobox', { name: 'Model' }))).toEqual([
        'llama3:8b',
        'qwen3:4b (not installed)',
      ]),
    )
  })

  it('lists the models of the server just tested', async () => {
    mockApi({
      'GET /api/preferences': aPreferences({ summaries_enabled: true }),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.type(await screen.findByRole('textbox', { name: 'Ollama URL' }), 'http://mac.local:11434')
    await user.click(screen.getByRole('button', { name: 'Test connection' }))

    expect(optionTexts(await screen.findByRole('combobox', { name: 'Model' }))).toEqual([
      'gemma3:4b',
      'qwen3:4b',
    ])
  })

  it('offers thinking for a model that can think', async () => {
    mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()

    await screen.findByRole('combobox', { name: 'Model' })

    expect(screen.getByRole('switch', { name: 'Thinking' })).toBeChecked()
  })

  it("replaces thinking with a note for a model that can't think, and doesn't save it", async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
      'PATCH /api/preferences': aiOn(),
    })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Thinking' }))
    await user.selectOptions(screen.getByRole('combobox', { name: 'Model' }), 'gemma3:4b')

    expect(screen.queryByRole('switch', { name: 'Thinking' })).not.toBeInTheDocument()
    expect(screen.getByText("gemma3:4b doesn't support thinking")).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Save changes' }))
    await screen.findByText('Saved')
    expect(patchBodies(fetchMock)).toEqual([{ summaries_model: 'gemma3:4b' }])
  })

  it('decides thinking section by section', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': { ...aiOn(), categorization_enabled: true },
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
      'PATCH /api/preferences': aiOn(),
    })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()
    const user = userEvent.setup()
    await user.click(
      await screen.findByRole('button', { name: 'Advanced categorization settings' }),
    )

    await user.click(await screen.findByRole('switch', { name: 'Categorization thinking' }))
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Categorization model' }),
      'gemma3:4b',
    )

    expect(screen.getByRole('switch', { name: 'Thinking' })).toBeChecked()
    expect(
      screen.queryByRole('switch', { name: 'Categorization thinking' }),
    ).not.toBeInTheDocument()
    expect(screen.getByText("gemma3:4b doesn't support thinking")).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Save changes' }))
    await screen.findByText('Saved')
    expect(patchBodies(fetchMock)).toEqual([{ categorization_model: 'gemma3:4b' }])
  })

  it('offers thinking while the models are unknown', async () => {
    mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/llm/ollama?url=http%3A%2F%2Fmac.local%3A11434': { status: 502, error: 'down' },
    })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()

    await screen.findByRole('textbox', { name: 'Model' })

    expect(screen.getByRole('switch', { name: 'Thinking' })).toBeInTheDocument()
  })

  it('explains thinking', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ summaries_enabled: true }) })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()

    await userEvent.setup().hover(await screen.findByRole('button', { name: 'About Thinking' }))

    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      'Lets the model reason before answering: slower, but more accurate. Some models, such as qwen3:4b, think even when it is off.',
    )
  })

  it('marks the prompt default or edited', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ ai_enabled: true, summaries_enabled: true }) })
    renderWithProviders(<AiSettingsView />)
    await openPrompt('Summaries')
    const toggle = within(screen.getByRole('group', { name: 'Summaries' })).getByRole('button', {
      name: /^Prompt/,
    })

    expect(toggle).toHaveTextContent('Prompt · default')
    expect(screen.queryByRole('button', { name: 'Reset to default' })).not.toBeInTheDocument()
    await userEvent.setup().type(screen.getByRole('textbox', { name: 'Prompt' }), ' more')

    expect(toggle).toHaveTextContent('Prompt · edited')
  })

  it('resets a saved prompt to the default', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences({
        ai_enabled: true,
        summaries_enabled: true,
        summaries_prompt: 'my prompt',
        summaries_prompt_is_default: false,
      }),
      'PATCH /api/preferences': aPreferences({ ai_enabled: true, summaries_enabled: true }),
    })
    renderWithProviders(<AiSettingsView />)
    await openPrompt('Summaries')
    const user = userEvent.setup()

    expect(await screen.findByRole('textbox', { name: 'Prompt' })).toHaveValue('my prompt')
    const toggle = within(screen.getByRole('group', { name: 'Summaries' })).getByRole('button', {
      name: /^Prompt/,
    })
    expect(toggle).toHaveTextContent('Prompt · edited')
    await user.click(screen.getByRole('button', { name: 'Reset to default' }))

    await waitFor(() => expect(toggle).toHaveTextContent('Prompt · default'))
    expect(screen.getByRole('textbox', { name: 'Prompt' })).toHaveValue(
      aPreferences().summaries_prompt,
    )
    expect(patchBodies(fetchMock)).toEqual([{ summaries_prompt: null }])
  })

  it('explains each prompt section and its variables behind a "?" next to the prompt', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ summaries_enabled: true }) })
    renderWithProviders(<AiSettingsView />)
    await openPrompt('Summaries')
    const user = userEvent.setup()

    expect(screen.queryByRole('region', { name: 'Prompt guide' })).not.toBeInTheDocument()
    await user.click(await screen.findByRole('button', { name: 'About Prompt' }))

    const guide = await screen.findByRole('region', { name: 'Prompt guide' })
    const reducePart = within(guide).getByRole('group', { name: 'reduce part' })

    expect(within(reducePart).getByText('How each part summary is written into {part_summaries}.')).toBeInTheDocument()
    expect(within(reducePart).getAllByRole('term').map((term) => term.textContent)).toEqual([
      '{part}',
      '{span}',
      '{summary}',
    ])
  })
})

/** Opens the "Prompt" disclosure of the `card` ("Summaries" or "Categorization"). */
async function openPrompt(card: 'Summaries' | 'Categorization') {
  await userEvent
    .setup()
    .click(
      within(await screen.findByRole('group', { name: card })).getByRole('button', {
        name: /^Prompt/,
      }),
    )
}

/** Opens the "Advanced" summary settings, once the page has loaded. */
async function openAdvanced() {
  await userEvent.setup().click(await screen.findByRole('button', { name: 'Advanced' }))
}

function aiOn() {
  return aPreferences({
    ai_enabled: true,
    ollama_url: 'http://mac.local:11434',
    summaries_enabled: true,
  })
}

/** AI with summaries and categorization on, running on Anthropic. */
function onAnthropic(overrides: Partial<Preferences> = {}): Preferences {
  return aPreferences({
    ai_enabled: true,
    ai_provider: 'anthropic',
    anthropic_available: true,
    summaries_enabled: true,
    categorization_enabled: true,
    ...overrides,
  })
}

function optionTexts(select: HTMLElement): (string | null)[] {
  return Array.from(select.querySelectorAll('option')).map((option) => option.textContent)
}

/** The body of every `PATCH /api/preferences` sent. */
function patchBodies(fetchMock: ReturnType<typeof mockApi>): unknown[] {
  return fetchMock.mock.calls
    .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PATCH')
    .map(([, init]) => JSON.parse((init as RequestInit).body as string))
}
