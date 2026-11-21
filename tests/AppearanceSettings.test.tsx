import { cleanup, fireEvent, render, screen, within } from '@testing-library/react';
import { useState } from 'react';
import { afterEach, expect, it } from 'vitest';
import { AppearanceSettings } from '../src/gameweave/AppearanceSettings';
import { DEFAULT } from '../src/gameweave/appearance';

afterEach(cleanup);

function Harness() {
  const [value, set] = useState(DEFAULT);
  return <AppearanceSettings value={value} onChange={set}/>;
}

it('uses the four real assets and allows keyboard theme changes', () => {
  render(<Harness/>);
  const options = within(screen.getByRole('radiogroup', { name: 'Color theme' })).getAllByRole('radio');
  expect(options.map(option => option.querySelector('img')?.getAttribute('src'))).toEqual([
    '/branding/carbon-green.webp', '/branding/cyan-purple.webp', '/branding/red-orange-gold.webp', '/branding/teal-green-gold.webp',
  ]);
  expect(options[0].getAttribute('aria-checked')).toBe('true');
  options[0].focus(); fireEvent.keyDown(options[0], { key: 'ArrowRight' });
  expect(document.activeElement).toBe(options[1]);
  expect(options[1].getAttribute('aria-checked')).toBe('true');
  expect(screen.getByText('Blue & Purple active')).toBeTruthy();
});

it('reflects live motion and glass preferences and resets them to defaults', () => {
  render(<Harness/>);
  const slider = screen.getByRole('slider', { name: 'Glass intensity' });
  const motion = screen.getByRole('checkbox', { name: /Reduce motion/ }) as HTMLInputElement;
  fireEvent.change(slider, { target: { value: '2' } });
  expect(slider.getAttribute('aria-valuetext')).toBe('Strong');
  expect(screen.getByText('Glass intensity: Strong')).toBeTruthy();
  fireEvent.click(motion); expect(motion.checked).toBe(true);
  fireEvent.click(screen.getByRole('radio', { name: 'Teal & Green' }));
  fireEvent.click(screen.getByRole('button', { name: 'Reset to Default' }));
  expect(slider.getAttribute('aria-valuetext')).toBe('Balanced');
  expect(motion.checked).toBe(false);
  expect(screen.getByText('Carbon Green active')).toBeTruthy();
});
