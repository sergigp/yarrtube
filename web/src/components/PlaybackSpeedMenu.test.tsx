import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PlaybackSpeedMenu } from './PlaybackSpeedMenu'

describe('PlaybackSpeedMenu', () => {
  it('shows the current speed and marks it in the menu', async () => {
    render(<PlaybackSpeedMenu rate={1.25} onRateChange={() => {}} disabled={false} />)
    const trigger = screen.getByRole('button', { name: 'Playback speed' })

    await userEvent.setup().click(trigger)

    expect(trigger).toHaveTextContent('1.25x')
    expect(
      screen.getAllByRole('menuitemradio').map((item) => [
        item.textContent,
        item.getAttribute('aria-checked'),
      ]),
    ).toEqual([
      ['1x', 'false'],
      ['1.1x', 'false'],
      ['1.25x', 'true'],
      ['1.5x', 'false'],
      ['2x', 'false'],
    ])
  })

  it('reports the chosen speed', async () => {
    const onRateChange = vi.fn()
    render(<PlaybackSpeedMenu rate={1} onRateChange={onRateChange} disabled={false} />)

    const user = userEvent.setup()
    await user.click(screen.getByRole('button', { name: 'Playback speed' }))
    await user.click(await screen.findByRole('menuitemradio', { name: '1.5x' }))

    expect(onRateChange).toHaveBeenCalledExactlyOnceWith(1.5)
  })
})
