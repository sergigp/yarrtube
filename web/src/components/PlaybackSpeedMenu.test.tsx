import { describe, expect, it } from 'vitest'
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
})
