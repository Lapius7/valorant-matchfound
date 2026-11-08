use base64::Engine;
use serde_json::Value;

#[derive(Clone, PartialEq, Debug)]
pub enum Phase { Closed, Starting, Menus, Queue, Pregame, Ingame }

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Closed => "closed", Phase::Starting => "starting", Phase::Menus => "menus",
            Phase::Queue => "queue", Phase::Pregame => "pregame", Phase::Ingame => "ingame",
        }
    }
}

pub struct Probe { pub phase: Phase, pub mode: String, pub map: String, pub raw_keys: String }

pub fn lockfile() -> Option<(u16, String)> {
    let p = std::path::PathBuf::from(std::env::var("LOCALAPPDATA").ok()?)
        .join("Riot Games/Riot Client/Config/lockfile");
    let s = std::fs::read_to_string(p).ok()?;
    let f: Vec<&str> = s.trim().split(':').collect();
    Some((f.get(2)?.parse().ok()?, f.get(3)?.to_string()))
}

// Riot Client のローカル API(画面は読まない)。自分の presence だけを見る
pub async fn probe(c: &reqwest::Client) -> Probe {
    let none = |p: Phase, why: &str| Probe { phase: p, mode: String::new(), map: String::new(), raw_keys: why.to_string() };
    let Some((port, pw)) = lockfile() else { return none(Phase::Closed, "no lockfile") };
    let get = |path: &str| {
        c.get(format!("https://127.0.0.1:{port}{path}")).basic_auth("riot", Some(&pw)).send()
    };
    let sess = match get("/chat/v1/session").await { Ok(r) => r, Err(e) => return none(Phase::Closed, &format!("riot not reachable (stale lockfile): {e}")) };
    let st = sess.status();
    let Ok(sess) = sess.json::<Value>().await else { return none(Phase::Starting, &format!("session json (http {st})")) };
    let Some(puuid) = sess["puuid"].as_str() else { return none(Phase::Starting, "no puuid in session") };
    let pr = match get("/chat/v4/presences").await { Ok(r) => r, Err(e) => return none(Phase::Starting, &format!("presences request: {e}")) };
    let Ok(pr) = pr.json::<Value>().await else { return none(Phase::Starting, "presences json") };
    let me = pr["presences"].as_array().and_then(|a| {
        a.iter().find(|p| p["puuid"].as_str() == Some(puuid) && p["product"].as_str() == Some("valorant"))
    });
    let Some(me) = me else { return none(Phase::Starting, "no valorant presence for me") };
    let Some(b64) = me["private"].as_str() else { return none(Phase::Starting, "no private field") };
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(b64) else { return none(Phase::Starting, "base64") };
    let Ok(p) = serde_json::from_slice::<Value>(&bytes) else { return none(Phase::Starting, "private json") };

    let loop_state = p["matchPresenceData"]["sessionLoopState"].as_str().unwrap_or("");
    let party_state = p["partyPresenceData"]["partyState"].as_str().unwrap_or("");
    let phase = match loop_state {
        "PREGAME" => Phase::Pregame,
        "INGAME" => Phase::Ingame,
        _ if party_state == "MATCHMAKING" => Phase::Queue,
        _ => Phase::Menus,
    };
    let s = |v: &Value| v.as_str().unwrap_or("").to_string();
    Probe {
        phase,
        mode: { let a = s(&p["partyPresenceData"]["queueId"]); if a.is_empty() { s(&p["queueId"]) } else { a } },
        map: { let a = s(&p["matchPresenceData"]["matchMap"]); if a.is_empty() { s(&p["matchMap"]) } else { a } },
        raw_keys: format!("loop={loop_state} party={party_state}"),
    }
}
