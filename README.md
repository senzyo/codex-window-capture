# Codex Window Capture

**English** | [简体中文](README.zh-CN.md)

Use this tool to capture a single foreground window on Windows 10 when Computer Use can find the window and read its controls but screenshots repeatedly fail with `FrameArrived timed out` or `window capture timed out`. If native screenshots work, you can skip installation. The tool has been tested on Windows 10 22H2 x64 with Sky 0.7.18.

Codex continues to use Sky to read and interact with accessibility controls. When Codex needs an image, a standalone executable captures a frame with Windows Graphics Capture. The callback hands the frame to a receiving thread and returns immediately. The receiving thread converts the frame to PNG. The tool works without modifying Sky or the desktop app. Agents can use it if they read `AGENTS.md` and can run PowerShell locally.

## Installation

1. Download `codex-window-capture-windows-x64.zip` from [Releases](https://github.com/senzyo/codex-window-capture/releases/latest). The portable ZIP includes the executable, the capture script, and the agent instruction. You can use it without installing Rust. Open PowerShell in the download directory and extract the ZIP.

   ```powershell
   $install = Join-Path $env:LOCALAPPDATA 'codex-window-capture'
   Expand-Archive -LiteralPath .\codex-window-capture-windows-x64.zip -DestinationPath $install
   ```

2. Append the English instruction from the bundled [AGENTS.md](AGENTS.md) to your user-level `%USERPROFILE%\.codex\AGENTS.md`, keeping any existing instructions. If you set a custom `CODEX_HOME`, use the `AGENTS.md` in that directory. If you extracted the ZIP elsewhere, update the script path in the instruction to match.
3. Start a new Codex session and ask it to inspect a window as usual. When the agent needs a screenshot, it runs the local script, which saves the PNG in the installation directory’s `screenshots\` folder. No additional skill is required.

## Test a capture

```powershell
& "$env:LOCALAPPDATA\codex-window-capture\scripts\capture-window.ps1" `
    -ApplicationPath "$env:WINDIR\System32\notepad.exe" -WindowTitle 'Untitled - Notepad'
```

Use the actual executable path and the full window title, including any localized text. When running the command from a terminal, switch back to the target window within two seconds of pressing Enter. On success, the script prints the PNG path. The target window must be visible on the interactive desktop, in the foreground, and not minimized. If two visible windows from the same application have the same title, the tool refuses the capture. The host’s desktop access permissions still apply.

The tool returns an image only. Coordinate clicks that require a Sky screenshot ID still depend on Sky’s native capture. If you downloaded only `window-capture.exe`, run `.\window-capture.exe <absolute-application-path> <full-window-title> <new-png-path>` from its directory.

## Build and release

On Windows x64, install the Rust MSVC toolchain, Visual Studio C++ Build Tools, and the Windows SDK. Run this command from the repository root.

```powershell
cargo build --locked --release --target x86_64-pc-windows-msvc
```

The capture script in the repository also looks for this local build. When a maintainer pushes a version tag such as `v0.1.0` to GitHub, the [release workflow](https://github.com/senzyo/codex-window-capture/blob/main/.github/workflows/release.yml) builds the Windows x64 executable and publishes it with the portable ZIP and `SHA256SUMS.txt`. To verify a download, compare its `Get-FileHash -Algorithm SHA256` output with the matching entry in `SHA256SUMS.txt`.

This project uses the [MIT License](LICENSE). Dependency notices are in [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt).
