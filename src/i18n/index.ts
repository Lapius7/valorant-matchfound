import ja from "./locales/ja.json";
import en from "./locales/en.json";

export type Locale = "ja" | "en";

const LOCALES: Record<Locale, Record<string, unknown>> = { ja, en };
const STORAGE_KEY = "valorant-matchfound:lang";

let current: Locale = loadStored();

function loadStored(): Locale {
  if (typeof localStorage === "undefined") return "ja";
  const v = localStorage.getItem(STORAGE_KEY);
  return v === "ja" || v === "en" ? v : "ja";
}

export function getLang(): Locale {
  return current;
}

export function setLang(lang: Locale) {
  current = lang;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(STORAGE_KEY, lang);
  }
}

function getNested(obj: Record<string, unknown>, path: string): unknown {
  return path.split(".").reduce<any>((o, k) => (o && typeof o === "object" ? (o as Record<string, unknown>)[k] : undefined), obj);
}

export function t(key: string, params: Record<string, string | number> = {}): string {
  let val: unknown = getNested(LOCALES[current], key);
  if (val === undefined) val = getNested(LOCALES.ja, key);
  if (typeof val !== "string") return key;
  let str = val;
  for (const [k, v] of Object.entries(params)) {
    str = str.replace(new RegExp(`\\{${k}\\}`, "g"), String(v));
  }
  return str;
}
