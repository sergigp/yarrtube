import { test, expect } from '../helpers/fixtures.js'
import { OLLAMA_VERSION } from '../ollama-stub.js'

test('AI settings: point at Ollama, pick a model and save, then reset the prompt', async ({ page, daemon }) => {
  const enableAi = page.getByRole('switch', { name: 'Enable AI' })
  const model = page.getByRole('combobox', { name: 'Model', exact: true })
  const temperature = page.getByRole('spinbutton', { name: 'Temperature' })
  const prompt = page.getByRole('textbox', { name: 'Prompt', exact: true })
  const save = page.getByRole('button', { name: 'Save' })

  await test.step('the settings menu opens the AI page', async () => {
    await page.goto('/')
    await page.getByRole('banner').getByRole('button', { name: 'Settings' }).click()
    await page.getByRole('menuitem', { name: 'AI', exact: true }).click()
    await expect(page).toHaveURL(/\/ai$/)
    await expect(enableAi).not.toBeChecked()
    await expect(prompt).toHaveCount(0)
  })

  await test.step('testing the connection lists the server and its models', async () => {
    await page.getByRole('textbox', { name: 'Ollama URL' }).fill(daemon.ollamaUrl)
    await page.getByRole('button', { name: 'Test connection' }).click()
    await expect(page.getByText(`Connected to Ollama ${OLLAMA_VERSION}, 2 models`)).toBeVisible()
    await expect(model.locator('option')).toHaveText(['gemma3:4b', 'qwen3:4b'])
  })

  await test.step('turning AI and summaries on with a model that can’t think is saved', async () => {
    await enableAi.click()
    await page.getByRole('switch', { name: 'Summaries enabled' }).click()
    await page.getByRole('button', { name: 'Advanced' }).click()
    await expect(page.getByText('Using the default prompt')).toBeVisible()
    await model.selectOption('gemma3:4b')
    await expect(page.getByText("gemma3:4b doesn't support thinking")).toBeVisible()
    await temperature.fill('0.5')
    await save.click()
    await expect(page.getByText('Saved')).toBeVisible()
    await expect(save).toBeDisabled()

    await page.reload()
    await page.getByRole('button', { name: 'Advanced' }).click()
    await expect(enableAi).toBeChecked()
    await expect(page.getByRole('switch', { name: 'Summaries enabled' })).toBeChecked()
    await expect(model).toHaveValue('gemma3:4b')
    await expect(temperature).toHaveValue('0.5')
  })

  await test.step('an edited prompt is saved and can be reset to the default', async () => {
    const shipped = await prompt.inputValue()
    await prompt.fill(shipped.replace('## user', '- Be brief.\n\n## user'))
    await save.click()
    await expect(page.getByText('Saved')).toBeVisible()

    await page.reload()
    await page.getByRole('button', { name: 'Advanced' }).click()
    await expect(prompt).toHaveValue(/- Be brief\./)
    await expect(page.getByText('Using the default prompt')).toHaveCount(0)

    await page.getByRole('button', { name: 'Reset to default' }).click()
    await expect(page.getByText('Using the default prompt')).toBeVisible()
    await expect(prompt).toHaveValue(shipped)
  })
})

test.describe(() => {
  // The daemon rejects the prompt, which the page explains.
  test.use({ expectedApiErrors: [{ method: 'PATCH', path: '/api/preferences', status: 400 }] })

  test('AI settings: a broken prompt is refused with the reason', async ({ page, daemon }) => {
    await page.goto('/ai')
    await page.getByRole('textbox', { name: 'Ollama URL' }).fill(daemon.ollamaUrl)
    await page.getByRole('switch', { name: 'Enable AI' }).click()
    await page.getByRole('switch', { name: 'Summaries enabled' }).click()
    await page.getByRole('button', { name: 'Advanced' }).click()
    const prompt = page.getByRole('textbox', { name: 'Prompt', exact: true })
    const shipped = await prompt.inputValue()
    await prompt.fill(shipped.replace('## map user', '## map usr'))
    await page.getByRole('button', { name: 'Save' }).click()

    await expect(page.getByRole('alert')).toHaveText('Prompt has an unknown section "map usr"')
    await expect(prompt).toHaveValue(/## map usr/)
  })
})
