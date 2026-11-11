import "./index.css";
import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import App from "./App";
import { detect, isLocale, setLang, storedLang } from "./i18n/index";

// 言語は、アプリで選んだもの → インストーラーで選んだもの → OS の言語 の順に決める
async function boot() {
  let lang = storedLang();
  if (!lang) {
    const fromInstaller = await invoke<string | null>("get_lang").catch(() => null);
    lang = isLocale(fromInstaller) ? fromInstaller : detect(navigator.language);
  }
  setLang(lang);
  document.documentElement.lang = lang;
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}
boot();
