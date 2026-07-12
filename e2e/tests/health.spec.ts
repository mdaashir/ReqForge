import { test, expect } from '@playwright/test'

/**
 * Basic smoke test: the app opens, shows the request builder UI,
 * and responds to user interaction.
 */
test('app loads and shows request builder', async ({ page }) => {
  await page.goto('/')

  // The page title should be set
  await expect(page).toHaveTitle(/ReqForge/)

  // The URL bar should be visible (core of the request builder)
  await expect(page.getByTestId('url-bar')).toBeVisible({ timeout: 10000 })

  // The send button should be clickable
  await expect(page.getByTestId('send-request')).toBeVisible()
})

test('can type a URL and method', async ({ page }) => {
  await page.goto('/')

  // Type a URL
  const urlInput = page.getByTestId('url-input')
  await urlInput.fill('https://jsonplaceholder.typicode.com/todos/1')

  // Change method
  await page.getByTestId('method-selector').click()
  await page.getByRole('option', { name: 'GET' }).click()

  // Hit send
  await page.getByTestId('send-request').click()

  // Wait for response panel to appear
  await expect(page.getByTestId('response-viewer')).toBeVisible({
    timeout: 15000,
  })

  // Check we got a 200 status
  await expect(page.getByTestId('response-status')).toContainText('200')
})

test('command palette opens and can search', async ({ page }) => {
  await page.goto('/')

  // Open command palette (Ctrl+K / Cmd+K)
  await page.keyboard.press('Control+k')

  // The palette should appear
  await expect(page.getByTestId('command-palette')).toBeVisible()

  // Type a search
  await page.getByTestId('command-input').fill('send')

  // Should show at least one result
  const results = page.getByTestId(/command-*/)
  await expect(results.first()).toBeVisible({ timeout: 3000 })
})

test('environment selector can be opened', async ({ page }) => {
  await page.goto('/')

  // Click the environment selector dropdown
  await page.getByTestId('environment-selector').click()

  // The dropdown should open
  await expect(page.getByTestId('environment-dropdown')).toBeVisible()
})
