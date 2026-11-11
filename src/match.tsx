import { useEffect, useRef, useState } from "react";
import { Check } from "lucide-react";
import { cn } from "@/lib/utils";
import { t } from "@/i18n/index";

export type MatchPlayer = { icon: string; name: string; agent: string; locked: boolean };
export type MatchInfo = { mode: string; map: string; mapImg: string; endsAt: number; players: MatchPlayer[] };

// 残り時間の円。全体の長さは、最初に見えた残り秒数とする
function Countdown({ endsAt }: { endsAt: number }) {
  const [now, setNow] = useState(() => Date.now() / 1000);
  const total = useRef(Math.max(1, endsAt - Date.now() / 1000));
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now() / 1000), 200);
    return () => clearInterval(t);
  }, []);
  const left = Math.max(0, endsAt - now);
  const R = 22;
  const C = 2 * Math.PI * R;
  const ratio = Math.min(1, left / total.current);
  const urgent = left <= 10;
  return (
    <div className={cn("relative grid size-14 flex-none place-items-center", urgent && "urgent")} aria-label={t("match.ariaLeft", { left: Math.ceil(left) })}>
      <svg viewBox="0 0 52 52" className="absolute inset-0 -rotate-90">
        <circle cx="26" cy="26" r={R} fill="none" stroke="var(--border)" strokeWidth="3" />
        <circle cx="26" cy="26" r={R} fill="none" stroke={urgent ? "var(--signal)" : "var(--foreground)"} strokeWidth="6" opacity=".12" strokeDasharray={C} strokeDashoffset={C * (1 - ratio)} style={{ filter: "blur(4px)", transition: "stroke-dashoffset 220ms linear" }} />
        <circle
          cx="26" cy="26" r={R} fill="none" strokeWidth="3" strokeLinecap="round"
          stroke={urgent ? "var(--signal)" : "var(--foreground)"}
          strokeDasharray={C} strokeDashoffset={C * (1 - ratio)}
          style={{ transition: "stroke-dashoffset 220ms linear, stroke 300ms" }}
        />
      </svg>
      <span className="flex flex-col items-center">
        <span className="font-mono text-lg leading-5 font-medium tabular-nums">{Math.ceil(left)}</span>
        <span className="text-[10px] leading-3 text-muted-foreground">{t("match.seconds")}</span>
      </span>
    </div>
  );
}

function Portrait({ icon, locked }: { icon: string; locked: boolean }) {
  return (
    <span
      className={cn(
        "relative grid size-10 flex-none place-items-center overflow-hidden rounded-full bg-background/60 transition-[box-shadow] duration-300",
        locked ? "shadow-[0_0_0_2px_var(--signal),0_0_16px_-2px_var(--signal)]" : "shadow-[0_0_0_1px_var(--border)]",
      )}
    >
      {icon ? (
        // key で画像が変わるたびに作り直し、小さく・ぼかした状態から現れる
        <img key={icon} src={icon} alt="" className="agent-in size-full object-cover" />
      ) : (
        <span className="size-3 rounded-full bg-muted-foreground/40 pulse-soft" />
      )}
    </span>
  );
}

export function MatchPanel({ m, ingame }: { m: MatchInfo; ingame: boolean }) {
  return (
    <div className="match-in mt-1 flex flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-1">
          <span className="truncate text-lg leading-6 font-medium tracking-[-0.01em]">{m.map || t("match.map")}</span>
          {m.mode && (
            <span className="flex h-6 w-fit items-center rounded-full bg-background/60 px-2.5 text-xs leading-4 text-muted-foreground shadow-[0_0_0_1px_var(--border)]">
              {m.mode}
            </span>
          )}
        </div>
        {!ingame && m.endsAt > 0 && <Countdown key={m.endsAt} endsAt={m.endsAt} />}
      </div>
      <ul className="flex flex-col gap-1.5">
        {m.players.map((p, i) => (
          <li
            key={p.name}
            className={cn(
              "row-in relative flex h-14 items-center gap-3 overflow-hidden rounded-xl bg-background/50 px-2 backdrop-blur-sm transition-[background-color,box-shadow] duration-300",
              p.locked ? "shadow-[0_0_0_1px_color-mix(in_oklch,var(--signal)_50%,transparent)]" : "shadow-[0_0_0_1px_var(--border)]",
            )}
            style={{ "--i": i } as React.CSSProperties}
          >
            {p.locked && <span key="sheen" className="sheen" aria-hidden />}
            <Portrait icon={p.icon} locked={p.locked} />
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="truncate text-sm leading-5">{p.name}</span>
              <span key={p.agent} className={cn("text-label truncate text-xs leading-4", p.agent ? "text-muted-foreground" : "text-muted-foreground/60")}>
                {p.agent || t("match.agentWaiting")}
              </span>
            </span>
            <span
              className={cn(
                "flex h-6 flex-none items-center gap-1 rounded-full px-2 text-xs leading-4 transition-[background-color,color] duration-300",
                p.locked ? "bg-signal text-white" : p.agent ? "bg-foreground/10 text-foreground" : "text-muted-foreground/60",
              )}
            >
              {p.locked ? <><Check key="c" className="pop size-3.5 flex-none" />{t("match.locked")}</> : p.agent ? t("match.selecting") : t("match.waiting")}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
