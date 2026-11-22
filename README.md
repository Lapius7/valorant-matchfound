<div align="center">

<p align="center">
  <img src="docs/images/banner.png" alt="VALORANT Matchfound" width="100%">
</p>

<p align="center"><b>English</b> | <a href="README.ja.md">日本語</a></p>

<h1 align="center">VALORANT Matchfound</h1>

<h3 align="center">A Discord DM the moment your VALORANT match is found.</h3>

<p align="center">A Windows tray app that sends you a Discord DM when a VALORANT match is found. The DM shows the mode, the map and each player's agent pick, and it keeps updating until the match ends.</p>

<p align="center">
  <a href="https://github.com/Lapius7/valorant-matchfound/releases"><img alt="Release" src="https://img.shields.io/github/v/release/Lapius7/valorant-matchfound?label=release&style=flat-square&color=e8833a&logo=github"></a>
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%2010%2B-0078d4?style=flat-square&logo=windows&logoColor=white">
  <a href="https://tauri.app"><img alt="Tauri" src="https://img.shields.io/badge/Tauri-v2-24c8db?style=flat-square&logo=tauri&logoColor=white"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/Lapius7/valorant-matchfound?style=flat-square&color=97ca00"></a>
  <a href="https://github.com/Lapius7/valorant-matchfound/stargazers"><img alt="Stars" src="https://img.shields.io/github/stars/Lapius7/valorant-matchfound?style=flat-square&color=e3b341&logo=github"></a>
  <a href="https://github.com/Lapius7/valorant-matchfound/commits/main"><img alt="Last commit" src="https://img.shields.io/github/last-commit/Lapius7/valorant-matchfound?style=flat-square&color=3b82f6"></a>
</p>

<p align="center"><a href="#install">Install</a> · <a href="#how-it-works">How it works</a> · <a href="#share-url">Share URL</a> · <a href="#privacy">Privacy</a></p>

</div>

---

> [!IMPORTANT]
> **Unofficial.** This is a personal project. It is not made, endorsed or supported by Riot Games. VALORANT is a trademark of Riot Games, Inc.

![VALORANT Matchfound](docs/images/hero.png)

## Features

- **DM on match found.** You are notified through Lapbot, even when you are away from the PC.
- **Live Embed.** Mode, map image, remaining time and each player's pick (selecting / locked) are edited into the same message. It switches to "in game" and "ended" by itself.
- **Share URL.** Issue a URL from the app and anyone who has it can watch the match in real time. You can regenerate or delete it at any time. The page is hidden from search engines.
- **Restart safe.** The message ID is saved, so restarting the app or the bot continues the same message (a lost message is recreated).
- **Auto update.** Checks on start, every 6 hours, and from the tray menu. The installer closes the running app by itself.
- **Quiet tray app.** Can start with Windows and begins watching when VALORANT starts.
- **11 languages.** The app UI has English, Japanese, German, Spanish, French, Italian, Korean, Brazilian Portuguese, Russian and Simplified and Traditional Chinese files.

## Install

Requires Windows 10 or later (64-bit).

1. Download the installer from [Releases](https://github.com/Lapius7/valorant-matchfound/releases/latest).
2. Run it and open the app.
3. Install [Lapbot](https://discord.com/oauth2/authorize?client_id=1470053949263577189&integration_type=1&scope=applications.commands) to your Discord account (user install). It is needed to receive DMs.
4. On the **Settings** tab, press the login button and approve it in your browser with your [Lapount](https://account.lapius7.com/dashboard) account. A Discord account must be linked to Lapount.
5. Press **Test DM** to check that a message arrives. If it does not, allow DMs from Lapbot in your Discord privacy settings.

## How it works

```
VALORANT - Riot Client local API -> app -> Lapbot API -> Discord DM
                                      \-> share page (SSE) -> viewers
```

- The app reads your own presence from the Riot Client local API (through its lockfile) and, during agent select, the pre-game match of your own account. It never reads the game screen or memory and does not touch the game process.
- Names and images come from [valorant-api.com](https://valorant-api.com).
- Login uses Lapount's device-code flow. Only the refresh token and your email are stored, in Windows Credential Manager.
- The app sends only match information (mode, map, players' names and agents) to the Lapbot API, which edits the DM.
- State changes are logged to `%APPDATA%\com.lapius7.valorant-matchfound\logs\states.log` (for diagnosing Riot format changes).

## Share URL

The **Share URL** card on the Home tab issues `https://lapbot.lapius7.com/systems/valorant-matchfound/<token>`.

> [!WARNING]
> Anyone who knows the URL can see the player names of your current match. Do not post it publicly.

- One URL per user. **Regenerate** invalidates the old one, **Delete** closes the page for everyone.
- The page sends `noindex`, `no-referrer` and `no-store`.

## Privacy

<details>
<summary>What is sent and stored</summary>

| Data | Where it goes |
|---|---|
| Match info (mode, map, player names, agents) | Lapbot API, then your Discord DM and the share page |
| Finished match results (map, each player's name, agent, K/D/A, ACS, rank change) | Stored on the Lapbot server (MySQL) so the same match is not fetched again, and shown in your match history on the site |
| Lapount refresh token and email | Windows Credential Manager on your PC |
| Usage count | While the app runs and you are logged in, it pings Lapbot every 5 seconds. Your Lapount ID, Discord ID, first/last-seen times and ping/DM counts are kept for the install and online counts and for troubleshooting. Only anonymous totals are public; per-user details are visible to the developer only |
| Riot credentials | Used only to talk to Riot on your PC. Never sent to Lapbot |

</details>

## Safety

> [!NOTE]
> The app only uses information available to your own account and does not automate or modify the game. Even so, it is not a Riot product, and you use it at your own risk.

> [!NOTE]
> Not verified for this README: the installer and the running app (it needs Windows and VALORANT). The 6-hour update check, the 5-second ping and the 11 language files were checked in the source.

## Build from source

Requires pnpm, Rust, and, for cross builds from WSL, the `x86_64-pc-windows-gnu` target.

```sh
pnpm install
pnpm build                                   # frontend, required before cargo
cd src-tauri
cargo build --release --target x86_64-pc-windows-gnu
```

The installer is built with [Inno Setup 6](https://jrsoftware.org/isinfo.php):

```sh
ISCC.exe /DAppVersion=0.0.15 installer/valorant-matchfound.iss
```

Stack: Tauri v2, Rust, React, Vite, Tailwind CSS v4, shadcn/ui.

## License

[MIT](LICENSE) © Lapius7
