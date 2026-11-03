import { describe, expect, it } from 'vitest'
import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AiSettingsView } from './AiSettingsView'
import { aPreferences, mockApi, renderWithProviders } from '@/test/helpers'

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
})
