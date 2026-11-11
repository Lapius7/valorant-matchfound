// 画面の言語。通知としてサーバーへ送る内容は日本語のまま変えず、アプリに出す文言だけを切り替える。
// 文言は src-tauri/locales/<言語>.json に置く(画面側は src/i18n/locales)
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

// (アプリの言語コード, valorant-api.com の言語コード, 文言)
const LANGS: &[(&str, &str, &str)] = &[
    ("ja", "ja-JP", include_str!("../locales/ja.json")),
    ("en", "en-US", include_str!("../locales/en.json")),
    ("ko", "ko-KR", include_str!("../locales/ko.json")),
    ("ru", "ru-RU", include_str!("../locales/ru.json")),
    ("it", "it-IT", include_str!("../locales/it.json")),
    ("de", "de-DE", include_str!("../locales/de.json")),
    ("fr", "fr-FR", include_str!("../locales/fr.json")),
    ("es", "es-ES", include_str!("../locales/es.json")),
    ("pt-BR", "pt-BR", include_str!("../locales/pt-BR.json")),
    ("zh-CN", "zh-CN", include_str!("../locales/zh-CN.json")),
    ("zh-TW", "zh-TW", include_str!("../locales/zh-TW.json")),
];
const JA: usize = 0;
const EN: usize = 1;

static CUR: AtomicUsize = AtomicUsize::new(JA);
static TABLES: OnceLock<Vec<Value>> = OnceLock::new();
// 言語ごとの、valorant-api.com の日本語名 → その言語の名前(エージェント・マップ・階級)
static NAMES: OnceLock<Mutex<HashMap<&'static str, HashMap<String, String>>>> = OnceLock::new();

fn tables() -> &'static Vec<Value> {
    TABLES.get_or_init(|| LANGS.iter().map(|(_, _, s)| serde_json::from_str(s).unwrap_or(Value::Null)).collect())
}
fn names() -> &'static Mutex<HashMap<&'static str, HashMap<String, String>>> { NAMES.get_or_init(Default::default) }

pub fn lang() -> &'static str { LANGS[CUR.load(Ordering::Relaxed)].0 }
pub fn api_lang(code: &str) -> Option<(&'static str, &'static str)> {
    LANGS.iter().find(|l| l.0 == code).map(|l| (l.0, l.1))
}
pub fn is_ja() -> bool { CUR.load(Ordering::Relaxed) == JA }

// 知らないコードは無視する。言語が変わったら true
pub fn set(code: &str) -> bool {
    let Some(i) = LANGS.iter().position(|l| l.0 == code) else { return false };
    CUR.swap(i, Ordering::Relaxed) != i
}

fn get(i: usize, key: &str) -> Option<&'static str> {
    key.split('.').try_fold(&tables()[i], |v, k| v.get(k))?.as_str()
}

// 今の言語の文言。無ければ英語、それも無ければキーを返す
pub fn t(key: &str) -> String {
    get(CUR.load(Ordering::Relaxed), key).or_else(|| get(EN, key)).unwrap_or(key).to_string()
}

pub fn has_names(code: &str) -> bool { names().lock().unwrap().contains_key(code) }
pub fn set_names(code: &'static str, m: HashMap<String, String>) { names().lock().unwrap().insert(code, m); }

// 内部で持っている日本語の表示名(モード・エージェント・マップ・階級)を、今の言語に直す
pub fn name(s: &str) -> String {
    if is_ja() || s.is_empty() { return s.into(); }
    if let Some((k, _)) = tables()[JA]["mode"].as_object().and_then(|m| m.iter().find(|(_, v)| v.as_str() == Some(s))) {
        return t(&format!("mode.{k}"));
    }
    names().lock().unwrap().get(lang()).and_then(|m| m.get(s).cloned()).unwrap_or_else(|| s.into())
}

// 名前を取れなかったプレイヤーの仮名(プレイヤーN)だけを直す。実在の名前は変えない
pub fn player(s: &str) -> String {
    match s.strip_prefix("プレイヤー") {
        Some(n) if !is_ja() && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => t("player").replace("{n}", n),
        _ => s.into(),
    }
}
