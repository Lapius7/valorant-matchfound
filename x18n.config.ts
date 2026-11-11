import { defineConfig } from "@x18n/cli";

export default defineConfig({
  sourceLocale: "ja",
  // 画面の t() は i18next と同じ呼び方なので、使用箇所はこれで拾える
  presets: ["i18next"],
  resources: [
    { pattern: "src/i18n/locales/{locale}.{ext}" },
    // Rust 側(src-tauri/src/i18n.rs)の文言。Rust は走査されないので、使うキーを下で指定する
    { pattern: "src-tauri/locales/{locale}.{ext}" },
  ],
  usage: {
    keysInUse: ["tray.*", "auth.*", "notify.*", "mode.*", "player", "update.*"],
  },
});
