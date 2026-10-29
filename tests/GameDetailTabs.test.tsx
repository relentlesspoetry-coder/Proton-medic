import { useState } from 'react';
import { afterEach, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { GameDetailTabs } from '../src/GameDetailTabs';
afterEach(cleanup);
function Work() { const [value,setValue]=useState('');return <input aria-label="Saved work" value={value} onChange={event=>setValue(event.target.value)}/>; }
const tabs=[{id:'one',label:'Diagnostics',content:<Work/>},{id:'two',label:'Launch & repair',content:<p>Launch controls</p>}];
it('shows one panel while preserving work in hidden panels',()=>{
 render(<GameDetailTabs tabs={tabs}/>);fireEvent.change(screen.getByRole('textbox'),{target:{value:'unfinished repair'}});
 fireEvent.click(screen.getByRole('tab',{name:'Launch & repair'}));expect(screen.getAllByRole('tabpanel')).toHaveLength(1);expect(screen.queryByRole('textbox')).toBeNull();
 fireEvent.click(screen.getByRole('tab',{name:'Diagnostics'}));expect((screen.getByRole('textbox') as HTMLInputElement).value).toBe('unfinished repair');
});
it('supports arrow, Home and End navigation with linked tab panels',()=>{
 render(<GameDetailTabs tabs={tabs}/>);const first=screen.getByRole('tab',{name:'Diagnostics'});first.focus();
 fireEvent.keyDown(first,{key:'ArrowRight'});const second=screen.getByRole('tab',{name:'Launch & repair'});expect(document.activeElement).toBe(second);expect(second.getAttribute('aria-selected')).toBe('true');
 expect(screen.getByRole('tabpanel').getAttribute('aria-labelledby')).toBe(second.id);
 fireEvent.keyDown(second,{key:'Home'});expect(document.activeElement).toBe(first);
 fireEvent.keyDown(first,{key:'End'});expect(document.activeElement).toBe(second);
 fireEvent.keyDown(second,{key:'ArrowRight'});expect(document.activeElement).toBe(first);
});
