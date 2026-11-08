use crate::config::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub access: String,
    pub refresh: String,
    pub email: String,
    pub exp: u64, // 失効の UNIX 秒
}

fn entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new("valorant-matchfound", "lapount").map_err(|e| e.to_string())
}

// Windows の資格情報ストアは 1 件 約1280 文字までなので、長い access token は入れず、
// 短い refresh token とメールだけを保存する。access token はメモリに置き、起動後に refresh で取り直す
static CACHE: std::sync::Mutex<Option<Session>> = std::sync::Mutex::new(None);

pub fn load() -> Option<Session> {
    if let Some(s) = CACHE.lock().ok()?.clone() {
        return Some(s);
    }
    let v: serde_json::Value = serde_json::from_str(&entry().ok()?.get_password().ok()?).ok()?;
    let s = Session { access: String::new(), refresh: v["r"].as_str()?.to_string(), email: v["e"].as_str().unwrap_or("").to_string(), exp: 0 };
    *CACHE.lock().ok()? = Some(s.clone());
    Some(s)
}
pub fn save(s: &Session) {
    if let Ok(mut c) = CACHE.lock() {
        *c = Some(s.clone());
    }
    if let Ok(e) = entry() {
        let _ = e.set_password(&serde_json::json!({"r": s.refresh, "e": s.email}).to_string());
    }
}
pub fn clear() {
    if let Ok(mut c) = CACHE.lock() {
        *c = None;
    }
    if let Ok(e) = entry() {
        let _ = e.delete_credential();
    }
}

fn anon() -> String {
    std::env::var("MATCHFOUND_ANON_KEY").unwrap_or_else(|_| ANON_KEY.trim().to_string())
}
pub fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[derive(Deserialize)]
pub struct Device {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

pub async fn device_create(c: &reqwest::Client) -> Result<Device, String> {
    let k = anon();
    c.post(format!("{SUPABASE_URL}/functions/v1/oauth-device-code"))
        .header("apikey", &k).bearer_auth(&k)
        .json(&serde_json::json!({"action":"create","client_id":"lp_matchfound"}))
        .send().await.map_err(|e| e.to_string())?
        .json::<Device>().await.map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct Poll { status: String, access_token: Option<String>, refresh_token: Option<String> }

pub enum PollResult { Pending, Done(Session), Failed(String) }

pub async fn device_poll(c: &reqwest::Client, code: &str) -> PollResult {
    let k = anon();
    let r = c.post(format!("{SUPABASE_URL}/functions/v1/oauth-device-code"))
        .header("apikey", &k).bearer_auth(&k)
        .json(&serde_json::json!({"action":"poll","device_code":code}))
        .send().await;
    let Ok(r) = r else { return PollResult::Pending };
    let Ok(p) = r.json::<Poll>().await else { return PollResult::Pending };
    match p.status.as_str() {
        "pending" => PollResult::Pending,
        "denied" => PollResult::Failed("ログインが拒否されました".into()),
        "expired" | "invalid" => PollResult::Failed("コードの有効期限が切れました".into()),
        "already_used" => PollResult::Failed("このコードは使用済みです".into()),
        "authorized" => match (p.access_token, p.refresh_token) {
            (Some(a), Some(r)) => {
                let email = user_email(c, &a).await.unwrap_or_default();
                PollResult::Done(Session { exp: now() + 3000, access: a, refresh: r, email })
            }
            _ => PollResult::Failed("トークンを受け取れませんでした".into()),
        },
        s => PollResult::Failed(format!("予期しない状態です: {s}")),
    }
}

async fn user_email(c: &reqwest::Client, token: &str) -> Option<String> {
    let k = anon();
    let v: serde_json::Value = c.get(format!("{SUPABASE_URL}/auth/v1/user"))
        .header("apikey", &k).bearer_auth(token).send().await.ok()?.json().await.ok()?;
    v["email"].as_str().map(String::from)
}

#[derive(Deserialize)]
struct Tok { access_token: String, refresh_token: String, expires_in: u64 }

// 失効が近ければ更新して返す。refresh token は更新のたびに変わるので必ず保存し直す
pub async fn fresh(c: &reqwest::Client, force: bool) -> Result<Session, String> {
    let s = load().ok_or("ログインしていません")?;
    if !force && s.exp > now() + 120 {
        return Ok(s);
    }
    let k = anon();
    let r = c.post(format!("{SUPABASE_URL}/auth/v1/token?grant_type=refresh_token"))
        .header("apikey", &k)
        .json(&serde_json::json!({"refresh_token": s.refresh}))
        .send().await.map_err(|e| e.to_string())?;
    if r.status().is_client_error() {
        clear();
        return Err("ログインの有効期限が切れました。もう一度ログインしてください".into());
    }
    let t: Tok = r.json().await.map_err(|e| e.to_string())?;
    let n = Session { access: t.access_token, refresh: t.refresh_token, email: s.email, exp: now() + t.expires_in.saturating_sub(60) };
    save(&n);
    Ok(n)
}

// Lapount アカウントと連携 Discord の表示用情報。取れない項目は空のまま
pub async fn profile(c: &reqwest::Client) -> Option<serde_json::Value> {
    let sess = fresh(c, false).await.ok()?;
    let v: serde_json::Value = c.get(format!("{SUPABASE_URL}/auth/v1/user"))
        .header("apikey", anon()).bearer_auth(&sess.access).send().await.ok()?.json().await.ok()?;
    let s = |x: &serde_json::Value| x.as_str().unwrap_or("").to_string();
    let ids = v["identities"].as_array().cloned().unwrap_or_default();
    let d = ids.iter().find(|i| i["provider"] == "discord").map(|i| i["identity_data"].clone()).unwrap_or_default();
    let pick = |ks: &[&str]| ks.iter().map(|k| s(&d[*k])).find(|x| !x.is_empty()).unwrap_or_default();
    let discord_id = ids.iter().find(|i| i["provider"] == "discord").map(|i| s(&i["id"])).unwrap_or_default();
    let g = s(&d["custom_claims"]["global_name"]);
    let dname = if g.is_empty() { pick(&["full_name", "name", "user_name"]) } else { g };
    let p = pick(&["provider_id", "sub"]);
    let did = if p.is_empty() { discord_id } else { p };
    Some(serde_json::json!({
        "lapountId": s(&v["id"]),
        "lapountEmail": s(&v["email"]),
        "createdAt": s(&v["created_at"]),
        "lastSignIn": s(&v["last_sign_in_at"]),
        "discordName": dname,
        "discordUser": pick(&["user_name", "preferred_username", "name"]),
        "discordId": did,
        "discordEmail": pick(&["email"]),
        "avatar": pick(&["avatar_url", "picture"]),
    }))
}
