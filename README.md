<p align="center">
  <img src="https://waxum.imtaqin.id/img/logo.png" alt="waxum agent" width="120" />
</p>

<h1 align="center">waxum agent</h1>

<p align="center">
  Voice-driven WhatsApp assistant on top of <a href="https://github.com/imtaqin/waxum">waxum</a> — a Jarvis-style app that reads incoming messages aloud and lets you reply or manage chats by voice. Desktop and Android, one Tauri codebase.
</p>

---

## What it does

- Connects to a waxum instance — either a **remote URL** (self-hosted server) or a **bundled binary** you point it at (spawned as a local child process, talking to `127.0.0.1`).
- Streams incoming WhatsApp messages live over waxum's `/events/tail` SSE endpoint.
- **Reads incoming messages aloud** (ElevenLabs TTS) — on by default, toggle in settings.
- **Voice commands** via push-to-talk (ElevenLabs STT): hold the mic button, speak, release.
  - "baca pesan" / "bacain pesan dari budi" — reads the latest message, optionally scoped to a contact
  - "balas ke budi bilang otw" / "kirim pesan ke budi bilang otw" — sends a text message
  - "list sesi" — how many active conversations the app has seen this run
- Design matches the rest of the waxum ecosystem (waxum-studio's charcoal/emerald palette, Geist font).

## Stack

Tauri v2 (Rust backend + Vue 3 / Tailwind frontend, one codebase for desktop and Android). Rust side owns the waxum REST client, the SSE bridge, the ElevenLabs client, and the bundled-binary process; the frontend owns settings, the voice UI, and a small regex-based command parser (`src/lib/commandParser.ts`) — commands here are a handful of fixed shapes, not open-ended NL, so no LLM in the loop for command parsing.

```
src/                    Vue frontend
├── App.vue               orchestrator: settings gate, message log, voice bar
├── components/
│   ├── SetupView.vue        connection mode, waxum + ElevenLabs settings
│   └── VoiceBar.vue         push-to-talk button
└── lib/
    ├── settings.ts           tauri-plugin-store persistence
    ├── waxum.ts               typed wrappers around the Tauri commands
    ├── voice.ts               MediaRecorder capture + TTS playback
    └── commandParser.ts       transcript -> intent

src-tauri/src/          Rust backend
├── waxum.rs               waxum REST client (status, sessions, send, search)
├── elevenlabs.rs          ElevenLabs TTS + STT
├── events.rs              SSE tail -> Tauri events, with reconnect/backoff
├── bundled.rs             spawn/kill a local waxum binary
└── commands.rs            #[tauri::command] surface the frontend calls
```

## Setup

```bash
npm install
npm run tauri dev      # desktop
```

On first launch you land on the settings screen:

- **Remote URL mode** — waxum base URL (e.g. `https://waxum-api.example.com/api/v1`) + bearer token + session id.
- **Bundled binary mode** — leave the path empty and it auto-downloads the latest [waxum release](https://github.com/imtaqin/waxum/releases) for your OS/arch (linux-amd64/arm64, windows-amd64) into the app data dir and spawns it on `127.0.0.1:3451`; point it at your own binary instead if you'd rather not auto-download.
- ElevenLabs API key + voice id (defaults to voice `21m00Tcm4TlvDq8ikWAM` — swap for your own).

Both connections have a "Test" button before you save.

### Android

```bash
npm run tauri android init   # one-time, needs Android SDK + NDK installed
npm run tauri android dev
```

Bundled-binary mode is desktop-only (no child-process spawning on Android) — the Rust side already reflects this (`bundled.rs` uses `tokio::process::Command`, which is a no-op target on mobile); the settings screen should be pointed at remote URL mode on Android.

### Desktop system dependencies (Linux)

Tauri's Linux target needs `webkit2gtk-4.1` + `libsoup3` dev headers:

```bash
sudo dnf install webkit2gtk4.1-devel libsoup3-devel gtk3-devel librsvg2-devel   # Fedora
sudo apt install libwebkit2gtk-4.1-dev libsoup-3.0-dev libgtk-3-dev librsvg2-dev  # Debian/Ubuntu
```

## Notes / known limitations (v1)

- The command parser is a fixed set of Indonesian regex patterns
  (`src/lib/commandParser.ts`), not free-form NL — extend the pattern
  lists there rather than reaching for an LLM unless open-ended commands
  become a real requirement.
- Contact resolution ("balas ke budi") matches against chats the app has
  *seen* in this run (via the SSE stream), not waxum's full contact list —
  say a contact's message once before addressing them by voice, or extend
  `resolveChat()` in `App.vue` to hit `waxum_search` as a fallback.
- Auto-reconnect and backoff on the SSE stream is handled Rust-side
  (`events.rs`); the bundled-binary process is not auto-restarted if it
  crashes — restart it from settings.
