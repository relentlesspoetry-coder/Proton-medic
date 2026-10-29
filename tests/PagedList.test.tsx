import { afterEach, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { PagedList } from '../src/PagedList';
afterEach(cleanup);
it('bounds mounted rows and makes every game reachable', () => {
  render(<PagedList items={Array.from({length:95},(_,i)=>i)} render={i=><p key={i}>Game {i}</p>}/>);
  expect(screen.getAllByText(/^Game /)).toHaveLength(40);
  expect(screen.queryByText('Game 40')).toBeNull();
  fireEvent.click(screen.getByText('Next')); expect(screen.getByText('Game 40')).toBeTruthy();
  fireEvent.click(screen.getByText('Next')); expect(screen.getAllByText(/^Game /)).toHaveLength(15);
  expect((screen.getByText('Next') as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByText('Previous')); expect(screen.getByText('Game 40')).toBeTruthy();
});
it('keeps remaining results visible after the library shrinks', () => {
  const row=(i:number)=><p key={i}>Game {i}</p>;
  const view=render(<PagedList items={Array.from({length:81},(_,i)=>i)} render={row}/>);
  fireEvent.click(screen.getByText('Next')); fireEvent.click(screen.getByText('Next'));
  view.rerender(<PagedList items={[1,2]} render={row}/>);
  expect(screen.getByText('Game 1')).toBeTruthy(); expect(screen.queryByText('Next')).toBeNull();
});
