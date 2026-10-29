import { useState, type ReactNode } from 'react';
const PAGE_SIZE = 40;
export function PagedList<T>({ items, render, className = 'pm-problem-list' }: { items: T[]; render: (item: T) => ReactNode; className?: string }) {
  const [page, setPage] = useState(0);
  const pages = Math.max(1, Math.ceil(items.length / PAGE_SIZE));
  const current = Math.min(page, pages - 1);
  return <><div className={className}>{items.slice(current * PAGE_SIZE, (current + 1) * PAGE_SIZE).map(render)}</div>
    {pages > 1 && <nav className="pm-pagination" aria-label="Game list pages">
      <button className="pm-button pm-button-secondary" disabled={current === 0} onClick={() => setPage(current - 1)}>Previous</button>
      <span role="status">Page {current + 1} of {pages}</span>
      <button className="pm-button pm-button-secondary" disabled={current + 1 >= pages} onClick={() => setPage(current + 1)}>Next</button>
    </nav>}</>;
}
