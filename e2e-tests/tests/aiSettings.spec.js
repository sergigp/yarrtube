import { test, expect } from '../helpers/fixtures.js'
import { MODELS } from '../anthropic-stub.js'
import { OLLAMA_VERSION } from '../ollama-stub.js'

test('AI settings: point at Ollama, pick a model and save, then reset the prompt', async ({ page, daemon }) => {
  const enableAi = page.getByRole('switch', { name: 'AI features' })
  const model = page.getByRole('combobox', { name: 'Model', exact: true })
  const temperature = page.getByRole('spinbutton', { name: 'Temperature', exact: true })
  const prompt = page.getByRole('textbox', { name: 'Prompt', exact: true })
  const save = page.getByRole('button', { name: 'Save changes' })
  const saved = page.getByRole('status').getByText('Saved', { exact: true })
  const summaries = page.getByRole('group', { name: 'Summaries', exact: true })
  const promptToggle = summaries.getByRole('button', { name: /^Prompt/ })

  await test.step('the settings page opens the AI section', async () => {
    await page.goto('/')
    await page.getByRole('banner').getByRole('button', { name: 'Settings' }).click()
    await page.getByRole('menuitem', { name: 'Settings', exact: true }).click()
    await page
      .getByRole('navigation', { name: 'Settings sections' })
      .getByRole('link', { name: 'AI', exact: true })
      .click()
    await expect(page).toHaveURL(/\/settings\/ai$/)
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
    await page.getByRole('switch', { name: 'Summaries', exact: true }).click()
    await page.getByRole('button', { name: 'Advanced', exact: true }).click()
    await expect(promptToggle).toHaveText('Prompt · default')
    await model.selectOption('gemma3:4b')
    await expect(page.getByText("gemma3:4b doesn't support thinking")).toBeVisible()
    await temperature.fill('0.5')
    await expect(page.getByRole('region', { name: 'Unsaved changes' })).toContainText(
      '5 unsaved changes',
    )
    await save.click()
    await expect(saved).toBeVisible()
    await expect(save).toHaveCount(0)

    await page.reload()
    await page.getByRole('button', { name: 'Advanced', exact: true }).click()
    await promptToggle.click()
    await expect(enableAi).toBeChecked()
    await expect(page.getByRole('switch', { name: 'Summaries', exact: true })).toBeChecked()
    await expect(model).toHaveValue('gemma3:4b')
    await expect(temperature).toHaveValue('0.5')
  })

  await test.step('an edited prompt is saved and can be reset to the default', async () => {
    const shipped = await prompt.inputValue()
    await prompt.fill(shipped.replace('## user', '- Be brief.\n\n## user'))
    await save.click()
    await expect(saved).toBeVisible()

    await page.reload()
    await page.getByRole('button', { name: 'Advanced', exact: true }).click()
    await promptToggle.click()
    await expect(prompt).toHaveValue(/- Be brief\./)
    await expect(promptToggle).toHaveText('Prompt · edited')

    await page.getByRole('button', { name: 'Reset to default' }).click()
    await expect(promptToggle).toHaveText('Prompt · default')
    await expect(prompt).toHaveValue(shipped)
  })

  await test.step('categorization has its own generation settings', async () => {
    const contextLength = page.getByRole('spinbutton', { name: 'Categorization context length' })
    const summaryTemperature = await temperature.inputValue()
    await page.getByRole('switch', { name: 'Categorization', exact: true }).click()
    await page.getByRole('button', { name: 'Advanced categorization settings' }).click()
    const auto = page.getByRole('switch', { name: 'Auto context length' })
    const reopen = async () => {
      await page.reload()
      await page.getByRole('button', { name: 'Advanced', exact: true }).click()
      await page.getByRole('button', { name: 'Advanced categorization settings' }).click()
    }
    // Auto, with no category to size it from, is the 4,096 floor.
    await expect(auto).toBeChecked()
    await expect(contextLength).toHaveValue('4096')
    await expect(contextLength).not.toBeEditable()
    await expect(page.getByRole('spinbutton', { name: 'Categorization temperature' })).toHaveValue('0')
    await expect(page.getByRole('switch', { name: 'Categorization thinking' })).toBeChecked()
    await auto.click()
    await contextLength.fill('8192')
    await save.click()
    await expect(saved).toBeVisible()

    await reopen()
    await expect(auto).not.toBeChecked()
    await expect(contextLength).toHaveValue('8192')
    await expect(temperature).toHaveValue(summaryTemperature)

    await auto.click()
    await save.click()
    await expect(saved).toBeVisible()
    await reopen()
    await expect(auto).toBeChecked()
    await expect(contextLength).toHaveValue('4096')
  })
})

test.describe(() => {
  // The daemon rejects the prompt, which the page explains.
  test.use({ expectedApiErrors: [{ method: 'PATCH', path: '/api/preferences', status: 400 }] })

  test('AI settings: a broken prompt is refused with the reason', async ({ page, daemon }) => {
    await page.goto('/settings/ai')
    await page.getByRole('textbox', { name: 'Ollama URL' }).fill(daemon.ollamaUrl)
    await page.getByRole('switch', { name: 'AI features' }).click()
    await page.getByRole('switch', { name: 'Summaries', exact: true }).click()
    await page
      .getByRole('group', { name: 'Summaries', exact: true })
      .getByRole('button', { name: /^Prompt/ })
      .click()
    const prompt = page.getByRole('textbox', { name: 'Prompt', exact: true })
    const shipped = await prompt.inputValue()
    await prompt.fill(shipped.replace('## map user', '## map usr'))
    await page.getByRole('button', { name: 'Save changes' }).click()

    await expect(page.getByRole('alert')).toHaveText('Prompt has an unknown section "map usr"')
    await expect(prompt).toHaveValue(/## map usr/)
  })
})

test('AI settings: test the Anthropic connection and save it as the provider', async ({ page }) => {
  const anthropic = page.getByRole('radio', { name: 'Anthropic' })
  const model = page.getByRole('combobox', { name: 'Model', exact: true })

  await page.goto('/settings/ai')
  await anthropic.check()
  await expect(page.getByRole('textbox', { name: 'Ollama URL' })).toHaveCount(0)
  await page.getByRole('button', { name: 'Test connection' }).click()
  await expect(page.getByText(`Connected to Anthropic, ${MODELS.length} models`)).toBeVisible()
  await expect(model.locator('option')).toHaveText(MODELS.map((listed) => listed.id))
  await page.getByRole('button', { name: 'Save changes' }).click()
  await expect(page.getByRole('status').getByText('Saved', { exact: true })).toBeVisible()

  await page.reload()
  await expect(anthropic).toBeChecked()
  await expect(model).toHaveValue('claude-haiku-5-5')
})

