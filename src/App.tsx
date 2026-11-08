import { useEffect, useState, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ArrowUpCircle, Check, Copy, Link2, Loader2, LogOut, TriangleAlert, RefreshCw, Send, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";

import { MatchPanel, type MatchInfo } from "@/match";

type Profile = { lapountId: string; lapountEmail: string; createdAt: string; lastSignIn: string; discordName: string; discordUser: string; discordId: string; discordEmail: string; avatar: string };
type Notify = { at: number; ok: boolean; message: string };
type Snap = {
  riot: "closed" | "starting" | "menus" | "queue" | "pregame" | "ingame";
  loggedIn: boolean;
  email: string;
  loginCode: string | null;
  loginError: string | null;
  lastNotify: Notify | null;
  matchInfo: MatchInfo | null;
  share: string | null;
  profile: Profile | null;
  botMissing: boolean;
  setupDone: boolean;
  update: { state: "checking" | "none" | "available" | "downloading" | "error"; version: string; message: string } | null;
};

const PHASE: Record<Snap["riot"], { label: string; hint: string; live: boolean }> = {
  closed: { label: "VALORANT が起動していません", hint: "起動すると自動で見張りを始めます", live: false },
  starting: { label: "接続しています", hint: "VALORANT の起動が終わるまでお待ちください", live: false },
  menus: { label: "待機中", hint: "マッチングを始めると、成立時に DM でお知らせします", live: true },
  queue: { label: "マッチング中", hint: "成立するまでこのまま待てます", live: true },
  pregame: { label: "マッチ成立", hint: "エージェント選択に進んでください", live: true },
  ingame: { label: "試合中", hint: "", live: true },
};

type Tab = "home" | "account" | "settings";
const SETUP_URL = "https://lapbot.lapius7.com/application/valorant-matchfound/setup";
const BOT_URL = "https://discord.com/oauth2/authorize?client_id=1470053949263577189&integration_type=1&scope=applications.commands";

function BotHelp({ n }: { n: Snap["lastNotify"] }) {
  if (!n || n.ok) return null;
  const closed = n.message.startsWith("DM を受け取れない");
  return (
    <div role="alert" className="flex flex-col gap-3 rounded-xl bg-secondary p-4 text-sm leading-5">
      <p className="flex items-start gap-2">
        <TriangleAlert className="mt-0.5 size-4 flex-none text-[#d99a7c]" aria-hidden />
        <span>{closed ? "DM を送れませんでした。Lapbot をあなたのアカウントにユーザーインストールしましたか？ まだなら、下のボタンから追加してください。" : n.message}</span>
      </p>
      {closed && (
        <div className="flex gap-2">
          <Button className="h-10 flex-1" onClick={() => openUrl(BOT_URL)}>Lapbot を追加</Button>
          <Button variant="outline" className="h-10" onClick={() => openUrl(SETUP_URL)}>手順を見る</Button>
        </div>
      )}
    </div>
  );
}

function LoginCard({ s, busy, run }: { s: Snap; busy: boolean; run: (c: string) => Promise<void> }) {
  return (
    <section className="rise flex flex-col gap-3 rounded-2xl bg-card p-5 shadow-[0_0_0_1px_var(--border)]">
      <h2 className="text-sm leading-5 font-medium">Lapount でログイン</h2>
      {s.loginCode ? (
        <>
          <p className="font-mono text-[28px] leading-9 font-medium tracking-[0.12em] select-text">{s.loginCode}</p>
          <p className="flex items-center gap-2 text-muted-foreground"><Loader2 className="size-4 flex-none animate-spin" />ブラウザで承認してください</p>
          <div className="flex gap-2">
            <Button className="h-10 flex-1" disabled={busy} onClick={() => run("login")}>やり直す</Button>
            <Button variant="outline" className="h-10" onClick={() => invoke("cancel_login")}>キャンセル</Button>
          </div>
        </>
      ) : (
        <>
          <p className="text-muted-foreground">ログインすると、連携済みの Discord に DM が届きます。</p>
          <Button className="h-10" disabled={busy} onClick={() => run("login")}>Lapount でログイン</Button>
        </>
      )}
      {s.loginError && <p className="text-destructive">{s.loginError}</p>}
    </section>
  );
}

function Setup({ s, busy, run }: { s: Snap; busy: boolean; run: (c: string) => Promise<void> }) {
  const needDiscord = s.loggedIn && !!s.profile && !s.profile.discordId;
  const step = !s.loggedIn ? 1 : needDiscord ? 2 : 3;
  const total = needDiscord || step === 2 ? 3 : 2;
  const num = !s.loggedIn ? 1 : needDiscord ? 2 : total;
  return (
    <section className="rise flex flex-col gap-4 rounded-2xl bg-card p-5 shadow-[0_0_0_1px_var(--border)]">
      <p className="font-mono text-xs leading-4 tracking-wide text-muted-foreground">セットアップ {num} / {total}</p>
      {step === 1 && (
        <>
          <p className="text-lg leading-7 font-medium">Lapount でログイン</p>
          {s.loginCode ? (
            <div className="flex flex-col gap-3">
              <p className="font-mono text-[28px] leading-9 font-medium tracking-[0.12em] select-text">{s.loginCode}</p>
              <p className="flex items-center gap-2 text-muted-foreground"><Loader2 className="size-4 flex-none animate-spin" />ブラウザで承認してください</p>
              <div className="flex gap-2">
                <Button className="h-10 flex-1" disabled={busy} onClick={() => run("login")}>やり直す</Button>
                <Button variant="outline" className="h-10" onClick={() => invoke("cancel_login")}>キャンセル</Button>
              </div>
            </div>
          ) : (
            <>
              <p className="text-muted-foreground">DM の送り先を確かめるため、Lapount アカウントでログインします。</p>
              <Button className="h-10" disabled={busy} onClick={() => run("login")}>Lapount でログイン</Button>
            </>
          )}
          {s.loginError && <p className="text-destructive">{s.loginError}</p>}
        </>
      )}
      {step === 2 && (
        <>
          <p className="text-lg leading-7 font-medium">Discord を連携</p>
          <p className="text-muted-foreground">Lapount に Discord アカウントが連携されていません。連携してから「確認」を押してください。</p>
          <div className="flex gap-2">
            <Button className="h-10 flex-1" onClick={() => openUrl("https://account.lapius7.com/dashboard")}>Lapount で連携する</Button>
            <Button variant="outline" className="h-10" disabled={busy} onClick={() => run("refresh_profile")}>確認</Button>
          </div>
        </>
      )}
      {step === 3 && (
        <>
          <p className="text-lg leading-7 font-medium">テスト送信</p>
          <p className="text-muted-foreground">Lapbot をアカウントに追加したあと、テスト送信で DM が届くか確かめます。届いたらセットアップ完了です。</p>
          <Button variant="outline" className="h-10" onClick={() => openUrl(SETUP_URL)}>Lapbot の追加方法</Button>
          <Button className="h-10" disabled={busy} onClick={() => run("send_test")}>
            {busy ? <Loader2 className="size-4 animate-spin" /> : <Send className="size-4" />}テスト送信
          </Button>
          <BotHelp n={s.lastNotify} />
        </>
      )}
    </section>
  );
}

const time = (s: number) => new Date(s * 1000).toLocaleTimeString("ja-JP", { hour: "2-digit", minute: "2-digit" });
const style = (i: number) => ({ "--i": i }) as CSSProperties;

export default function App() {
  const [s, setS] = useState<Snap | null>(null);
  const [auto, setAuto] = useState(false);
  const [busy, setBusy] = useState(false);
  const [tab, setTab] = useState<Tab>("home");

  useEffect(() => {
    invoke<Snap>("get_state").then(setS);
    invoke<boolean>("get_autostart").then(setAuto);
    const un = listen<Snap>("state", (e) => setS(e.payload));
    return () => void un.then((f) => f());
  }, []);

  if (!s) return null;
  const p = PHASE[s.riot];
  const hot = s.riot === "pregame";
  const compact = !!s.matchInfo;

  const run = async (cmd: string) => {
    setBusy(true);
    try { await invoke(cmd); } finally { setBusy(false); }
  };

  return (
    <main className="flex h-screen flex-col gap-3 p-4">
      <header className="rise flex h-8 items-center justify-between" style={style(0)}>
        <h1 className="text-[15px] leading-5 font-medium tracking-[-0.01em]">valorant-matchfound</h1>
        <UpdateChip u={s.update} />
      </header>
      {!s.setupDone ? <Setup s={s} busy={busy} run={run} /> : <>
      <Tabs tab={tab} set={setTab} />
      {tab === "home" && <>

      <section
        className={cn(
          "rise relative isolate flex flex-none flex-col gap-3 overflow-hidden rounded-2xl p-5 shadow-[0_0_0_1px_var(--border)] transition-[background-color,box-shadow] duration-300",
          hot ? "bg-signal/10 shadow-[0_0_0_1px_color-mix(in_oklch,var(--signal)_40%,transparent)]" : "bg-card",
        )}
        style={style(1)}
      >
        {s.matchInfo?.mapImg && <img key={s.matchInfo.mapImg} src={s.matchInfo.mapImg} alt="" className="map-bg absolute inset-0 -z-10 h-full w-full object-cover" />}
        <div className="flex items-center gap-2.5">
          <span className={cn("relative size-2.5 flex-none rounded-full", p.live ? "bg-signal ping" : "bg-muted-foreground/50")} />
          <span className="font-mono text-xs leading-4 tracking-wide text-muted-foreground uppercase">{s.riot}</span>
        </div>
        <p className="text-[22px] leading-8 font-medium tracking-[-0.02em]">{p.label}</p>
        {!s.matchInfo && <p className="min-h-5 text-muted-foreground">{p.hint}</p>}
        {s.matchInfo && <MatchPanel m={s.matchInfo} ingame={s.riot === "ingame"} />}
        
      </section>
      {s.loggedIn && s.botMissing && (
        <button type="button" onClick={() => openUrl("https://lapbot.lapius7.com/application/valorant-matchfound/setup")} className="rise flex items-center gap-3 rounded-2xl bg-card px-5 py-3 text-left text-sm leading-5 shadow-[0_0_0_1px_#d99a7c66] transition-[scale] duration-150 active:scale-[0.98]" style={style(2)}>
          <TriangleAlert className="size-4 flex-none text-[#d99a7c]" aria-hidden />
          <span>Lapbot が未追加のため DM が届きません。追加方法を開く</span>
        </button>
      )}
      {s.loggedIn && s.profile && !s.profile.discordId && (
        <button type="button" onClick={() => setTab("account")} className="rise flex items-center gap-3 rounded-2xl bg-card px-5 py-3 text-left text-sm leading-5 shadow-[0_0_0_1px_#d99a7c66] transition-[scale] duration-150 active:scale-[0.98]" style={style(2)}>
          <TriangleAlert className="size-4 flex-none text-[#d99a7c]" aria-hidden />
          <span>Discord が未連携のため DM は届きません。アカウントタブで確認してください。</span>
        </button>
      )}
      {s.loggedIn ? <ShareCard url={s.share} compact={compact} /> : (
        <button type="button" onClick={() => setTab("account")} className="rise flex h-12 items-center justify-between rounded-2xl bg-card px-5 text-sm shadow-[0_0_0_1px_var(--border)] transition-[scale] duration-150 active:scale-[0.98]" style={style(2)}>
          <span className="text-muted-foreground">DM を受け取るには Lapount でログイン</span><span>アカウントへ</span>
        </button>
      )}
      </>}
      {tab === "account" && <>
      {s.loggedIn ? <>
        <AccountCard p={s.profile} email={s.email} botMissing={s.botMissing} />
        <Button variant="outline" className="rise h-11 flex-none rounded-xl" style={style(2)} onClick={() => invoke("logout")}>
          <LogOut className="size-4" />ログアウト
        </Button>
      </> : <LoginCard s={s} busy={busy} run={run} />}
      </>}
      {tab === "settings" && <>
      <section className="rise flex flex-none flex-col gap-3 rounded-2xl bg-card p-5 shadow-[0_0_0_1px_var(--border)]" style={style(1)}>
        <h2 className="text-sm leading-5 font-medium">Discord への通知</h2>
        <p className="text-xs leading-5 text-muted-foreground">マッチ成立の DM が届くか確かめます。</p>
        <Button variant="secondary" className="h-10" disabled={busy || !s.loggedIn} onClick={() => run("send_test")}>
          {busy ? <Loader2 className="size-4 animate-spin" /> : <Send className="size-4" />}テスト送信
        </Button>
        {s.lastNotify && (
          <p className={cn("text-xs leading-5 tabular-nums", s.lastNotify.ok ? "text-muted-foreground" : "text-destructive")}>
            {time(s.lastNotify.at)}　{s.lastNotify.message}
          </p>
        )}
        <BotHelp n={s.lastNotify} />
        {!(s.lastNotify && !s.lastNotify.ok) && <div className="flex gap-4 text-xs leading-4 text-muted-foreground">
          <button type="button" className="underline underline-offset-4 transition-colors duration-150 hover:text-foreground" onClick={() => openUrl(BOT_URL)}>Lapbot を追加</button>
          <button type="button" className="underline underline-offset-4 transition-colors duration-150 hover:text-foreground" onClick={() => openUrl(SETUP_URL)}>追加方法</button>
        </div>}
      </section>
      <section className="rise flex h-14 flex-none items-center justify-between rounded-2xl bg-card px-5 shadow-[0_0_0_1px_var(--border)]" style={style(2)}>
        <label htmlFor="auto" className="text-sm leading-5">Windows の起動時に開始</label>
        <Switch id="auto" checked={auto} onCheckedChange={(v) => { setAuto(v); invoke("set_autostart", { on: v }); }} />
      </section>
      <p className="rise px-1 font-mono text-xs leading-4 text-muted-foreground" style={style(3)}>v{__APP_VERSION__}</p>
      </>}
      </>}
    </main>
  );
}

// 右上のバージョン表示。押すと更新を確認し、新しい版があればそのまま入れる
function UpdateChip({ u }: { u: Snap["update"] }) {
  const busy = u?.state === "checking" || u?.state === "downloading";
  const label =
    u?.state === "checking" ? "確認中" :
    u?.state === "downloading" ? `v${u.version} をダウンロード中` :
    u?.state === "available" ? `v${u.version} あり` :
    u?.state === "none" ? "最新版です" :
    u?.state === "error" ? u.message : "更新を確認";
  const Icon = busy ? Loader2 : u?.state === "available" ? ArrowUpCircle : u?.state === "none" ? Check : RefreshCw;
  return (
    <button
      type="button"
      disabled={busy}
      title={u?.message || undefined}
      onClick={() => invoke("update_now")}
      className={cn(
        "group relative flex h-8 max-w-[260px] items-center gap-1.5 rounded-full px-3 font-mono text-xs leading-4 transition-[background-color,color,scale] duration-150 active:scale-[0.96] before:absolute before:-inset-1 before:content-['']",
        u?.state === "error" ? "text-destructive" : u?.state === "available" ? "bg-signal/15 text-signal" : "text-muted-foreground hover:bg-card hover:text-foreground",
      )}
    >
      <Icon className={cn("size-3.5 flex-none", busy && "animate-spin")} />
      <span className="truncate">{u ? label : `v${__APP_VERSION__}`}</span>
    </button>
  );
}

// 観戦用URL。知っている人は誰でも見られるため、発行・再発行・削除をここで行う
function ShareCard({ url, compact }: { url: string | null; compact: boolean }) {
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const op = async (action: "create" | "revoke") => {
    setBusy(true);
    try { await invoke("share_op", { action }); } finally { setBusy(false); }
  };
  const copy = async () => {
    if (!url) return;
    await navigator.clipboard.writeText(url);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };
  return (
    <section className={cn("rise flex flex-none flex-col gap-3 rounded-2xl bg-card px-5 shadow-[0_0_0_1px_var(--border)] transition-[padding] duration-300", compact ? "py-3.5" : "py-5")} style={style(3)}>
      <div className="flex h-5 items-center justify-between gap-3">
        <h2 className="text-sm leading-5 font-medium">観戦URL</h2>
        {url && <span className="flex items-center gap-1.5 text-xs leading-4 text-muted-foreground"><span className="size-1.5 rounded-full bg-signal" />公開中</span>}
      </div>
      {url ? (
        <>
          <p className="truncate rounded-lg bg-secondary px-3 py-2 font-mono text-xs leading-5 select-text">{url}</p>
          <div className="flex gap-2">
            <Button variant="secondary" className="h-10 flex-1" onClick={copy}>
              {copied ? <Check className="size-4" /> : <Copy className="size-4" />}{copied ? "コピーしました" : "コピー"}
            </Button>
            <Button variant="outline" className="h-10" disabled={busy} onClick={() => op("create")}>
              <RefreshCw className={cn("size-4", busy && "animate-spin")} />再発行
            </Button>
            <Button variant="outline" className="h-10 w-10 px-0" disabled={busy} aria-label="削除" onClick={() => op("revoke")}>
              <Trash2 className="size-4" />
            </Button>
          </div>
          <p className="text-xs leading-4 text-muted-foreground">URL を知っている人は誰でも、マッチ中のプレイヤー名を見られます。検索には出ません。</p>
        </>
      ) : (
        <Button variant="secondary" className="h-10" disabled={busy} onClick={() => op("create")}>
          {busy ? <Loader2 className="size-4 animate-spin" /> : <Link2 className="size-4" />}URLを発行
        </Button>
      )}
    </section>
  );
}

function Tabs({ tab, set }: { tab: Tab; set: (t: Tab) => void }) {
  const items = [["home", "ホーム"], ["account", "アカウント"], ["settings", "設定"]] as const;
  return (
    <nav className="rise flex h-10 flex-none gap-1 rounded-xl bg-secondary p-1" style={style(0)}>
      {items.map(([k, l]) => (
        <button key={k} type="button" onClick={() => set(k)} className={cn("flex-1 rounded-lg text-sm leading-5 transition-[background-color,color,scale] duration-150 active:scale-[0.96]", tab === k ? "bg-card text-foreground shadow-[0_0_0_1px_var(--border)]" : "text-muted-foreground hover:text-foreground")}>{l}</button>
      ))}
    </nav>
  );
}

const day = (v: string) => (v ? new Date(v).toLocaleString("ja-JP", { dateStyle: "medium", timeStyle: "short" }) : "—");

// ログイン中の Lapount アカウントと、連携している Discord の詳細
function AccountCard({ p, email, botMissing }: { p: Profile | null; email: string; botMissing: boolean }) {
  const rows: [string, string][] = [
    ["Discord 名", p?.discordName || "—"],
    ["Discord ユーザー名", p?.discordUser || "—"],
    ["Discord ID", p?.discordId || "—"],
    ["Discord メール", p?.discordEmail || "—"],
    ["Lapount メール", p?.lapountEmail || email || "—"],
    ["Lapount ID", p?.lapountId || "—"],
    ["登録日", day(p?.createdAt ?? "")],
    ["最終ログイン", day(p?.lastSignIn ?? "")],
  ];
  return (
    <section className="flex flex-none flex-col gap-3 rounded-2xl bg-card p-5 shadow-[0_0_0_1px_var(--border)]">
      <div className="flex items-center gap-3">
        {p?.avatar ? <img src={p.avatar} alt="" className="size-10 flex-none rounded-full outline outline-1 -outline-offset-1 outline-white/10" /> : <span className="size-10 flex-none rounded-full bg-secondary" />}
        <div className="min-w-0">
          <p className="truncate text-sm leading-5 font-medium">{p?.discordName || p?.discordUser || "アカウント"}</p>
          <p className="truncate text-xs leading-4 text-muted-foreground">{p?.discordUser ? `@${p.discordUser}` : "Lapount ログイン中"}</p>
        </div>
      </div>
      {p && !p.discordId && (
        <div role="alert" className="flex flex-col gap-3 rounded-xl bg-secondary p-4">
          <p className="flex items-start gap-2 text-sm leading-5">
            <TriangleAlert className="mt-0.5 size-4 flex-none text-[#d99a7c]" aria-hidden />
            <span>Lapount に Discord が連携されていません。連携すると、マッチ成立の DM が届きます。</span>
          </p>
          <Button variant="secondary" className="h-10 bg-card" onClick={() => openUrl("https://account.lapius7.com/dashboard")}>Lapount で Discord を連携する</Button>
          <p className="text-xs leading-4 text-muted-foreground">連携したあと、アプリを開き直すと反映されます。</p>
        </div>
      )}
      <p className={cn("text-xs leading-5", botMissing ? "rounded-xl bg-secondary p-3 text-sm text-foreground" : "text-muted-foreground")}>
        {botMissing ? "Lapbot がアカウントに追加されていないため、DM が届きません。" : "DM を受け取るには、Lapbot をアカウントに追加してください。"}
        <button type="button" className="ml-1 underline underline-offset-4 transition-colors duration-150 hover:text-foreground" onClick={() => openUrl("https://lapbot.lapius7.com/application/valorant-matchfound/setup")}>追加方法を見る</button>
      </p>
      <dl className="flex flex-col gap-1.5 text-xs leading-4">
        {rows.map(([k, v]) => (
          <div key={k} className="flex justify-between gap-4">
            <dt className="flex-none text-muted-foreground">{k}</dt>
            <dd className="min-w-0 truncate text-right tabular-nums select-text">{v}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}
