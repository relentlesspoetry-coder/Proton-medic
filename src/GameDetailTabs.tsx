import { useId, useState, type ReactNode } from 'react';
type Tab = { id: string; label: string; content: ReactNode };
/** Panels stay mounted so launch observations and repair receipts survive tab changes. */
export function GameDetailTabs({ tabs }: { tabs: Tab[] }) {
  const prefix = useId();
  const [active, setActive] = useState(tabs[0].id);
  return <div className="pm-detail-workspace">
    <div className="pm-detail-tabs" role="tablist" aria-label="Game details">{tabs.map((tab, index) =>
      <button key={tab.id} type="button" role="tab" id={`${prefix}-${tab.id}-tab`} aria-selected={active === tab.id}
        aria-controls={`${prefix}-${tab.id}-panel`} tabIndex={active === tab.id ? 0 : -1}
        onClick={() => setActive(tab.id)} onKeyDown={event => {
          let next = index;
          if (event.key === 'ArrowRight') next = (index + 1) % tabs.length;
          else if (event.key === 'ArrowLeft') next = (index + tabs.length - 1) % tabs.length;
          else if (event.key === 'Home') next = 0;
          else if (event.key === 'End') next = tabs.length - 1;
          else return;
          event.preventDefault(); setActive(tabs[next].id);
          document.getElementById(`${prefix}-${tabs[next].id}-tab`)?.focus();
        }}>{tab.label}</button>
    )}</div>
    {tabs.map(tab => <section key={tab.id} className="pm-tab-panel" role="tabpanel" tabIndex={0}
      id={`${prefix}-${tab.id}-panel`} aria-labelledby={`${prefix}-${tab.id}-tab`} hidden={active !== tab.id}>{tab.content}</section>)}
  </div>;
}
