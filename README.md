<p align="center">
  <img src="src-tauri/logo/appicon.png" alt="NanoWhisper" width="128" height="128">
</p>

<h1 align="center">NanoWhisper</h1>

<p align="center">
  <strong>Pure Whisper. Nothing else.</strong>
</p>

<p align="center">
  <a href="https://github.com/jicaiinc/nanowhisper/releases/latest"><img alt="Latest Release" src="https://img.shields.io/github/v/release/jicaiinc/nanowhisper?style=flat-square&color=1c1c1e"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/github/license/jicaiinc/nanowhisper?style=flat-square&color=1c1c1e&cacheSeconds=1"></a>
  <img alt="Platform" src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-333?style=flat-square">
</p>

<p align="center">
  <a href="https://github.com/jicaiinc/nanowhisper/releases/latest">Download</a>
</p>

<p align="center">
  English | <a href="README.zh.md">简体中文</a>
</p>

---

NanoWhisper is a minimal desktop speech-to-text app. Press a shortcut, speak, and the transcribed text is auto-pasted into your active application. That's it.

Powered by OpenAI Whisper API, Google Gemini API, DashScope, and Custom Whisper API endpoints. Built with Tauri v2.

## How It Works

Two recording modes (configurable in Settings):

**Toggle mode:**
1. Press `Right ⌘` on macOS / `Right Ctrl` on Windows
2. Speak
3. Press again to stop — text is transcribed and pasted instantly

**Hold mode:**
1. Press and hold `Right ⌘` / `Right Ctrl`
2. Speak
3. Release to send — text is transcribed and pasted instantly

## Features

- **One Shortcut** — Global hotkey to start/stop recording. No UI to navigate.
- **Auto-Paste** — Transcribed text goes straight to your cursor. No copy needed.
- **Waveform Overlay** — Minimal always-on-top visualizer while recording.
- **History** — All transcriptions saved locally with audio files for retry.
- **System Tray** — Runs quietly in the background.

## Build from Source

Prerequisites: [Node.js](https://nodejs.org/) and [Rust](https://rustup.rs/).

```bash
git clone https://github.com/jicaiinc/nanowhisper.git
cd nanowhisper
npm install
npm run tauri dev
```

## License

[Apache License 2.0](LICENSE)

---

<p align="center">
  纯粹的语音转文字，仅此而已。<br>
  <sub>&copy; 2025 <a href="https://github.com/jicaiinc">Jicai, Inc.</a></sub>
</p>
