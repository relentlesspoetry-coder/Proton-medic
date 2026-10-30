import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
const PAGE_SIZE = 40;
export function PagedList<T>({ items, render, className = 'pm-problem-list', page: controlledPage, onPageChange, clampPage = true }: {
  items: T[]; render: (item: T) => ReactNode; className?: string;
  page?: number; onPageChange?: (page: number) => void; clampPage?: boolean;
}) {
  const [localPage, setLocalPage] = useState(0);
  const page = controlledPage ?? localPage;
  const setPage = onPageChange ?? setLocalPage;
  const pages = Math.max(1, Math.ceil(items.length / PAGE_SIZE));
  const current = Math.max(0, Math.min(page, pages - 1));
  const results = useRef<HTMLDivElement>(null);
  const focusResults = useRef(false);
  useEffect(() => {
    if (clampPage && page !== current) setPage(current);
  }, [clampPage, page, current, setPage]);
  useLayoutEffect(() => {
    if (!focusResults.current) return;
    focusResults.current = false;
    results.current?.focus({ preventScroll: true });
    results.current?.scrollIntoView?.({ block: 'start', behavior: 'auto' });
  }, [current]);
  const changePage = (next: number) => { focusResults.current = true; setPage(next); };
  return <><div ref={results} className={className} role="group" aria-label="Game list results" tabIndex={-1}>{items.slice(current * PAGE_SIZE, (current + 1) * PAGE_SIZE).map(render)}</div>
    {pages > 1 && <nav className="pm-pagination" aria-label="Game list pages">
      <button type="button" className="pm-button pm-button-secondary" disabled={current === 0} onClick={() => changePage(current - 1)}>Previous</button>
      <span role="status">Page {current + 1} of {pages}</span>
      <button type="button" className="pm-button pm-button-secondary" disabled={current + 1 >= pages} onClick={() => changePage(current + 1)}>Next</button>
    </nav>}</>;
}
