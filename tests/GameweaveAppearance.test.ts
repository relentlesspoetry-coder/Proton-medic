import { expect, it } from 'vitest';
import { DEFAULT, KEY, THEMES, normalizeAppearance, readAppearance, saveAppearance } from '../src/gameweave/appearance';

it('contains all four approved Gameweave themes',()=>{
  expect(THEMES.map(x=>x.id)).toEqual([
    'carbon-green','cyan-purple','red-orange-gold','teal-green-gold']);
});
it('ignores malformed or invalid stored data',()=>{
  expect(normalizeAppearance(null)).toEqual(DEFAULT);
  expect(normalizeAppearance({theme:'not-valid',glass:'unsafe',reduceMotion:5})).toEqual(DEFAULT);
  expect(readAppearance({getItem:()=>'{bad JSON'})).toEqual(DEFAULT);
});
it('persists only visual preferences',()=>{
  const state=new Map<string,string>();
  const store={getItem:(k:string)=>state.get(k)??null,
    setItem:(k:string,v:string)=>{state.set(k,v);}};
  const value={theme:'cyan-purple' as const,glass:'strong' as const,reduceMotion:true};
  expect(saveAppearance(value,store)).toBe(true);
  expect(readAppearance(store)).toEqual(value);
  expect([...state.keys()]).toEqual([KEY]);
});
it('handles unavailable storage gracefully',()=>{
  expect(readAppearance({getItem:()=>{throw Error('disabled');}})).toEqual(DEFAULT);
  expect(saveAppearance(DEFAULT,{setItem:()=>{throw Error('disabled');}})).toBe(false);
});

it('uses the four approved names while keeping saved IDs stable',()=>{
  expect(THEMES.map(theme=>theme.label)).toEqual(['Carbon Green','Blue & Purple','Red & Orange','Teal & Green']);
});
