import { useState, useCallback } from "react";
import { getLang, setLang, t, type Locale } from "./index";

export function useI18n() {
  const [lang, setLangState] = useState<Locale>(getLang);
  const changeLang = useCallback((next: Locale) => {
    setLang(next);
    setLangState(next);
  }, []);
  return { lang, setLang: changeLang, t };
}
