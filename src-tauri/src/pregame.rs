// エージェント選択中の情報(ローカル API の資格情報 + リモートのゲーム API)。形式は非公開のため、失敗しても通知本体には影響させない
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Clone, PartialEq, Debug, Default)]
pub struct Player { pub icon: String, pub name: String, pub agent: String, pub locked: bool, #[serde(skip)] pub me: bool }

#[derive(Default, Clone)]
pub struct Names { pub agents: HashMap<String, String>, pub agent_img: HashMap<String, String>, pub maps: HashMap<String, (String, String)>, pub version: String }

pub struct Info { pub ends_at: u64, pub map: String, pub map_img: String, pub agent_img: String, pub queue: String, pub players: Vec<Player> }

const PLATFORM: &str = "ew0KCSJwbGF0Zm9ybVR5cGUiOiAiUEMiLA0KCSJwbGF0Zm9ybU9TIjogIldpbmRvd3MiLA0KCSJwbGF0Zm9ybU9TVmVyc2lvbiI6ICIxMC4wLjE5MDQyLjEuMjU2LjY0Yml0IiwNCgkicGxhdGZvcm1DaGlwc2V0IjogIlVua25vd24iDQp9";

// agent / map の表示名と クライアント版を valorant-api.com から取る(失敗したら空のまま)
pub async fn load_names(http: &reqwest::Client) -> Names {
    let mut n = Names::default();
    let get = |u: &'static str| async move { http.get(u).send().await.ok()?.json::<Value>().await.ok() };
    if let Some(v) = get("https://valorant-api.com/v1/agents?isPlayableCharacter=true&language=ja-JP").await {
        for a in v["data"].as_array().into_iter().flatten() {
            if let (Some(id), Some(nm)) = (a["uuid"].as_str(), a["displayName"].as_str()) { n.agents.insert(id.to_lowercase(), nm.into()); n.agent_img.insert(id.to_lowercase(), a["displayIcon"].as_str().unwrap_or("").into()); }
        }
    }
    if let Some(v) = get("https://valorant-api.com/v1/maps?language=ja-JP").await {
        for m in v["data"].as_array().into_iter().flatten() {
            if let (Some(u), Some(nm)) = (m["mapUrl"].as_str(), m["displayName"].as_str()) { n.maps.insert(u.into(), (nm.into(), m["listViewIcon"].as_str().unwrap_or("").into())); }
        }
    }
    if let Some(v) = get("https://valorant-api.com/v1/version").await {
        n.version = v["data"]["riotClientVersion"].as_str().unwrap_or("").into();
    }
    n
}

pub fn mode_name(q: &str) -> String {
    match q {
        "unrated" => "アンレート", "competitive" => "コンペティティブ", "swiftplay" => "スウィフトプレイ",
        "spikerush" => "スパイクラッシュ", "deathmatch" => "デスマッチ", "ggteam" => "エスカレーション",
        "onefa" => "レプリケーション", "hurm" => "チームデスマッチ", "premier" => "プレミア", "snowball" => "スノーボールファイト",
        "custom" => "カスタム", "" => "", o => o,
    }.to_string()
}

// 自分の puuid・地域は presences / external-sessions から。pregame に入っていなければ None
pub async fn fetch(local: &reqwest::Client, remote: &reqwest::Client, names: &Names) -> Option<Info> {
    let (port, pw) = crate::riot::lockfile()?;
    let l = |p: &str| local.get(format!("https://127.0.0.1:{port}{p}")).basic_auth("riot", Some(&pw)).send();
    let ent: Value = l("/entitlements/v1/token").await.ok()?.json().await.ok()?;
    let (tok, ent_tok, puuid) = (ent["accessToken"].as_str()?, ent["token"].as_str()?, ent["subject"].as_str()?);
    let ses: Value = l("/product-session/v1/external-sessions").await.ok()?.json().await.ok()?;
    let mut region = String::new();
    for s in ses.as_object()?.values() {
        for a in s["launchConfiguration"]["arguments"].as_array().into_iter().flatten() {
            if let Some(r) = a.as_str().and_then(|a| a.strip_prefix("-ares-deployment=")) { region = r.to_string(); }
        }
    }
    if region.is_empty() { return None; }
    let shard = match region.as_str() { "latam" | "br" => "na", r => r };
    let glz = format!("https://glz-{region}-1.{shard}.a.pvp.net");
    let pd = format!("https://pd.{shard}.a.pvp.net");
    let rq = |u: String| remote.get(u).bearer_auth(tok).header("X-Riot-Entitlements-JWT", ent_tok)
        .header("X-Riot-ClientPlatform", PLATFORM).header("X-Riot-ClientVersion", names.version.as_str());
    let pl: Value = rq(format!("{glz}/pregame/v1/players/{puuid}")).send().await.ok()?.json().await.ok()?;
    let mid = pl["MatchID"].as_str()?;
    let m: Value = rq(format!("{glz}/pregame/v1/matches/{mid}")).send().await.ok()?.json().await.ok()?;
    let team = m["AllyTeam"]["Players"].as_array()?;
    let ids: Vec<&str> = team.iter().filter_map(|p| p["Subject"].as_str()).collect();
    // 名前は name-service で解決(失敗時は番号)
    let mut nm: HashMap<String, String> = HashMap::new();
    if let Ok(r) = remote.put(format!("{pd}/name-service/v2/players")).bearer_auth(tok)
        .header("X-Riot-Entitlements-JWT", ent_tok).header("X-Riot-ClientPlatform", PLATFORM)
        .header("X-Riot-ClientVersion", names.version.as_str()).json(&ids).send().await {
        if let Ok(v) = r.json::<Value>().await {
            for e in v.as_array().into_iter().flatten() {
                if let Some(s) = e["Subject"].as_str() {
                    nm.insert(s.into(), format!("{}#{}", e["GameName"].as_str().unwrap_or(""), e["TagLine"].as_str().unwrap_or("")));
                }
            }
        }
    }
    let players = team.iter().enumerate().map(|(i, p)| {
        let sub = p["Subject"].as_str().unwrap_or("");
        let cid = p["CharacterID"].as_str().unwrap_or("").to_lowercase();
        Player {
            icon: names.agent_img.get(&cid).cloned().unwrap_or_default(),
            name: nm.get(sub).filter(|s| s.len() > 1).cloned().unwrap_or_else(|| format!("プレイヤー{}", i + 1)),
            agent: if cid.is_empty() { String::new() } else { names.agents.get(&cid).cloned().unwrap_or_else(|| "不明".into()) },
            me: sub == puuid,
            locked: p["CharacterSelectionState"].as_str() == Some("locked"),
        }
    }).collect();
    let (map, map_img) = names.maps.get(m["MapID"].as_str().unwrap_or("")).cloned().unwrap_or_default();
    let agent_img = team.iter().find(|p| p["Subject"].as_str() == Some(puuid))
        .and_then(|p| names.agent_img.get(&p["CharacterID"].as_str().unwrap_or("").to_lowercase())).cloned().unwrap_or_default();
    let ends_at = m["PhaseTimeRemainingNS"].as_u64().map(|ns| crate::auth::now() + ns / 1_000_000_000).unwrap_or(0);
    Some(Info { ends_at, map, map_img, agent_img, queue: queue_of(&m), players })
}

// QueueID が空のことがあるので、Mode のパスや ProvisioningFlow からも推定する
fn queue_of(m: &Value) -> String {
    if let Some(q) = m["QueueID"].as_str().filter(|q| !q.is_empty()) { return q.to_lowercase(); }
    if m["ProvisioningFlow"].as_str() == Some("CustomGame") { return "custom".into(); }
    let mode = m["Mode"].as_str().unwrap_or("");
    for (k, q) in [("QuickBomb", "spikerush"), ("Deathmatch", "deathmatch"), ("GunGame", "ggteam"), ("Bomb", "unrated")] {
        if mode.contains(k) { return q.into(); }
    }
    String::new()
}
