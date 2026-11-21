/** Gameweave appearance is local-only; it never changes games, prefixes or Proton configurations. */
// Stable internal IDs also reference the original logo assets and previously saved selections.
export const THEMES = [
  { id:'carbon-green', label:'Carbon Green', colors:['#07110d','#42ef70','#c3ff44'] },
  { id:'cyan-purple', label:'Blue & Purple', colors:['#12142e','#6d9dff','#bd8aff'] },
  { id:'red-orange-gold', label:'Red & Orange', colors:['#230f15','#ff6b73','#ff9a55'] },
  { id:'teal-green-gold', label:'Teal & Green', colors:['#09201a','#42e4cc','#72ed8f'] },
] as const;
export type Theme = typeof THEMES[number]['id'];
export type Glass = 'subtle'|'balanced'|'strong';
export type Appearance = {theme:Theme; glass:Glass; reduceMotion:boolean};
export const KEY = 'gameweave.appearance.v1';
export const DEFAULT:Appearance = {theme:'carbon-green',glass:'balanced',reduceMotion:false};
export function normalizeAppearance(input:unknown):Appearance {
  if(!input||typeof input!=='object'||Array.isArray(input))return {...DEFAULT};
  const v=input as Record<string,unknown>;
  return {
    theme: THEMES.some(t=>t.id===v.theme)?v.theme as Theme:DEFAULT.theme,
    glass: v.glass==='subtle'||v.glass==='balanced'||v.glass==='strong'?v.glass:DEFAULT.glass,
    reduceMotion: typeof v.reduceMotion==='boolean'?v.reduceMotion:DEFAULT.reduceMotion,
  };
}
export function readAppearance(storage?:Pick<Storage,'getItem'>):Appearance {
  try {const raw=(storage??window.localStorage).getItem(KEY);return raw?normalizeAppearance(JSON.parse(raw) as unknown):{...DEFAULT};}
  catch{return {...DEFAULT};}
}
export function saveAppearance(value:Appearance,storage?:Pick<Storage,'setItem'>):boolean {
  try{(storage??window.localStorage).setItem(KEY,JSON.stringify(normalizeAppearance(value)));return true;}
  catch{return false;}
}
