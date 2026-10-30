import { afterEach, expect, it, vi } from 'vitest';
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

it('moves focus and scroll to new results only for explicit page changes', () => {
  const scroll = vi.fn();
  const view = render(<PagedList items={Array.from({length:95},(_,i)=>i)} render={i=><button key={i}>Game {i}</button>}/>);
  const results=screen.getByRole('group',{name:'Game list results'});
  results.scrollIntoView=scroll;
  fireEvent.click(screen.getByText('Next'));
  expect(document.activeElement).toBe(results);
  expect(scroll).toHaveBeenCalledWith({block:'start',behavior:'auto'});
  fireEvent.click(screen.getByText('Previous'));
  expect(document.activeElement).toBe(results);
  expect(scroll).toHaveBeenCalledTimes(2);
  const first=screen.getByText('Game 0'); first.focus();
  view.rerender(<PagedList items={[0,1]} render={i=><button key={i}>Game {i}</button>}/>);
  expect(document.activeElement).toBe(first);
  expect(scroll).toHaveBeenCalledTimes(2);
});
it('commits a clamped page so removed pages do not reappear after growth', () => {
  const row=(i:number)=><p key={i}>Game {i}</p>;
  const all=Array.from({length:95},(_,i)=>i);
  const view=render(<PagedList items={all} render={row}/>);
  fireEvent.click(screen.getByText('Next')); fireEvent.click(screen.getByText('Next'));
  view.rerender(<PagedList items={[0,1]} render={row}/>);
  view.rerender(<PagedList items={all} render={row}/>);
  expect(screen.getByText('Page 1 of 3')).toBeTruthy();
});
