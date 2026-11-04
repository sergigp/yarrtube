import { describe, expect, it } from 'vitest'
import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AiSettingsView } from './AiSettingsView'
import { fetchChannelVideos } from '@/api/client'
import { queryKeys } from '@/api/queries'
import { aPreferences, aVideo, anOllamaServer, mockApi, renderWithProviders } from '@/test/helpers'

describe('AiSettingsView', () => {
  it('shows the defaults, with the model disabled and the advanced settings hidden', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })
    renderWithProviders(<AiSettingsView />)

    expect(await screen.findByRole('switch', { name: 'Enable AI' })).not.toBeChecked()
    expect(screen.getByRole('textbox', { name: 'Ollama URL' })).toHaveValue('')
    expect(screen.getByRole('switch', { name: 'Summaries enabled' })).not.toBeChecked()
    expect(screen.getByRole('switch', { name: 'Summaries enabled' })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: 'Model' })).toHaveValue('qwen3:4b')
    expect(screen.getByRole('textbox', { name: 'Model' })).toBeDisabled()
    expect(screen.queryByRole('button', { name: 'Advanced' })).not.toBeInTheDocument()
  })

  it('keeps the model disabled while summaries are off', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ ai_enabled: true }) })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    const model = await screen.findByRole('textbox', { name: 'Model' })
    expect(model).toBeDisabled()

    await user.click(screen.getByRole('switch', { name: 'Summaries enabled' }))

    expect(model).toBeEnabled()
    expect(screen.getByRole('button', { name: 'Advanced' })).toBeInTheDocument()
  })

  it('shows the summary settings once summaries are on', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Enable AI' }))
    await user.click(screen.getByRole('switch', { name: 'Summaries enabled' }))
    await openAdvanced()

    expect(screen.getByRole('textbox', { name: 'Model' })).toHaveValue('qwen3:4b')
    expect(screen.getByRole('spinbutton', { name: 'Context length' })).toHaveValue(12288)
    expect(screen.getByRole('spinbutton', { name: 'Temperature' })).toHaveValue(0.3)
    expect(screen.getByRole('switch', { name: 'Thinking' })).toBeChecked()
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
    expect(screen.getByRole('textbox', { name: 'Prompt' })).toBeInTheDocument()
  })

  it('disables the summary settings while AI is off', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ summaries_enabled: true }) })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()

    expect(await screen.findByRole('switch', { name: 'Summaries enabled' })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: 'Model' })).toBeDisabled()
    expect(screen.getByRole('spinbutton', { name: 'Context length' })).toBeDisabled()
    expect(screen.getByRole('spinbutton', { name: 'Temperature' })).toBeDisabled()
    expect(screen.getByRole('switch', { name: 'Thinking' })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: 'Prompt' })).toBeDisabled()
  })

  it('saves only the changed fields', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': aPreferences({
        ai_enabled: true,
        ollama_url: 'http://mac.local:11434',
      }),
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    const save = await screen.findByRole('button', { name: 'Save' })
    expect(save).toBeDisabled()
    await user.type(screen.getByRole('textbox', { name: 'Ollama URL' }), 'http://mac.local:11434')
    await user.click(screen.getByRole('switch', { name: 'Enable AI' }))
    await user.click(save)

    expect(await screen.findByText('Saved')).toBeInTheDocument()
    expect(patchBodies(fetchMock)).toEqual([
      { ai_enabled: true, ollama_url: 'http://mac.local:11434' },
    ])
    await waitFor(() => expect(screen.getByRole('button', { name: 'Save' })).toBeDisabled())
  })

  it('refetches video lists after saving', async () => {
    let videoListRequests = 0
    mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
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

    await user.click(await screen.findByRole('switch', { name: 'Enable AI' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(await screen.findByText('Saved')).toBeInTheDocument()
    expect(videoListRequests).toBe(2)
  })

  it('shows why a save failed and keeps the edits', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'PATCH /api/preferences': { status: 400, error: 'Ollama URL must be set to enable AI' },
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Enable AI' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Ollama URL must be set to enable AI',
    )
    expect(screen.getByRole('switch', { name: 'Enable AI' })).toBeChecked()
  })

  it('refuses to save a number field that holds no number', async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': aPreferences({
        ai_enabled: true,
        ollama_url: 'http://mac.local:11434',
        summaries_enabled: true,
      }),
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()
    const user = userEvent.setup()

    await user.clear(await screen.findByRole('spinbutton', { name: 'Temperature' }))
    await user.click(screen.getByRole('button', { name: 'Save' }))

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Context length and temperature must be numbers',
    )
    expect(patchBodies(fetchMock)).toEqual([])
  })

  it('tests the connection to the typed URL', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
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
      'GET /api/ollama?url=http%3A%2F%2Fnowhere%3A11434': {
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
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)

    const model = await screen.findByRole('combobox', { name: 'Model' })

    expect(optionTexts(model)).toEqual(['gemma3:4b', 'qwen3:4b'])
    expect(model).toHaveValue('qwen3:4b')
  })

  it('keeps a saved model the server no longer has', async () => {
    mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer({
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
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': { status: 502, error: 'down' },
    })
    renderWithProviders(<AiSettingsView />)

    expect(await screen.findByRole('textbox', { name: 'Model' })).toHaveValue('qwen3:4b')
  })

  it('lists the models of a newly saved server', async () => {
    mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
      'PATCH /api/preferences': aPreferences({ ai_enabled: true, ollama_url: 'http://nas:11434' }),
      'GET /api/ollama?url=http%3A%2F%2Fnas%3A11434': anOllamaServer({
        models: [{ name: 'llama3:8b', size_bytes: 1, thinking: false }],
      }),
    })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    await screen.findByRole('combobox', { name: 'Model' })
    const url = screen.getByRole('textbox', { name: 'Ollama URL' })
    await user.clear(url)
    await user.type(url, 'http://nas:11434')
    await user.click(screen.getByRole('button', { name: 'Save' }))

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
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
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
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()

    await screen.findByRole('combobox', { name: 'Model' })

    expect(screen.getByRole('switch', { name: 'Thinking' })).toBeChecked()
  })

  it("replaces thinking with a note for a model that can't think, and doesn't save it", async () => {
    const fetchMock = mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
      'PATCH /api/preferences': aiOn(),
    })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()
    const user = userEvent.setup()

    await user.click(await screen.findByRole('switch', { name: 'Thinking' }))
    await user.selectOptions(screen.getByRole('combobox', { name: 'Model' }), 'gemma3:4b')

    expect(screen.queryByRole('switch', { name: 'Thinking' })).not.toBeInTheDocument()
    expect(screen.getByText("gemma3:4b doesn't support thinking")).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Save' }))
    await screen.findByText('Saved')
    expect(patchBodies(fetchMock)).toEqual([{ summaries_model: 'gemma3:4b' }])
  })

  it('offers thinking while the models are unknown', async () => {
    mockApi({
      'GET /api/preferences': aiOn(),
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': { status: 502, error: 'down' },
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

  it('says the default prompt is in use', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ summaries_enabled: true }) })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()

    expect(await screen.findByText('Using the default prompt')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Reset to default' })).not.toBeInTheDocument()
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
    await openAdvanced()
    const user = userEvent.setup()

    expect(await screen.findByRole('textbox', { name: 'Prompt' })).toHaveValue('my prompt')
    expect(screen.queryByText('Using the default prompt')).not.toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: 'Reset to default' }))

    expect(await screen.findByText('Using the default prompt')).toBeInTheDocument()
    expect(screen.getByRole('textbox', { name: 'Prompt' })).toHaveValue(
      aPreferences().summaries_prompt,
    )
    expect(patchBodies(fetchMock)).toEqual([{ summaries_prompt: null }])
  })

  it('explains each prompt section and its variables behind a "?" next to the prompt', async () => {
    mockApi({ 'GET /api/preferences': aPreferences({ summaries_enabled: true }) })
    renderWithProviders(<AiSettingsView />)
    await openAdvanced()
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

function optionTexts(select: HTMLElement): (string | null)[] {
  return Array.from(select.querySelectorAll('option')).map((option) => option.textContent)
}

/** The body of every `PATCH /api/preferences` sent. */
function patchBodies(fetchMock: ReturnType<typeof mockApi>): unknown[] {
  return fetchMock.mock.calls
    .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PATCH')
    .map(([, init]) => JSON.parse((init as RequestInit).body as string))
}
