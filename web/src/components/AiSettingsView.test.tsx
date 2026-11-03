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
})

/** The body of every `PATCH /api/preferences` sent. */
function patchBodies(fetchMock: ReturnType<typeof mockApi>): unknown[] {
  return fetchMock.mock.calls
    .filter(([, init]) => (init as RequestInit | undefined)?.method === 'PATCH')
    .map(([, init]) => JSON.parse((init as RequestInit).body as string))
}
