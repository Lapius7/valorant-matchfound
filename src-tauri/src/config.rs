// 変わりやすい値はここに集約する
pub const SUPABASE_URL: &str = "https://supabase.lapius7.com";
pub const SHARE_URL: &str = "https://lapbot.lapius7.com/api/matchfound/share";
pub const PING_URL: &str = "https://lapbot.lapius7.com/api/matchfound/ping";
pub const MATCH_URL: &str = "https://lapbot.lapius7.com/api/matchfound/match";
pub const NOTIFY_URL: &str = "https://lapbot.lapius7.com/api/matchfound/notify";
// 公開鍵(anon)。Lapount のログインにだけ使う。実行時の環境変数 MATCHFOUND_ANON_KEY で上書きできる
pub const ANON_KEY: &str = include_str!("anon_key.txt");
pub const POLL_MS: u64 = 2000;
