// エージェント選択中の情報(ローカル API の資格情報 + リモートのゲーム API)。形式は非公開のため、失敗しても通知本体には影響させない
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Clone, PartialEq, Debug, Default)]
pub struct Player { pub icon: String, pub name: String, pub agent: String, pub locked: bool, #[serde(skip)] pub me: bool }

#[derive(Default, Clone)]
pub struct Names { pub agents: HashMap<String, String>, pub agent_img: HashMap<String, String>, pub maps: HashMap<String, (String, String)>, pub tiers: HashMap<u64, String>, pub version: String }

pub struct Info { pub puuid: String, pub match_id: String, pub ends_at: u64, pub map: String, pub map_img: String, pub agent_img: String, pub queue: String, pub players: Vec<Player> }

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
    if let Some(v) = get("https://valorant-api.com/v1/competitivetiers?language=ja-JP").await {
        // 一番新しいエピソードの階級表を使う
        if let Some(t) = v["data"].as_array().and_then(|a| a.last()) {
            for e in t["tiers"].as_array().into_iter().flatten() {
                if let (Some(tn), Some(nm)) = (e["tier"].as_u64(), e["tierName"].as_str()) { n.tiers.insert(tn, nm.into()); }
            }
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
    Some(Info { puuid: puuid.to_string(), match_id: mid.to_string(), ends_at, map, map_img, agent_img, queue: queue_of(&m), players })
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

pub enum Res { Ok(Value), Retry, Stop }

struct Creds { tok: String, ent: String, puuid: String, pd: String }
async fn creds(local: &reqwest::Client) -> Option<Creds> {
    let (port, pw) = crate::riot::lockfile()?;
    let l = |p: &str| local.get(format!("https://127.0.0.1:{port}{p}")).basic_auth("riot", Some(&pw)).send();
    let ent: Value = l("/entitlements/v1/token").await.ok()?.json().await.ok()?;
    let ses: Value = l("/product-session/v1/external-sessions").await.ok()?.json().await.ok()?;
    let mut region = String::new();
    for s in ses.as_object()?.values() {
        for a in s["launchConfiguration"]["arguments"].as_array().into_iter().flatten() {
            if let Some(r) = a.as_str().and_then(|a| a.strip_prefix("-ares-deployment=")) { region = r.to_string(); }
        }
    }
    if region.is_empty() { return None; }
    let shard = match region.as_str() { "latam" | "br" => "na", r => r };
    Some(Creds { tok: ent["accessToken"].as_str()?.into(), ent: ent["token"].as_str()?.into(), puuid: ent["subject"].as_str()?.into(), pd: format!("https://pd.{shard}.a.pvp.net") })
}

// 試合終了後のスコアボード(match-details)。1 試合につき呼ぶのは数回まで。429(取りすぎ)なら Stop を返し、呼び出し側は諦める
pub async fn result(local: &reqwest::Client, remote: &reqwest::Client, names: &Names, mid: &str) -> Res {
    let Some(c) = creds(local).await else { return Res::Retry };
    let Ok(r) = remote.get(format!("{}/match-details/v1/matches/{mid}", c.pd)).bearer_auth(&c.tok).header("X-Riot-Entitlements-JWT", &c.ent)
        .header("X-Riot-ClientPlatform", PLATFORM).header("X-Riot-ClientVersion", names.version.as_str()).send().await else { return Res::Retry };
    if r.status().as_u16() == 429 { return Res::Stop; }
    if !r.status().is_success() { return Res::Retry; }
    let Ok(d) = r.json::<Value>().await else { return Res::Retry };
    let Some(raw) = d["players"].as_array().filter(|a| !a.is_empty()) else { return Res::Retry };
    let rounds_list = d["roundResults"].as_array().cloned().unwrap_or_default();
    let rounds = if rounds_list.is_empty() { d["teams"].as_array().into_iter().flatten().map(|t| t["roundsWon"].as_u64().unwrap_or(0)).sum::<u64>() } else { rounds_list.len() as u64 };
    // ヘッドショット率(頭 / (頭+胴+脚))
    let mut shots: HashMap<String, (u64, u64)> = HashMap::new();
    for rr in &rounds_list {
        for ps in rr["playerStats"].as_array().into_iter().flatten() {
            let Some(sub) = ps["subject"].as_str() else { continue };
            let e = shots.entry(sub.to_string()).or_default();
            for dm in ps["damage"].as_array().into_iter().flatten() {
                let (h, b, l) = (dm["headshots"].as_u64().unwrap_or(0), dm["bodyshots"].as_u64().unwrap_or(0), dm["legshots"].as_u64().unwrap_or(0));
                e.0 += h; e.1 += h + b + l;
            }
        }
    }
    let top = raw.iter().map(|p| p["stats"]["score"].as_u64().unwrap_or(0)).max().unwrap_or(0);
    let mut players: Vec<Value> = raw.iter().enumerate().map(|(i, p)| {
        let cid = p["characterId"].as_str().unwrap_or("").to_lowercase();
        let gn = p["gameName"].as_str().unwrap_or("");
        let st = &p["stats"];
        let sub = p["subject"].as_str().unwrap_or("");
        let score = st["score"].as_u64().unwrap_or(0);
        let (hs, tot) = shots.get(sub).copied().unwrap_or((0, 0));
        serde_json::json!({
            "name": if gn.is_empty() { format!("プレイヤー{}", i + 1) } else { format!("{}#{}", gn, p["tagLine"].as_str().unwrap_or("")) },
            "agent": names.agents.get(&cid).cloned().unwrap_or_default(),
            "team": p["teamId"].as_str().unwrap_or(""), "sub": sub,
            "k": st["kills"].as_u64().unwrap_or(0), "d": st["deaths"].as_u64().unwrap_or(0), "a": st["assists"].as_u64().unwrap_or(0),
            "score": score, "acs": if rounds > 0 { score / rounds } else { 0 },
            "hs": if tot > 0 { hs * 100 / tot } else { 0 },
            "mvp": top > 0 && score == top, "me": sub == c.puuid,
        })
    }).collect();
    players.sort_by(|a, b| b["score"].as_u64().cmp(&a["score"].as_u64()));
    let my_team = players.iter().find(|p| p["me"] == true).and_then(|p| p["team"].as_str()).unwrap_or("").to_string();
    let teams: Vec<Value> = d["teams"].as_array().into_iter().flatten().map(|t| serde_json::json!({
        "id": t["teamId"].as_str().unwrap_or(""), "won": t["won"].as_bool().unwrap_or(false),
        "rounds": t["roundsWon"].as_u64().unwrap_or(0), "mine": t["teamId"].as_str() == Some(my_team.as_str()),
    })).collect();
    Res::Ok(serde_json::json!({"teams": teams, "players": players, "rounds": rounds}))
}

// コンペの RR の増減。反映が遅れることがあるので、終了した試合の MatchID と一致したときだけ採用する
pub async fn rank_update(local: &reqwest::Client, remote: &reqwest::Client, names: &Names, mid: &str) -> Res {
    let Some(c) = creds(local).await else { return Res::Retry };
    let Ok(r) = remote.get(format!("{}/mmr/v1/players/{}/competitiveupdates?queue=competitive&startIndex=0&endIndex=1", c.pd, c.puuid)).bearer_auth(&c.tok)
        .header("X-Riot-Entitlements-JWT", &c.ent).header("X-Riot-ClientPlatform", PLATFORM).header("X-Riot-ClientVersion", names.version.as_str()).send().await else { return Res::Retry };
    if r.status().as_u16() == 429 { return Res::Stop; }
    if !r.status().is_success() { return Res::Retry; }
    let Ok(d) = r.json::<Value>().await else { return Res::Retry };
    let Some(m) = d["Matches"].as_array().and_then(|a| a.first()) else { return Res::Retry };
    if m["MatchID"].as_str() != Some(mid) { return Res::Retry; }
    let tn = |k: &str| names.tiers.get(&m[k].as_u64().unwrap_or(0)).cloned().unwrap_or_default();
    Res::Ok(serde_json::json!({
        "name": tn("TierAfterUpdate"), "before": tn("TierBeforeUpdate"),
        "rr": m["RankedRatingAfterUpdate"].as_i64().unwrap_or(0), "delta": m["RankedRatingEarned"].as_i64().unwrap_or(0),
    }))
}
