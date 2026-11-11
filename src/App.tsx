import { useEffect, useState, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ArrowUpCircle, Check, Copy, Link2, Loader2, LogOut, TriangleAlert, RefreshCw, Send, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { useI18n } from "@/i18n/useI18n";
import { getLang, LOCALE_NAMES, t, type Locale } from "@/i18n/index";

import { MatchPanel, type MatchInfo } from "@/match";

type Profile = { lapountId: string; lapountEmail: string; createdAt: string; lastSignIn: string; discordName: string; discordUser: string; discordId: string; discordEmail: string; avatar: string };
type Notify = { at: number; ok: boolean; message: string; dmClosed: boolean };
type Snap = {
  riot: "closed" | "starting" | "menus" | "queue" | "pregame" | "ingame";
  loggedIn: boolean;
  email: string;
  loginCode: string | null;
  loginError: string | null;
  lastNotify: Notify | null;
  matchInfo: MatchInfo | null;
  party: { name: string; me: boolean; level: number; rank: string }[];
  share: string | null;
  profile: Profile | null;
  botMissing: boolean;
  setupDone: boolean;
  update: { state: "checking" | "none" | "available" | "downloading" | "error"; version: string; message: string } | null;
};

const phaseConfig = (): Record<Snap["riot"], { label: string; hint: string; live: boolean }> => ({
  closed: { label: t("phase.closed.label"), hint: t("phase.closed.hint"), live: false },
  starting: { label: t("phase.starting.label"), hint: t("phase.starting.hint"), live: false },
  menus: { label: t("phase.menus.label"), hint: t("phase.menus.hint"), live: true },
  queue: { label: t("phase.queue.label"), hint: t("phase.queue.hint"), live: true },
  pregame: { label: t("phase.pregame.label"), hint: t("phase.pregame.hint"), live: true },
  ingame: { label: t("phase.ingame.label"), hint: t("phase.ingame.hint"), live: true },
});

type Tab = "home" | "account" | "settings";
const SETUP_URL = "https://lapbot.lapius7.com/application/valorant-matchfound/setup";
const BOT_URL = "https://discord.com/oauth2/authorize?client_id=1470053949263577189&integration_type=1&scope=applications.commands";

function BotHelp({ n }: { n: Snap["lastNotify"] }) {
  if (!n || n.ok) return null;
  const closed = n.dmClosed;
  return (
    <div role="alert" className="flex flex-col gap-3 rounded-xl bg-secondary p-4 text-sm leading-5">
      <p className="flex items-start gap-2">
        <TriangleAlert className="mt-0.5 size-4 flex-none text-[#d99a7c]" aria-hidden />
        <span>{closed ? t("bot.dmFailed") : n.message}</span>
      </p>
      {closed && (
        <div className="flex gap-2">
          <Button className="h-10 flex-1" onClick={() => openUrl(BOT_URL)}>{t("bot.addLapbot")}</Button>
          <Button variant="outline" className="h-10" onClick={() => openUrl(SETUP_URL)}>{t("bot.seeSteps")}</Button>
        </div>
      )}
    </div>
  );
}

function LoginCard({ s, busy, run }: { s: Snap; busy: boolean; run: (c: string) => Promise<void> }) {
  return (
    <section className="rise flex flex-col gap-3 rounded-2xl bg-card p-5 shadow-[0_0_0_1px_var(--border)]">
      <h2 className="text-sm leading-5 font-medium">{t("login.title")}</h2>
      {s.loginCode ? (
        <>
          <p className="font-mono text-[28px] leading-9 font-medium tracking-[0.12em] select-text">{s.loginCode}</p>
          <p className="flex items-center gap-2 text-muted-foreground"><Loader2 className="size-4 flex-none animate-spin" />{t("login.approveInBrowser")}</p>
          <div className="flex gap-2">
            <Button className="h-10 flex-1" disabled={busy} onClick={() => run("login")}>{t("login.redo")}</Button>
            <Button variant="outline" className="h-10" onClick={() => invoke("cancel_login")}>{t("login.cancel")}</Button>
          </div>
        </>
      ) : (
        <>
          <p className="text-muted-foreground">{t("login.desc")}</p>
          <Button className="h-10" disabled={busy} onClick={() => run("login")}>{t("login.button")}</Button>
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
      <p className="font-mono text-xs leading-4 tracking-wide text-muted-foreground">{t("app.setupProgress", { num, total })}</p>
      {step === 1 && (
        <>
          <p className="text-lg leading-7 font-medium">{t("setup.title")}</p>
          {s.loginCode ? (
            <div className="flex flex-col gap-3">
              <p className="font-mono text-[28px] leading-9 font-medium tracking-[0.12em] select-text">{s.loginCode}</p>
              <p className="flex items-center gap-2 text-muted-foreground"><Loader2 className="size-4 flex-none animate-spin" />{t("login.approveInBrowser")}</p>
              <div className="flex gap-2">
                <Button className="h-10 flex-1" disabled={busy} onClick={() => run("login")}>{t("login.redo")}</Button>
                <Button variant="outline" className="h-10" onClick={() => invoke("cancel_login")}>{t("login.cancel")}</Button>
              </div>
            </div>
          ) : (
            <>
              <p className="text-muted-foreground">{t("setup.desc")}</p>
              <Button className="h-10" disabled={busy} onClick={() => run("login")}>{t("setup.loginButton")}</Button>
            </>
          )}
          {s.loginError && <p className="text-destructive">{s.loginError}</p>}
        </>
      )}
      {step === 2 && (
        <>
          <p className="text-lg leading-7 font-medium">{t("setup.discordTitle")}</p>
          <p className="text-muted-foreground">{t("setup.discordDesc")}</p>
          <div className="flex gap-2">
            <Button className="h-10 flex-1" onClick={() => openUrl("https://account.lapius7.com/dashboard")}>{t("setup.linkDiscord")}</Button>
            <Button variant="outline" className="h-10" disabled={busy} onClick={() => run("refresh_profile")}>{t("setup.confirm")}</Button>
          </div>
        </>
      )}
      {step === 3 && (
        <>
          <p className="text-lg leading-7 font-medium">{t("setup.testTitle")}</p>
          <p className="text-muted-foreground">{t("setup.testDesc")}</p>
          <Button variant="outline" className="h-10" onClick={() => openUrl(SETUP_URL)}>{t("setup.testMethod")}</Button>
          <Button className="h-10" disabled={busy} onClick={() => run("send_test")}>
            {busy ? <Loader2 className="size-4 animate-spin" /> : <Send className="size-4" />}{t("setup.testButton")}
          </Button>
          <BotHelp n={s.lastNotify} />
        </>
      )}
    </section>
  );
}

const time = (s: number) => new Date(s * 1000).toLocaleTimeString(getLang(), { hour: "2-digit", minute: "2-digit" });
const day = (v: string) => (v ? new Date(v).toLocaleString(getLang(), { dateStyle: "medium", timeStyle: "short" }) : "—");
const style = (i: number) => ({ "--i": i }) as CSSProperties;

export default function App() {
  const { lang, setLang, t } = useI18n();
  const [s, setS] = useState<Snap | null>(null);
  const [auto, setAuto] = useState(false);
  const [bootShow, setBootShow] = useState(false);
  const [queueN, setQueueN] = useState(false);
  const [busy, setBusy] = useState(false);
  const [tab, setTab] = useState<Tab>("home");

  // バックエンド(トレイ・エラー文・試合の表示名)と html の lang を画面の言語に合わせる
  useEffect(() => {
    document.documentElement.lang = lang;
    invoke("set_lang", { lang });
  }, [lang]);

  useEffect(() => {
    invoke<Snap>("get_state").then(setS);
    invoke<boolean>("get_autostart").then(setAuto);
    invoke<boolean>("get_boot_show").then(setBootShow);
    invoke<boolean>("get_queue_notify").then(setQueueN);
    const un = listen<Snap>("state", (e) => setS(e.payload));
    return () => void un.then((f) => f());
  }, []);

  if (!s) return null;
  const p = phaseConfig()[s.riot];
  const hot = s.riot === "pregame";
  const compact = !!s.matchInfo;

  const run = async (cmd: string) => {
    setBusy(true);
    try { await invoke(cmd); } finally { setBusy(false); }
  };

  return (
    <main className="flex h-screen flex-col gap-3 p-4">
      <header className="rise flex h-8 items-center justify-between" style={style(0)}>
        <h1 className="text-[15px] leading-5 font-medium tracking-[-0.01em]">{t("app.title")}</h1>
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
        {!s.matchInfo && s.party.length > 0 && (<>
          <p className="pt-1 text-xs leading-4 text-muted-foreground">{t("party.party", { count: s.party.length })}</p>
          <ul className="grid gap-1.5 pt-1">
            {s.party.map((m) => (
              <li key={m.name} className="flex h-7 items-center justify-between gap-3 text-sm leading-5">
                <span className="min-w-0 truncate">{m.name}{m.me && <span className="ml-1.5 text-xs text-muted-foreground">{t("party.you")}</span>}</span>
                <span className="flex-none text-xs text-muted-foreground tabular-nums">{m.rank || "—"}{m.level ? ` / Lv ${m.level}` : ""}</span>
              </li>
            ))}
          </ul></>)}
        
      </section>
      {s.loggedIn && s.botMissing && (
        <button type="button" onClick={() => openUrl("https://lapbot.lapius7.com/application/valorant-matchfound/setup")} className="rise flex items-center gap-3 rounded-2xl bg-card px-5 py-3 text-left text-sm leading-5 shadow-[0_0_0_1px_#d99a7c66] transition-[scale] duration-150 active:scale-[0.98]" style={style(2)}>
          <TriangleAlert className="size-4 flex-none text-[#d99a7c]" aria-hidden />
          <span>{t("account.botMissing")}</span>
        </button>
      )}
      {s.loggedIn && s.profile && !s.profile.discordId && (
        <button type="button" onClick={() => setTab("account")} className="rise flex items-center gap-3 rounded-2xl bg-card px-5 py-3 text-left text-sm leading-5 shadow-[0_0_0_1px_#d99a7c66] transition-[scale] duration-150 active:scale-[0.98]" style={style(2)}>
          <TriangleAlert className="size-4 flex-none text-[#d99a7c]" aria-hidden />
          <span>{t("account.noDiscord.title")}</span>
        </button>
      )}
      {s.loggedIn && <Button variant="outline" className="h-10 flex-none" onClick={() => openUrl(`https://lapbot.lapius7.com/application/valorant-matchfound/history?lang=${lang}`)}>{t("settings.matchHistory")}</Button>}
      {s.loggedIn ? <ShareCard url={s.share} compact={compact} /> : (
        <button type="button" onClick={() => setTab("account")} className="rise flex h-12 items-center justify-between rounded-2xl bg-card px-5 text-sm shadow-[0_0_0_1px_var(--border)] transition-[scale] duration-150 active:scale-[0.98]" style={style(2)}>
          <span className="text-muted-foreground">{t("login.requireDm")}</span><span>{t("login.goAccount")}</span>
        </button>
      )}
      </>}
      {tab === "account" && <>
      {s.loggedIn ? <>
        <AccountCard p={s.profile} email={s.email} botMissing={s.botMissing} />
        <Button variant="outline" className="rise h-11 flex-none rounded-xl" style={style(2)} onClick={() => invoke("logout")}>
          <LogOut className="size-4" />{t("account.logout")}
        </Button>
      </> : <LoginCard s={s} busy={busy} run={run} />}
      </>}
      {tab === "settings" && <>
      <section className="rise flex flex-none flex-col gap-3 rounded-2xl bg-card p-5 shadow-[0_0_0_1px_var(--border)]" style={style(1)}>
        <h2 className="text-sm leading-5 font-medium">{t("settings.notificationsTitle")}</h2>
        <p className="text-xs leading-5 text-muted-foreground">{t("settings.notificationsDesc")}</p>
        <Button variant="secondary" className="h-10" disabled={busy || !s.loggedIn} onClick={() => run("send_test")}>
          {busy ? <Loader2 className="size-4 animate-spin" /> : <Send className="size-4" />}{t("settings.testButton")}
        </Button>
        {s.lastNotify && (
          <p className={cn("text-xs leading-5 tabular-nums", s.lastNotify.ok ? "text-muted-foreground" : "text-destructive")}>
            {time(s.lastNotify.at)}{" "}{s.lastNotify.message}
          </p>
        )}
        <BotHelp n={s.lastNotify} />
        {!(s.lastNotify && !s.lastNotify.ok) && <div className="flex gap-4 text-xs leading-4 text-muted-foreground">
          <button type="button" className="underline underline-offset-4 transition-colors duration-150 hover:text-foreground" onClick={() => openUrl(BOT_URL)}>{t("settings.addLapbot")}</button>
          <button type="button" className="underline underline-offset-4 transition-colors duration-150 hover:text-foreground" onClick={() => openUrl(SETUP_URL)}>{t("settings.addMethod")}</button>
        </div>}
      </section>
      <section className="rise flex h-14 flex-none items-center justify-between rounded-2xl bg-card px-5 shadow-[0_0_0_1px_var(--border)]" style={style(2)}>
        <label htmlFor="auto" className="text-sm leading-5">{t("settings.autostart")}</label>
        <Switch id="auto" checked={auto} onCheckedChange={(v) => { setAuto(v); invoke("set_autostart", { on: v }); }} />
      </section>
      <section className="rise flex h-14 flex-none items-center justify-between rounded-2xl bg-card px-5 shadow-[0_0_0_1px_var(--border)]" style={style(3)}>
        <label htmlFor="bootshow" className="text-sm leading-5">{t("settings.bootShow")}</label>
        <Switch id="bootshow" checked={bootShow} onCheckedChange={(v) => { setBootShow(v); invoke("set_boot_show", { on: v }); }} />
      </section>
      <section className="rise flex h-14 flex-none items-center justify-between rounded-2xl bg-card px-5 shadow-[0_0_0_1px_var(--border)]" style={style(4)}>
        <label htmlFor="queuen" className="text-sm leading-5">{t("settings.queueNotify")}</label>
        <Switch id="queuen" checked={queueN} onCheckedChange={(v) => { setQueueN(v); invoke("set_queue_notify", { on: v }); }} />
      </section>
      <section className="rise flex h-14 flex-none items-center justify-between rounded-2xl bg-card px-5 shadow-[0_0_0_1px_var(--border)]" style={style(2)}>
        <label htmlFor="lang" className="text-sm leading-5">{t("settings.lang")}</label>
        <select id="lang" value={lang} onChange={(e) => setLang(e.target.value as Locale)} className="text-sm leading-5 rounded-md border border-input bg-background px-2 py-1.5">
          {LOCALE_NAMES.map(([code, name]) => <option key={code} value={code}>{name}</option>)}
        </select>
      </section>
      <section className="rise grid gap-1 rounded-2xl bg-card p-5 text-xs leading-5 text-muted-foreground shadow-[0_0_0_1px_var(--border)]" style={style(3)}>
          <p className="text-sm leading-5 font-medium text-foreground">{t("settings.contact")}</p>
          <p>{t("settings.contactDiscordLabel")} <span className="font-mono text-foreground select-text">x_lapius.xyz</span>{t("settings.contactDiscordSuffix")}</p>
          <p>X: <button type="button" className="text-foreground underline underline-offset-4" onClick={() => openUrl("https://x.com/Lapius7")}>@Lapius7</button>{t("settings.contactXSuffix")}</p>
          <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1">
            <button type="button" className="underline underline-offset-4 hover:text-foreground" onClick={() => openUrl("https://lapbot.lapius7.com/application/valorant-matchfound")}>{t("settings.officialPage")}</button>
            <button type="button" className="underline underline-offset-4 hover:text-foreground" onClick={() => openUrl(SETUP_URL)}>{t("settings.docs")}</button>
            <button type="button" className="underline underline-offset-4 hover:text-foreground" onClick={() => openUrl("https://github.com/Lapius7/valorant-matchfound")}>{t("settings.github")}</button>
          </div>
          <p className="mt-2 font-mono">{t("settings.version", { version: __APP_VERSION__ })}</p>
        </section>
      </>}
      </>}
    </main>
  );
}

// 右上のバージョン表示。押すと更新を確認し、新しい版があればそのまま入れる
function UpdateChip({ u }: { u: Snap["update"] }) {
  const [ask, setAsk] = useState(false);
  const busy = u?.state === "checking" || u?.state === "downloading";
  const label =
    u?.state === "checking" ? t("update.checking") :
    u?.state === "downloading" ? t("update.downloading", { version: u.version }) :
    u?.state === "available" ? t("update.available", { version: u.version }) :
    u?.state === "none" ? t("update.none") :
    u?.state === "error" ? t("update.error", { message: u.message }) :
    t("update.check");
  const Icon = busy ? Loader2 : u?.state === "available" ? ArrowUpCircle : u?.state === "none" ? Check : RefreshCw;
  return (
    <>
    <button
      type="button"
      disabled={busy}
      title={u?.message || undefined}
      onClick={() => (u?.state === "available" ? setAsk(true) : invoke("check_update"))}
      className={cn(
        "group relative flex h-8 max-w-[260px] items-center gap-1.5 rounded-full px-3 font-mono text-xs leading-4 transition-[background-color,color,scale] duration-150 active:scale-[0.96] before:absolute before:-inset-1 before:content-['']",
        u?.state === "error" ? "text-destructive" : u?.state === "available" ? "bg-signal/15 text-signal" : "text-muted-foreground hover:bg-card hover:text-foreground",
      )}
    >
      <Icon className={cn("size-3.5 flex-none", busy && "animate-spin")} />
      <span className="truncate">{u ? label : t("settings.version", { version: __APP_VERSION__ })}</span>
    </button>
      {ask && createPortal(
        <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 p-6" onClick={() => setAsk(false)}>
          <div role="alertdialog" aria-modal="true" className="w-full max-w-[300px] rounded-2xl bg-card p-5 shadow-[0_0_0_1px_var(--border)]" onClick={(e) => e.stopPropagation()}>
            <p className="text-sm leading-6 font-medium">{t("update.dialogTitle")}</p>
            <p className="mt-1 text-xs leading-5 text-muted-foreground">{t("update.dialogDesc", { version: u?.version ?? "" })}</p>
            <div className="mt-4 flex justify-end gap-2">
              <Button variant="ghost" size="sm" onClick={() => setAsk(false)}>{t("update.cancel")}</Button>
              <Button size="sm" onClick={() => { setAsk(false); invoke("update_now"); }}>{t("update.update")}</Button>
            </div>
          </div>
        </div>,
        document.body,
      )}
    </>
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
        <h2 className="text-sm leading-5 font-medium">{t("share.title")}</h2>
        {url && <span className="flex items-center gap-1.5 text-xs leading-4 text-muted-foreground"><span className="size-1.5 rounded-full bg-signal" />{t("share.public")}</span>}
      </div>
      {url ? (
        <>
          <p className="truncate rounded-lg bg-secondary px-3 py-2 font-mono text-xs leading-5 select-text">{url}</p>
          <div className="flex gap-2">
            <Button variant="secondary" className="h-10 flex-1" onClick={copy}>
              {copied ? <Check className="size-4" /> : <Copy className="size-4" />}{copied ? t("share.copied") : t("share.copy")}
            </Button>
            <Button variant="outline" className="h-10" disabled={busy} onClick={() => op("create")}>
              <RefreshCw className={cn("size-4", busy && "animate-spin")} />{t("share.reissue")}
            </Button>
            <Button variant="outline" className="h-10 w-10 px-0" disabled={busy} aria-label={t("share.delete")} onClick={() => op("revoke")}>
              <Trash2 className="size-4" />
            </Button>
          </div>
          <p className="text-xs leading-4 text-muted-foreground">{t("share.warning")}</p>
        </>
      ) : (
        <Button variant="secondary" className="h-10" disabled={busy} onClick={() => op("create")}>
          {busy ? <Loader2 className="size-4 animate-spin" /> : <Link2 className="size-4" />}{t("share.issue")}
        </Button>
      )}
    </section>
  );
}

function Tabs({ tab, set }: { tab: Tab; set: (t: Tab) => void }) {
  const items = [["home", t("tabs.home")], ["account", t("tabs.account")], ["settings", t("tabs.settings")]] as const;
  return (
    <nav className="rise flex h-10 flex-none gap-1 rounded-xl bg-secondary p-1" style={style(0)}>
      {items.map(([k, l]) => (
        <button key={k} type="button" onClick={() => set(k)} className={cn("flex-1 rounded-lg text-sm leading-5 transition-[background-color,color,scale] duration-150 active:scale-[0.96]", tab === k ? "bg-card text-foreground shadow-[0_0_0_1px_var(--border)]" : "text-muted-foreground hover:text-foreground")}>{l}</button>
      ))}
    </nav>
  );
}

// ログイン中の Lapount アカウントと、連携している Discord の詳細
function AccountCard({ p, email, botMissing }: { p: Profile | null; email: string; botMissing: boolean }) {
  const rows: [string, string][] = [
    [t("account.row.discordName"), p?.discordName || "—"],
    [t("account.row.discordUser"), p?.discordUser || "—"],
    [t("account.row.discordId"), p?.discordId || "—"],
    [t("account.row.discordEmail"), p?.discordEmail || "—"],
    [t("account.row.lapountEmail"), p?.lapountEmail || email || "—"],
    [t("account.row.lapountId"), p?.lapountId || "—"],
    [t("account.row.registered"), day(p?.createdAt ?? "")],
    [t("account.row.lastSignIn"), day(p?.lastSignIn ?? "")],
  ];
  return (
    <section className="flex flex-none flex-col gap-3 rounded-2xl bg-card p-5 shadow-[0_0_0_1px_var(--border)]">
      <div className="flex items-center gap-3">
        {p?.avatar ? <img src={p.avatar} alt="" className="size-10 flex-none rounded-full outline outline-1 -outline-offset-1 outline-white/10" /> : <span className="size-10 flex-none rounded-full bg-secondary" />}
        <div className="min-w-0">
          <p className="truncate text-sm leading-5 font-medium">{p?.discordName || p?.discordUser || t("account.loginLabel")}</p>
          <p className="truncate text-xs leading-4 text-muted-foreground">{p?.discordUser ? `@${p.discordUser}` : t("account.loginLabel")}</p>
        </div>
      </div>
      {p && !p.discordId && (
        <div role="alert" className="flex flex-col gap-3 rounded-xl bg-secondary p-4">
          <p className="flex items-start gap-2 text-sm leading-5">
            <TriangleAlert className="mt-0.5 size-4 flex-none text-[#d99a7c]" aria-hidden />
            <span>{t("account.noDiscord.title")}</span>
          </p>
          <Button variant="secondary" className="h-10 bg-card" onClick={() => openUrl("https://account.lapius7.com/dashboard")}>{t("account.noDiscord.button")}</Button>
          <p className="text-xs leading-4 text-muted-foreground">{t("account.noDiscord.note")}</p>
        </div>
      )}
      <p className={cn("text-xs leading-5", botMissing ? "rounded-xl bg-secondary p-3 text-sm text-foreground" : "text-muted-foreground")}>
        {botMissing ? t("account.botMissing") : t("account.botAdd")}
        <button type="button" className="ml-1 underline underline-offset-4 transition-colors duration-150 hover:text-foreground" onClick={() => openUrl("https://lapbot.lapius7.com/application/valorant-matchfound/setup")}>{t("account.howToAdd")}</button>
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
