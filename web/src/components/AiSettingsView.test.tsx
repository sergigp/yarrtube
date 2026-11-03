import { describe, expect, it } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AiSettingsView } from './AiSettingsView'
import { aPreferences, anOllamaServer, mockApi, renderWithProviders } from '@/test/helpers'

describe('AiSettingsView', () => {
  it('shows the defaults with summary settings disabled until AI is on', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })
    renderWithProviders(<AiSettingsView />)
    const user = userEvent.setup()

    const enableAi = await screen.findByRole('switch', { name: 'Enable AI' })
    expect(enableAi).not.toBeChecked()
    expect(screen.getByRole('textbox', { name: 'Ollama URL' })).toHaveValue('')
    expect(screen.getByRole('switch', { name: 'Enable summaries' })).toBeDisabled()
    expect(screen.getByRole('textbox', { name: 'Model' })).toHaveValue('qwen3:4b')
    expect(screen.getByRole('textbox', { name: 'Model' })).toBeDisabled()
    expect(screen.getByRole('spinbutton', { name: 'Context length' })).toHaveValue(12288)
    expect(screen.getByRole('spinbutton', { name: 'Temperature' })).toHaveValue(0.3)
    expect(screen.getByRole('switch', { name: 'Thinking' })).toBeChecked()
    expect(screen.getByRole('textbox', { name: 'Prompt' })).toHaveValue(
      aPreferences().summaries_prompt,
    )

    await user.click(enableAi)

    expect(screen.getByRole('switch', { name: 'Enable summaries' })).toBeEnabled()
    expect(screen.getByRole('textbox', { name: 'Model' })).toBeEnabled()
    expect(screen.getByRole('spinbutton', { name: 'Context length' })).toBeEnabled()
    expect(screen.getByRole('spinbutton', { name: 'Temperature' })).toBeEnabled()
    expect(screen.getByRole('switch', { name: 'Thinking' })).toBeEnabled()
    expect(screen.getByRole('textbox', { name: 'Prompt' })).toBeEnabled()
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
      }),
      'GET /api/ollama?url=http%3A%2F%2Fmac.local%3A11434': anOllamaServer(),
    })
    renderWithProviders(<AiSettingsView />)
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

  it('lists the models of the server just tested', async () => {
    mockApi({
      'GET /api/preferences': aPreferences(),
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

    await screen.findByRole('textbox', { name: 'Model' })

    expect(screen.getByRole('switch', { name: 'Thinking' })).toBeInTheDocument()
  })

  it('explains thinking', async () => {
    mockApi({ 'GET /api/preferences': aPreferences() })
    renderWithProviders(<AiSettingsView />)

    await userEvent.setup().hover(await screen.findByRole('button', { name: 'About Thinking' }))

    expect(await screen.findByRole('tooltip')).toHaveTextContent(
      'Lets the model reason before answering: slower, but more accurate. Some models, such as qwen3:4b, think even when it is off.',
    )
  })
})

function aiOn() {
  return aPreferences({ ai_enabled: true, ollama_url: 'http://mac.local:11434' })
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
