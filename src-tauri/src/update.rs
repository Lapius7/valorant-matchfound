// GitHub Releases の最新版を確認し、インストーラーを検証してから静かに実行する
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const REPO: &str = "Lapius7/valorant-matchfound";
const UA: &str = concat!("valorant-matchfound/", env!("CARGO_PKG_VERSION"));

pub struct Release { pub version: String, pub url: String, pub sha256: String }

fn parse(v: &str) -> Vec<u64> {
    v.trim_start_matches('v').split('.').map(|x| x.parse().unwrap_or(0)).collect()
}

// 新しい版があれば Some。リリースがまだ無い場合は None
pub async fn check(c: &reqwest::Client) -> Result<Option<Release>, String> {
    let r = c.get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("User-Agent", UA).header("Accept", "application/vnd.github+json")
        .send().await.map_err(|_| "更新サーバーに接続できません".to_string())?;
    if r.status().as_u16() == 404 { return Ok(None); }
    if !r.status().is_success() { return Err(format!("更新の確認に失敗しました ({})", r.status().as_u16())); }
    let v: Value = r.json().await.map_err(|_| "更新情報を読めませんでした".to_string())?;
    let tag = v["tag_name"].as_str().unwrap_or("");
    if parse(tag) <= parse(env!("CARGO_PKG_VERSION")) { return Ok(None); }
    let a = v["assets"].as_array().and_then(|a| a.iter().find(|a| {
        let n = a["name"].as_str().unwrap_or("");
        n.contains("installer") && n.ends_with(".exe")
    })).ok_or("インストーラーが見つかりません")?;
    let url = a["browser_download_url"].as_str().unwrap_or("").to_string();
    // 改ざん対策: 配布側が出す SHA-256 と一致しなければ実行しない
    let sha256 = a["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).ok_or("ハッシュ値がないため更新できません")?.to_lowercase();
    if !url.starts_with(&format!("https://github.com/{REPO}/releases/download/")) { return Err("配布元が想定と違います".into()); }
    Ok(Some(Release { version: tag.trim_start_matches('v').to_string(), url, sha256 }))
}

pub async fn download(rel: &Release) -> Result<std::path::PathBuf, String> {
    let c = reqwest::Client::builder().timeout(std::time::Duration::from_secs(180)).build().map_err(|e| e.to_string())?;
    let bytes = c.get(&rel.url).header("User-Agent", UA).send().await
        .map_err(|_| "ダウンロードに失敗しました".to_string())?
        .bytes().await.map_err(|_| "ダウンロードに失敗しました".to_string())?;
    let got: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    if got != rel.sha256 { return Err("ダウンロードしたファイルが一致しません。更新を中止しました".into()); }
    let dir = std::env::temp_dir().join("valorant-matchfound-update");
    let _ = std::fs::create_dir_all(&dir);
    let f = dir.join(format!("valorant-matchfound-installer-v{}.exe", rel.version));
    std::fs::write(&f, &bytes).map_err(|e| e.to_string())?;
    Ok(f)
}

// 静かに実行する。インストーラー側が起動中のアプリを閉じ、終わったら自動で再起動する
pub fn launch(path: &std::path::Path) -> Result<(), String> {
    let mut cmd = std::process::Command::new(path);
    cmd.args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0000_0008 | 0x0000_0200); // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}
