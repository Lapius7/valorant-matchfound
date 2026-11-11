import ja from "./locales/ja.json";
import en from "./locales/en.json";
import ko from "./locales/ko.json";
import ru from "./locales/ru.json";
import it from "./locales/it.json";
import de from "./locales/de.json";
import fr from "./locales/fr.json";
import es from "./locales/es.json";
import ptBR from "./locales/pt-BR.json";
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";

const LOCALES = { ja, en, ko, ru, it, de, fr, es, "pt-BR": ptBR, "zh-CN": zhCN, "zh-TW": zhTW } as Record<string, Record<string, unknown>>;
export type Locale = "ja" | "en" | "ko" | "ru" | "it" | "de" | "fr" | "es" | "pt-BR" | "zh-CN" | "zh-TW";

// 選択肢に出す名前(それぞれの言語で書く)
export const LOCALE_NAMES: [Locale, string][] = [
  ["ja", "日本語"], ["en", "English"], ["ko", "한국어"], ["zh-CN", "简体中文"], ["zh-TW", "繁體中文"],
  ["ru", "Русский"], ["de", "Deutsch"], ["fr", "Français"], ["it", "Italiano"], ["es", "Español"], ["pt-BR", "Português (Brasil)"],
];

const STORAGE_KEY = "valorant-matchfound:lang";

export const isLocale = (v: unknown): v is Locale => typeof v === "string" && v in LOCALES;

// OS の言語から近いものを選ぶ。どれにも当たらなければ英語
export function detect(tag: string): Locale {
  if (isLocale(tag)) return tag;
  const [base, region = ""] = tag.split("-");
  if (base === "zh") return ["TW", "HK", "MO"].includes(region.toUpperCase()) || /hant/i.test(tag) ? "zh-TW" : "zh-CN";
  if (base === "pt") return "pt-BR";
  return isLocale(base) ? base : "en";
}

let current: Locale = "ja";

export function storedLang(): Locale | null {
  if (typeof localStorage === "undefined") return null;
  const v = localStorage.getItem(STORAGE_KEY);
  return isLocale(v) ? v : null;
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
  if (val === undefined) val = getNested(LOCALES.en, key);
  if (typeof val !== "string") return key;
  let str = val;
  for (const [k, v] of Object.entries(params)) {
    str = str.replace(new RegExp(`\\{${k}\\}`, "g"), String(v));
  }
  return str;
}
