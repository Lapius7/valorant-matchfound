mod auth;
mod config;
mod i18n;
mod pregame;
mod riot;
mod update;

use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};
use tauri_plugin_autostart::ManagerExt;

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    riot: String,
    logged_in: bool,
    email: String,
    login_code: Option<String>,
    login_error: Option<String>,
    last_notify: Option<Notify>,
    match_info: Option<serde_json::Value>,
    party: Vec<serde_json::Value>,
    update: Option<serde_json::Value>,
    share: Option<String>,
    profile: Option<serde_json::Value>,
    bot_missing: bool,
    setup_done: bool,
}

#[derive(Serialize, Clone)]
struct Notify { at: u64, ok: bool, message: String, #[serde(rename = "dmClosed")] dm_closed: bool }

struct Shared {
    snap: Mutex<Snapshot>,
    http: reqwest::Client,
    riot_http: reqwest::Client,
    remote_http: reqwest::Client,
    last_shared: Mutex<String>,
}
type St = Arc<Shared>;

fn push(app: &AppHandle, st: &Shared, f: impl FnOnce(&mut Snapshot)) {
    let mut s = st.snap.lock().unwrap();
    f(&mut s);
    let _ = app.emit("state", localized(&s));
}

fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[tauri::command]
fn get_state(st: State<'_, St>) -> Snapshot { localized(&st.snap.lock().unwrap()) }

fn localized(s: &Snapshot) -> Snapshot {
    let mut s = s.clone();
    if i18n::is_ja() { return s; }
    let tr_field = |v: &mut serde_json::Value, k: &str| {
        if let Some(t) = v[k].as_str().map(i18n::name) { v[k] = t.into(); }
    };
    if let Some(m) = s.match_info.as_mut() {
        tr_field(m, "mode");
        tr_field(m, "map");
        if let Some(ps) = m["players"].as_array_mut() {
            for p in ps { tr_field(p, "agent"); if let Some(n) = p["name"].as_str().map(i18n::player) { p["name"] = n.into(); } }
        }
    }
    for p in s.party.iter_mut() { tr_field(p, "rank"); }
    s
}

// トレイのメニュー(項目, 文言のキー)。言語を変えたら書き換える
struct TrayItems(Vec<(MenuItem<tauri::Wry>, &'static str)>);

fn lang_file(app: &AppHandle) -> Option<std::path::PathBuf> { app.path().app_data_dir().ok().map(|d| d.join("lang")) }

// 保存してある言語(アプリかインストーラーが書いたもの)
#[tauri::command]
fn get_lang(app: AppHandle) -> Option<String> {
    let l = std::fs::read_to_string(lang_file(&app)?).ok()?;
    i18n::api_lang(l.trim()).map(|(c, _)| c.to_string())
}

// 画面の言語を受け取り、保存してトレイと状態の表示を合わせる
#[tauri::command]
fn set_lang(app: AppHandle, st: State<'_, St>, lang: String) {
    if i18n::api_lang(&lang).is_none() { return; }
    let changed = i18n::set(&lang);
    if let Some(f) = lang_file(&app) { let _ = std::fs::create_dir_all(f.parent().unwrap()); let _ = std::fs::write(f, &lang); }
    if let Some(t) = app.try_state::<TrayItems>() {
        for (item, key) in &t.0 { let _ = item.set_text(i18n::t(key)); }
    }
    // 前の言語で作ったメッセージは消す(DM 拒否のバナーは bot_missing で別に出るので残る)
    push(&app, &st, |s| {
        if !changed { return; }
        s.login_error = None;
        s.last_notify = None;
        if s.update.as_ref().and_then(|u| u["state"].as_str()) == Some("error") { s.update = None; }
    });
    // その言語のエージェント・マップ・階級名がまだ無ければ取りに行き、取れたら表示し直す
    let (a, st) = (app.clone(), st.inner().clone());
    tauri::async_runtime::spawn(async move {
        if pregame::load_local(&st.http, &lang).await { push(&a, &st, |_| {}); }
    });
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool { app.autolaunch().is_enabled().unwrap_or(false) }

fn boot_flag(app: &AppHandle) -> Option<std::path::PathBuf> { app.path().app_data_dir().ok().map(|d| d.join("show_on_boot")) }
#[tauri::command]
fn get_boot_show(app: AppHandle) -> bool { boot_flag(&app).map(|f| f.exists()).unwrap_or(false) }
#[tauri::command]
fn set_boot_show(app: AppHandle, on: bool) {
    let Some(f) = boot_flag(&app) else { return };
    if on { let _ = std::fs::create_dir_all(f.parent().unwrap()); let _ = std::fs::write(f, "1"); } else { let _ = std::fs::remove_file(f); }
}

fn queue_flag(app: &AppHandle) -> Option<std::path::PathBuf> { app.path().app_data_dir().ok().map(|d| d.join("queue_notify")) }
#[tauri::command]
fn get_queue_notify(app: AppHandle) -> bool { queue_flag(&app).map(|f| f.exists()).unwrap_or(false) }
#[tauri::command]
fn set_queue_notify(app: AppHandle, on: bool) {
    let Some(f) = queue_flag(&app) else { return };
    if on { let _ = std::fs::create_dir_all(f.parent().unwrap()); let _ = std::fs::write(f, "1"); } else { let _ = std::fs::remove_file(f); }
}

#[tauri::command]
fn set_autostart(app: AppHandle, on: bool) -> Result<(), String> {
    let a = app.autolaunch();
    (if on { a.enable() } else { a.disable() }).map_err(|e| e.to_string())
}

#[tauri::command]
async fn login(app: AppHandle, st: State<'_, St>) -> Result<(), String> {
    let st = st.inner().clone();
    let d = auth::device_create(&st.http).await?;
    push(&app, &st, |s| { s.login_code = Some(d.user_code.clone()); s.login_error = None; });
    let _ = tauri_plugin_opener::open_url(&d.verification_uri_complete, None::<&str>);
    tauri::async_runtime::spawn(async move {
        let end = auth::now() + d.expires_in;
        while auth::now() < end {
            tokio::time::sleep(std::time::Duration::from_secs(d.interval.max(2))).await;
            if st.snap.lock().unwrap().login_code.as_deref() != Some(d.user_code.as_str()) { return; }
            match auth::device_poll(&st.http, &d.device_code).await {
                auth::PollResult::Pending => continue,
                auth::PollResult::Done(sess) => {
                    auth::save(&sess);
                    push(&app, &st, |s| { s.logged_in = true; s.email = sess.email.clone(); s.login_code = None; });
                    let _ = share_call(&app, &st, "status").await; load_profile(&app, &st).await;
                    return;
                }
                auth::PollResult::Failed(m) => {
                    push(&app, &st, |s| { s.login_code = None; s.login_error = Some(m); });
                    return;
                }
            }
        }
        push(&app, &st, |s| { s.login_code = None; s.login_error = Some(i18n::t("auth.expired")); });
    });
    Ok(())
}

#[tauri::command]
fn cancel_login(app: AppHandle, st: State<'_, St>) {
    push(&app, &st, |s| { s.login_code = None; s.login_error = None; });
}

#[tauri::command]
async fn refresh_profile(app: AppHandle, st: State<'_, St>) -> Result<(), String> { let st = st.inner().clone(); load_profile(&app, &st).await; Ok(()) }

#[tauri::command]
fn logout(app: AppHandle, st: State<'_, St>) {
    auth::clear();
    push(&app, &st, |s| { s.logged_in = false; s.email.clear(); s.share = None; s.profile = None; });
}

async fn notify(app: &AppHandle, st: &Shared, body: serde_json::Value) -> Option<String> {
    let result = send(st, body).await;
    let (ok, message, id) = match result {
        Ok(v) => (true, if v["skipped"].is_string() { i18n::t("notify.skipped") } else { i18n::t("notify.sent") }, v["messageId"].as_str().map(String::from)),
        Err(m) => (false, m, None),
    };
    push(app, st, |s| { let closed = !ok && message == dm_closed_msg(); s.bot_missing = closed; s.last_notify = Some(Notify { at: auth::now(), ok, message, dm_closed: closed }); });
    id
}

// サーバーのキャッシュを引く(GET)。見つからなければ Err
async fn get_json(st: &Shared, url: &str) -> Result<serde_json::Value, String> {
    let sess = auth::fresh(&st.http, false).await?;
    let r = st.http.get(url).bearer_auth(&sess.access).send().await.map_err(|_| i18n::t("notify.noConnection"))?;
    if r.status().as_u16() != 200 { return Err("not found".into()); }
    r.json().await.map_err(|_| "bad json".into())
}

fn dm_closed_msg() -> String {
    i18n::t("notify.dmClosed")
}

async fn send(st: &Shared, mut body: serde_json::Value) -> Result<serde_json::Value, String> {
    body["lang"] = i18n::lang().into(); // Bot が DM をこの言語で書くため
    send_to(st, config::NOTIFY_URL, body).await
}

async fn send_to(st: &Shared, url: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
    for attempt in 0..2 {
        let sess = auth::fresh(&st.http, attempt == 1).await?;
        let r = st.http.post(url).bearer_auth(&sess.access).json(&body)
            .send().await.map_err(|_| i18n::t("notify.noServer"))?;
        let status = r.status().as_u16();
        // 前段の nginx が 4xx の本文を差し替えることがあるので、ステータスだけで判定する
        match status {
            200 => return Ok(r.json().await.unwrap_or_default()),
            401 if attempt == 0 => continue,
            401 => return Err(i18n::t("notify.invalidLogin")),
            403 => return Err(i18n::t("notify.noDiscord")),
            409 => return Err(dm_closed_msg()),
            _ => return Err(format!("{} ({status})", i18n::t("notify.failed"))),
        }
    }
    Err(i18n::t("notify.failed"))
}

#[tauri::command]
async fn send_test(app: AppHandle, st: State<'_, St>) -> Result<(), String> {
    notify(&app, &st, serde_json::json!({"phase": "pregame", "test": true, "mode": i18n::t("mode.test"), "map": i18n::t("mode.testSend")})).await;
    let ok = st.snap.lock().unwrap().last_notify.as_ref().map(|n| n.ok).unwrap_or(false);
    if ok { mark_setup(&app); push(&app, &st, |s| s.setup_done = true); }
    Ok(())
}

// 共有 URL の発行・再発行(create)・削除(revoke)・確認(status)
async fn share_call(app: &AppHandle, st: &Shared, action: &str) -> Result<(), String> {
    let r = send_to(st, config::SHARE_URL, serde_json::json!({"action": action})).await?;
    let url = r["url"].as_str().map(String::from);
    *st.last_shared.lock().unwrap() = String::new(); // 次の巡回で現在の状態を送り直す
    push(app, st, |s| s.share = url);
    Ok(())
}

#[tauri::command]
async fn share_op(app: AppHandle, st: State<'_, St>, action: String) -> Result<(), String> {
    if !matches!(action.as_str(), "create" | "revoke" | "status") { return Err(i18n::t("notify.invalidAction")); }
    share_call(&app, &st, &action).await
}

// 画面の状態が変わったら共有 URL のサーバーへ送る(URL を発行しているときだけ)
fn sync_share(st: &St) {
    let (has, riot, mi) = { let s = st.snap.lock().unwrap(); (s.share.is_some(), s.riot.clone(), s.match_info.clone()) };
    if !has { return; }
    let key = format!("{riot}|{}", mi.as_ref().map(|m| m.to_string()).unwrap_or_default());
    { let mut l = st.last_shared.lock().unwrap(); if *l == key { return; } *l = key; }
    let m = mi.map(|m| {
        let left = (m["endsAt"].as_u64().unwrap_or(0) as i64 - auth::now() as i64).max(0);
        serde_json::json!({"mode": m["mode"], "map": m["map"], "mapImg": m["mapImg"], "left": left, "players": m["players"]})
    });
    let st = st.clone();
    tauri::async_runtime::spawn(async move {
        let _ = send_to(&st, config::SHARE_URL, serde_json::json!({"action": "push", "state": {"riot": riot, "match": m}})).await;
    });
}

fn set_update(app: &AppHandle, st: &Shared, state: &str, version: &str, message: &str) {
    let v = serde_json::json!({"state": state, "version": version, "message": message});
    push(app, st, |s| s.update = Some(v));
}

// 更新を確認する。manual なら見つけ次第すぐ入れる。自動のときは試合中を避け、true を返して再試行を促す
async fn do_update(app: &AppHandle, st: &Shared, install: bool) -> bool {
    if matches!(st.snap.lock().unwrap().update.as_ref().and_then(|u| u["state"].as_str()), Some("checking" | "downloading")) { return false; }
    set_update(app, st, "checking", "", "");
    let rel = match update::check(&st.http).await {
        Ok(Some(r)) => r,
        Ok(None) => { set_update(app, st, "none", "", &i18n::t("update.latest")); return false; }
        Err(m) => { set_update(app, st, "error", "", &m); return false; }
    };
    if !install { set_update(app, st, "available", &rel.version, &i18n::t("update.found")); return false; }
    set_update(app, st, "downloading", &rel.version, "");
    match update::download(&rel).await {
        Ok(path) => match update::launch(&path) {
            Ok(()) => { app.exit(0); false }
            Err(m) => { set_update(app, st, "error", &rel.version, &m); false }
        },
        Err(m) => { set_update(app, st, "error", &rel.version, &m); false }
    }
}

#[tauri::command]
async fn update_now(app: AppHandle, st: State<'_, St>) -> Result<(), String> {
    do_update(&app, &st, true).await;
    Ok(())
}

// 確認だけ行う(入れない)
#[tauri::command]
async fn check_update(app: AppHandle, st: State<'_, St>) -> Result<(), String> {
    do_update(&app, &st, false).await;
    Ok(())
}

// 起動直後と 6 時間ごとに確認する。見つけても入れず、表示だけにする
async fn auto_update(app: AppHandle, st: St) {
    loop {
        do_update(&app, &st, false).await;
        tokio::time::sleep(std::time::Duration::from_secs(6 * 3600)).await;
    }
}

// 試合中に再起動しても Embed を追えるよう、メッセージ ID と表示内容を保存する
#[derive(Serialize, serde::Deserialize, Default, Clone)]
struct Saved { msg: String, phase: String, view: View }
fn setup_path(app: &AppHandle) -> Option<std::path::PathBuf> { app.path().app_data_dir().ok().map(|d| d.join("setup.done")) }
fn mark_setup(app: &AppHandle) { if let Some(p) = setup_path(app) { if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); } let _ = std::fs::write(p, b"1"); } }
fn saved_path(app: &AppHandle) -> Option<std::path::PathBuf> { app.path().app_data_dir().ok().map(|d| d.join("match.json")) }
fn save_state(app: &AppHandle, msg: &str, phase: &str, v: &View) {
    let Some(f) = saved_path(app) else { return };
    let _ = std::fs::create_dir_all(f.parent().unwrap());
    let _ = std::fs::write(f, serde_json::to_string(&Saved { msg: msg.into(), phase: phase.into(), view: v.clone() }).unwrap_or_default());
}
fn load_state(app: &AppHandle) -> Option<Saved> {
    serde_json::from_str(&std::fs::read_to_string(saved_path(app)?).ok()?).ok()
}
fn clear_state(app: &AppHandle) { if let Some(f) = saved_path(app) { let _ = std::fs::remove_file(f); } }

// 2 秒ごとに VALORANT の状態を見る。マッチ成立(PREGAME)で Embed を送り、選択の変化・試合開始・終了・キャンセルで同じメッセージを編集する。
// 編集に失敗したら、成功するまで間隔を空けて再送する(Bot の再起動・ネットワーク断への対策)
async fn monitor(app: AppHandle, st: St) {
    let mut names = pregame::load_names(&st.http).await;
    if pregame::load_local(&st.http, i18n::lang()).await { push(&app, &st, |_| {}); }
    if auth::load().is_some() { let _ = share_call(&app, &st, "status").await; load_profile(&app, &st).await; }
    let mut prev = riot::Phase::Closed;
    let mut first = true;
    let mut last_ping = std::time::Instant::now() - std::time::Duration::from_secs(60);
    let mut why = String::new();
    let mut msg: Option<String> = None;
    let mut shown: Vec<pregame::Player> = Vec::new();
    let mut v = View::default();
    let mut want = "pregame".to_string(); // Embed に出したい段階
    let mut sent_key = String::new();    // 最後に Discord へ反映できた内容
    let mut failed = false;
    let mut last_try = 0u64;
    let mut next_board = 0u64;
    let mut board_tries = 0u32;          // 終了後のスコア取得の試行回数
    let mut id_tried = false;
    let mut party_now: Vec<serde_json::Value> = vec![];
    let mut away = 0u32;                 // 試合の画面から外れた連続回数(一瞬の取得失敗を終了と見なさないため)
    let mut saved = load_state(&app);    // 前回の起動から引き継ぐ試合
    loop {
        let p = riot::probe(&st.riot_http).await;
        let in_match = matches!(p.phase, riot::Phase::Pregame | riot::Phase::Ingame);

        // ロビーのパーティー(メニュー/待機中だけ。変わったときだけ画面へ反映)
        {
            let pty: Vec<serde_json::Value> = if matches!(p.phase, riot::Phase::Menus | riot::Phase::Queue) && !p.party.is_empty() {
                p.party.iter().map(|m| serde_json::json!({
                    "name": m["name"], "me": m["me"], "level": m["level"],
                    "rank": names.tiers.get(&m["tier"].as_u64().unwrap_or(0)).cloned().unwrap_or_default(),
                })).collect()
            } else { vec![] };
            if st.snap.lock().unwrap().party != pty { push(&app, &st, |s| s.party = pty.clone()); }
            party_now = pty;
        }
        // 試合 ID が無いまま試合中(再起動で引き継いだ場合など): 試合の ID を 1 回だけ取りに行く
        if p.phase == riot::Phase::Ingame && msg.is_some() && v.match_id.is_empty() && !id_tried {
            id_tried = true;
            if let Some((pu, mid)) = pregame::current_match(&st.riot_http, &st.remote_http, &names).await { v.puuid = pu; v.match_id = mid; log(&app, "match id recovered"); }
        }
        if !in_match && msg.is_none() { id_tried = false; }

        // 前回の起動で追っていた試合: まだ続いていれば再開、終わっていれば終了/キャンセルを反映
        if let Some(sv) = saved.clone() {
            if in_match && msg.is_none() {
                msg = Some(sv.msg); v = sv.view; want = if p.phase == riot::Phase::Ingame { "ingame".into() } else { "pregame".into() };
                sent_key.clear(); saved = None;
                log(&app, "resumed saved match");
            } else if matches!(p.phase, riot::Phase::Menus | riot::Phase::Queue) {
                msg = Some(sv.msg); v = sv.view; shown.clear();
                want = if sv.phase == "ingame" { "ended".into() } else { "cancelled".into() };
                sent_key.clear(); saved = None;
            }
        }

        if p.phase != prev {
            // 起動時に名前を取りきれなかったときは、状態が変わるたびに取り直す
            if !names.complete() { names = pregame::load_names(&st.http).await; }
            if pregame::load_local(&st.http, i18n::lang()).await { push(&app, &st, |_| {}); }
            log(&app, &format!("{:?} -> {:?} [{}] mode={} map={}", prev, p.phase, p.raw_keys, p.mode, p.map));
            let entered = p.phase == riot::Phase::Pregame && !matches!(prev, riot::Phase::Pregame | riot::Phase::Ingame);
            // 起動時にすでに PREGAME だった場合は通知しない(古い状態の可能性がある)
            if entered && !first && msg.is_none() {
                v = View::default();
                v.mode = pregame::mode_name(&p.mode);
                if let Some((n, img)) = names.maps.get(&p.map) { v.map = n.clone(); v.map_img = img.clone(); }
                else { v.map = p.map.rsplit('/').next().unwrap_or("").to_string(); }
                shown.clear();
                push(&app, &st, |s| s.match_info = None);
                want = "pregame".into();
                msg = notify(&app, &st, v.body("pregame", &shown)).await;
                if let Some(id) = &msg { sent_key = key_of(&want, &shown, &v); save_state(&app, id, &want, &v); }
            } else if msg.is_some() && p.phase == riot::Phase::Ingame {
                want = "ingame".into();
            }
            let name = p.phase.as_str().to_string();
            push(&app, &st, |s| {
                s.riot = name;
                if !in_match { s.match_info = None; }
                else if p.phase == riot::Phase::Ingame && let Some(m) = s.match_info.as_mut() { m["endsAt"] = 0.into(); }
            });
            // 待機開始の通知(既定はオフ。app_data_dir に queue_notify というファイルがあるときだけ送る。VALORANT への追加の問い合わせはしない)
            if p.phase == riot::Phase::Queue && prev == riot::Phase::Menus && !first && msg.is_none()
                && app.path().app_data_dir().map(|d| d.join("queue_notify").exists()).unwrap_or(false) {
                let q = View { mode: pregame::mode_name(&p.mode), ..Default::default() };
                let mut b = q.body("queue", &shown);
                b["party"] = serde_json::Value::Array(party_now.clone());
                let _ = notify(&app, &st, b).await;
            }
            prev = p.phase.clone();
        }

        // 試合の画面から 3 回連続(約 6 秒)外れたら、終了(試合中だった場合)かキャンセルとして扱う
        if msg.is_some() && !in_match && !matches!(want.as_str(), "ended" | "cancelled") {
            away += 1;
            if away >= 3 || matches!(p.phase, riot::Phase::Menus | riot::Phase::Queue) {
                want = if want == "ingame" { "ended".into() } else { "cancelled".into() };
            }
        } else if in_match { away = 0; }

        // エージェント選択中は、選択の変化を取り込む
        if msg.is_some() && want == "pregame" && p.phase == riot::Phase::Pregame
            && let Some(info) = pregame::fetch(&st.riot_http, &st.remote_http, &names).await {
                if !info.map.is_empty() { v.map = info.map.clone(); v.map_img = info.map_img.clone(); }
                let m = pregame::mode_name(&info.queue);
                if !m.is_empty() { v.mode = m; }
                if v.ends_at == 0 { v.ends_at = info.ends_at; }
                v.agent_img = info.agent_img;
                if !info.match_id.is_empty() { v.match_id = info.match_id; v.puuid = info.puuid; }
                shown = info.players;
                push(&app, &st, |s| s.match_info = Some(serde_json::json!({"mode": v.mode, "map": v.map, "endsAt": v.ends_at, "mapImg": v.map_img, "players": shown})));
            }

        // 終了したら、スコアボード(とコンペなら RR)が揃うまで送るのを待つ。Riot への問い合わせは控えめにする:
        //  - まずサーバーのキャッシュを見る(同じ試合を誰かが保存していれば Riot へ行かない)
        //  - Riot へは 4 秒おきに最大 6 回。429 が返ったら即やめる
        if msg.is_some() && want == "ended" && !v.match_id.is_empty() && board_tries < 6 && auth::now() >= next_board {
            if v.board.is_none() {
                board_tries += 1; next_board = auth::now() + 4;
                let url = format!("{}/{}?puuid={}", config::MATCH_URL, v.match_id, v.puuid);
                if let Ok(c) = get_json(&st, &url).await
                    && c["board"].is_object() { v.board = Some(c["board"].clone()); log(&app, "scoreboard from cache"); }
                if v.board.is_none() {
                    match pregame::result(&st.riot_http, &st.remote_http, &names, &v.match_id).await {
                        pregame::Res::Ok(b) => { v.board = Some(b); log(&app, "scoreboard fetched"); }
                        pregame::Res::Stop => { board_tries = 99; log(&app, "riot rate limited, skip scoreboard"); }
                        pregame::Res::Retry => {}
                    }
                }
            } else if v.mode == "コンペティティブ" && v.rank.is_none() {
                board_tries += 1; next_board = auth::now() + 5;
                match pregame::rank_update(&st.riot_http, &st.remote_http, &names, &v.match_id).await {
                    pregame::Res::Ok(r) => { v.rank = Some(r); }
                    pregame::Res::Stop => { board_tries = 99; }
                    pregame::Res::Retry => {}
                }
            }
        }
        let waiting_board = want == "ended" && !v.match_id.is_empty() && board_tries < 6 && (v.board.is_none() || (v.mode == "コンペティティブ" && v.rank.is_none()));

        // Discord 側が最新でなければ編集する。失敗中は 6 秒以上あけて再試行する
        if let Some(id) = msg.clone() {
            let key = key_of(&want, &shown, &v);
            if key != sent_key && !waiting_board && (!failed || auth::now() >= last_try + 6) {
                last_try = auth::now();
                match notify(&app, &st, v.upd(&id, &want, &shown)).await {
                    Some(new_id) => {
                        failed = false; sent_key = key;
                        if matches!(want.as_str(), "ended" | "cancelled") { msg = None; board_tries = 0; next_board = 0; clear_state(&app); }
                        else { save_state(&app, &new_id, &want, &v); msg = Some(new_id); }
                    }
                    None => { failed = true; log(&app, "update failed, will retry"); }
                }
            }
        }
        if p.phase == riot::Phase::Starting && p.raw_keys != why {
            log(&app, &format!("starting: {}", p.raw_keys));
        }
        why = p.raw_keys.clone();
        first = false;
        sync_share(&st);
        if last_ping.elapsed().as_secs() >= 5 {
            last_ping = std::time::Instant::now();
            if st.snap.lock().unwrap().logged_in {
                let st2 = st.clone();
                tokio::spawn(async move { let _ = send_to(&st2, config::PING_URL, serde_json::json!({})).await; });
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(config::POLL_MS)).await;
    }
}

fn key_of(phase: &str, players: &[pregame::Player], v: &View) -> String {
    format!("{phase}|{:?}|{}", players, v.agent_img)
}

#[derive(Default, Clone, Serialize, serde::Deserialize)]
#[serde(default)]
struct View { ends_at: u64, mode: String, map: String, map_img: String, agent_img: String, match_id: String, puuid: String, board: Option<serde_json::Value>, rank: Option<serde_json::Value> }
impl View {
    fn body(&self, phase: &str, players: &[pregame::Player]) -> serde_json::Value {
        serde_json::json!({"phase": phase, "mode": self.mode, "map": self.map, "mapImage": self.map_img, "agentImage": self.agent_img, "endsAt": self.ends_at, "players": players, "board": self.board, "matchId": self.match_id, "rank": self.rank})
    }
    fn upd(&self, id: &str, phase: &str, players: &[pregame::Player]) -> serde_json::Value {
        let mut b = self.body(phase, players);
        b["action"] = "update".into();
        b["messageId"] = id.into();
        b
    }
}

// 状態の切り替わりを記録する(Riot 側の形式が変わったときの調査用。ファイルは 200KB で作り直す)
fn log(app: &AppHandle, line: &str) {
    let Ok(dir) = app.path().app_log_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let f = dir.join("states.log");
    if std::fs::metadata(&f).map(|m| m.len() > 200_000).unwrap_or(false) {
        let _ = std::fs::remove_file(&f);
    }
    use std::io::Write;
    if let Ok(mut h) = std::fs::OpenOptions::new().create(true).append(true).open(f) {
        let _ = writeln!(h, "{} {}", auth::now(), line);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // インストーラー(Inno Setup の AppMutex)が起動中を検出するための名前付き Mutex。プロセス終了まで保持する
    #[cfg(windows)]
    unsafe {
        let name: Vec<u16> = "valorant-matchfound-running\0".encode_utf16().collect();
        windows_sys::Win32::System::Threading::CreateMutexW(std::ptr::null(), 0, name.as_ptr());
    }
    let http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(15)).build().unwrap();
    let remote_http = reqwest::Client::builder().timeout(std::time::Duration::from_secs(5)).build().unwrap();
    let riot_http = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(3))
        .build().unwrap();
    let session = auth::load();
    let st: St = Arc::new(Shared {
        snap: Mutex::new(Snapshot {
            riot: "closed".into(),
            logged_in: session.is_some(),
            email: session.map(|s| s.email).unwrap_or_default(),
            ..Default::default()
        }),
        http, riot_http, remote_http, last_shared: Mutex::new(String::new()),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
        .manage(st.clone())
        .invoke_handler(tauri::generate_handler![get_state, get_autostart, set_autostart, get_boot_show, set_boot_show, get_queue_notify, set_queue_notify, login, cancel_login, refresh_profile, logout, send_test, update_now, check_update, share_op, get_lang, set_lang])
        .on_window_event(|w, e| {
            // ×ボタンはトレイへ隠すだけにする
            if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                api.prevent_close();
                let _ = w.hide();
            }
        })
        .setup(move |app| {
            if let Some(l) = lang_file(app.handle()).and_then(|f| std::fs::read_to_string(f).ok()) { i18n::set(l.trim()); }
            { let done = setup_path(app.handle()).map(|p| p.exists()).unwrap_or(false); let st3 = app.state::<St>(); if done || st3.snap.lock().unwrap().logged_in { push(app.handle(), &st3, |s| s.setup_done = true); } }
            let open = MenuItem::with_id(app, "open", i18n::t("tray.open"), true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", i18n::t("tray.quit"), true, None::<&str>)?;
            let upd = MenuItem::with_id(app, "update", i18n::t("tray.update"), true, None::<&str>)?;
            app.manage(TrayItems(vec![(open.clone(), "tray.open"), (upd.clone(), "tray.update"), (quit.clone(), "tray.quit")]));
            let menu = Menu::with_items(app, &[&open, &upd, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("valorant-matchfound")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| match e.id.as_ref() {
                    "open" => show(app),
                    "update" => {
                        show(app);
                        let (a, st) = (app.clone(), app.state::<St>().inner().clone());
                        tauri::async_runtime::spawn(async move { do_update(&a, &st, false).await; });
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|t, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                        show(t.app_handle());
                    }
                })
                .build(app)?;
            if !std::env::args().any(|a| a == "--hidden") || get_boot_show(app.handle().clone()) {
                show(app.handle());
            }
            tauri::async_runtime::spawn(monitor(app.handle().clone(), st.clone()));
            tauri::async_runtime::spawn(auto_update(app.handle().clone(), st.clone()));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn load_profile(app: &AppHandle, st: &Shared) {
    if let Some(p) = auth::profile(&st.http).await { push(app, st, |s| s.profile = Some(p)); }
}
