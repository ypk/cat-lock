# CatLock – Cross‑Platform Roadmap

**Version:** 0.3 – 2026‑07‑10

## 1️⃣ High‑Level Goal
Provide a **single‑source Rust code‑base** that delivers the same locking experience on **Windows** *and* **Linux**, with a polished UI, hot‑key support, global input blocking, fullscreen overlay, and power‑inhibition.

---
## 2️⃣ Current Status (as of 2026‑07‑10)

| Area | Platform | Implementation | Notes |
|------|----------|----------------|-------|
| Core (settings, UI, logging, CLI) | Windows & Linux | ✅ Complete (shared) | Stable, builds on both OSes. |
| Input interceptor | Windows | ✅ `Win32InputHook` (global hotkey, input block) | Works via Win32 API. |
| Input interceptor | Linux | ✅ `LinuxInputGrab` (x11rb XGrabKeyboard/Pointer) | Hooks X11 inputs. |
| Overlay window | Windows | ✅ `SlintOverlay` (Slint cross-platform UI) | Fullscreen opaque/transparent. |
| Overlay window | Linux | ✅ `SlintOverlay` (Slint cross-platform UI) | Fullscreen opaque/transparent. |
| Power inhibitor | Windows | ✅ `Win32PowerInhibitor` (calls `SetThreadExecutionState`) | |
| Power inhibitor | Linux | ✅ `LinuxPowerInhibitor` (uses `zbus` logind Inhibit) | |
| CI / Tests | Both | ✅ Complete | Cross-platform tests and GitHub Actions CI in place. |
| Docs & README | Both | ✅ Windows and Linux described | README contains installation instructions. |
| **Configurable logging** | Both | ✅ Complete (`src/logging.rs`) | Console + file output, log level, timestamps — all from `config.json`. |
| **Console window control** | Windows | ✅ Complete | `show_console` config key; defaults to off on startup. |
| **Double-toggle bug fix** | Windows | ✅ Fixed | Win32 LL hook + `global-hotkey` double-fire on unlock. |
| **PawSense (Auto-Detect Cat)** | Both | ✅ Complete | Background key heuristics via Windows hook and X11 polling. |
| System notifications | Both | 🔲 Planned (F2) | Desktop toast/notification on lock and unlock events. |
| Code signing | Windows | ❌ Cancelled (F3) | Dropped (Requires paid certificate/manual verification). |
| Auto-update check | Both | ❌ Cancelled (F4) | Dropped (App is stable/feature-complete, avoids HTTP bloat). |

---
## 3️⃣ Required Work Items (grouped by platform)

### 3.1 Linux Platform (priority: high)

| # | Work Item | Description | Estimated Effort |
|---|-----------|-------------|------------------|
| L1 | **Add dependencies** | `zbus` for D‑Bus power inhibition, optionally `winit` for overlay, keep `x11rb`. | ✅ Done |
| L2 | **PowerInhibitor** (`src/platform/linux/power.rs`) | Replace stub with real D‑Bus call to `org.freedesktop.login1.Manager.Inhibit`. Store inhibitor handle, implement `Drop`. | ✅ Done |
| L3 | **InputInterceptor** (`src/platform/linux/input_grab.rs`) | Use `x11rb` to connect to X server, register global hotkey (`Ctrl+Shift+L`), call `XGrabKeyboard`/`XGrabPointer` to block input, release on unlock, provide `Drop`. | ✅ Done |
| L4 | **OverlayWindow** (`src/platform/slint_overlay.rs`) | Use Slint framework to create a cross-platform GUI overlay window; implement show/hide/is_visible. | ✅ Done |
| L5 | **Linux `mod.rs`** | Export the new structs, re‑export trait implementations, ensure `create_*` factories compile. | ✅ Done |
| L6 | **Unit & integration tests** | Mock X11 where possible; test hot‑key registration, input grab/release, overlay visibility, power inhibition via D‑Bus. | ✅ Done |
| L7 | **Documentation** | Update README with Linux installation steps, required X server/Wayland, permissions (e.g., `Xorg` access), usage examples. | ✅ Done |
| L8 | **CI pipeline** | Add GitHub Actions matrix for Windows + Ubuntu, run tests, produce release artifacts. | ✅ Done |

**Total Linux effort:** ~27 hours (≈ 3.5 working days).

### 3.2 Windows Platform (maintenance)

| # | Work Item | Description | Estimated Effort |
|---|-----------|-------------|------------------|
| W1 | **Code cleanup** | Remove any dead code/comments, ensure `Drop` implementations are safe, add `#[cfg(test)]` unit tests for Windows modules. | ✅ Done |
| W2 | **Binary size audit** | Verify final binary stays ≤ 4 MB; strip symbols, ensure `panic = "abort"` is set. | ✅ Done |
| W3 | **Documentation** | Add Windows‑specific troubleshooting (e.g., hot‑key conflicts). | ✅ Done |

**Total Windows effort:** ~4 hours.

### 3.3 Cross‑cutting Tasks

| # | Work Item | Description | Estimated Effort |
|---|-----------|-------------|------------------|
| C1 | **Version bump & release** | Update `Cargo.toml` version, tag a release, produce Windows & Linux binaries. | ✅ Done |
| C2 | **License audit** | Ensure GPL‑3.0 compliance for any new dependencies. | ✅ Done |
| C3 | **User feedback loop** | Publish a pre‑release, collect user reports, fix any platform‑specific bugs. | Ongoing |
| C4 | **Tray Icons** | Design SVG lock/unlock icons, export to PNG, embed via `include_bytes!`, decode via `image` crate. | ✅ Done |

**Total cross‑cutting effort:** ~4 hours.

### 3.4 Polish & Feature Parity

| # | Work Item | Description | Estimated Effort |
|---|-----------|-------------|------------------|
| F1 | **Overlay opacity slider** | ❌ Cancelled | Expose `overlay_alpha: u8` in `config.json`. (Cancelled by user request, opacity functionality removed entirely). |
| F5 | **PawSense (Auto-Detect Cat)** | ✅ Done | Automatically lock the screen if a cat walks on the keyboard by detecting 4+ non-modifier keys pressed within 50ms. Works on Windows via `WH_KEYBOARD_LL` hook and Linux via `XQueryKeymap` polling. (Permanently ON by design, UI toggle removed). |
| F2 | **System notifications** | ✅ Done | On lock and unlock events, fire a desktop notification: Windows via `ToastNotification` (WinRT) or a simpler `Shell_NotifyIcon` balloon tip; Linux via `notify-send` (D‑Bus `org.freedesktop.Notifications`). Add `notifications_enabled: bool` to `config.json` (default `true`) and a tray menu toggle. |
| F3 | **Code signing** | ❌ Cancelled | Skipped: Requires paid certificate or manual human verification for open-source foundations. |
| F4 | **Auto-update check** | ❌ Cancelled | Skipped: The app is extremely stable and feature-complete. Adding HTTP capabilities (`ureq`/`reqwest`) would unnecessarily bloat the binary size for a feature that is rarely needed. |

**Total polish effort:** ~13 hours.

### 3.5 Bug Fixes (completed)

| # | Bug | Root Cause | Fix | Status |
|---|-----|------------|-----|--------|
| B1 | **Double-toggle on unlock** | The Win32 LL hook (`keyboard_proc`) and the `global-hotkey` crate both fire for the same `Ctrl+Shift+L` keypress. The hook posts `WM_CATLOCK_TOGGLE` (and swallows the key), but `global-hotkey` has already queued a `GlobalHotKeyEvent` via its own OS registration. The event loop processed both within ~73 ms, causing an immediate re-lock after unlock. | After handling `WM_CATLOCK_TOGGLE`, drain `hotkey_rx` with `while hotkey_rx.try_recv().is_ok() {}` before continuing the loop. | ✅ Fixed |

---
## 4️⃣ Timeline (working‑day estimate, 8 h/day)

| Week | Day | Tasks |
|------|-----|-------|
| **Week 1** | Mon | L1, L2 (setup dependencies, power inhibitor). |
| | Tue | L2 (complete, add Drop), L3 (initial hot‑key registration). |
| | Wed | L3 (input grab implementation), L4 (overlay skeleton). |
| | Thu | L4 (full overlay, privacy mode), L5 (mod.rs). |
| | Fri | L6 (write unit tests for power & input). |
| **Week 2** | Mon | L6 (integration test, CI config). |
| | Tue | L7 (README Linux section). |
| | Wed | C1 (release prep), C2 (license checks). |
| | Thu | W1 (Windows code cleanup & tests). |
| | Fri | W2 (binary size audit), W3 (Windows docs). |
| **Week 3** | Mon | Final CI run, fix any CI failures. |
| | Tue | Publish official v1.0.0 release (Windows & Linux binaries). |
| | Wed‑Fri | Gather early user feedback, address blocker bugs. |

> **Note:** Timeline assumes a single developer full‑time. With multiple contributors, tasks can be parallelised to shorten the schedule.

---
## 5️⃣ Risk Assessment & Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| **X11 vs Wayland fragmentation** | Users on Wayland may see the lock not work. | Medium | Start with X11‑only (simpler). Later add Wayland support using `winit` or a Wayland‑specific crate. |
| **D‑Bus permission errors** | Power inhibition may fail on some distros. | Low‑Medium | Gracefully fall back to a warning; document need for `systemd-logind` session. |
| **Global hot‑key conflicts** | `Ctrl+Shift+L` might be taken by another app. | Low | Make the hot‑key configurable via `config.json`. |
| **Binary size blow‑up** | Adding `winit` or heavy GUI libs could increase size > 5 MB. | Medium | Prefer raw X11 for overlay; add `winit` only if Wayland support is required. |
| **CI environment lacking X server** | Automated tests may fail on headless CI runners. | Medium | Use `xvfb-run` in CI to provide a virtual X server. |
| **WinRT / Toast API availability** | `ToastNotification` requires Windows 8+; balloon tips are simpler but deprecated in Win11. | Low | Default to balloon tip; add WinRT toast behind a feature flag. |
| **Code-signing certificate cost/renewal** | Commercial EV certs are expensive (~$300/yr). | Medium | Apply for SignPath Foundation free certificate; document self-signed fallback. |
| **GitHub API rate limiting** | Unauthenticated requests are limited to 60/hr per IP. | Low | Cache the last-checked timestamp in `config.json`; only poll once per 24 h. |

---
## 6️⃣ Deliverables

| Artifact | Description |
|----------|-------------|
| `src/platform/linux/input_grab.rs` | Implements `LinuxInputGrab` (global hot‑key, input block). |
| `src/platform/linux/overlay.rs` | Implements `LinuxOverlay` (fullscreen window). |
| `src/platform/linux/power.rs` | Real D‑Bus power inhibitor. |
| `src/platform/linux/mod.rs` | Re‑exports the three structs. |
| Updated `Cargo.toml` | Linux‑specific dependencies (`zbus`, optional `winit`); `fern`+`chrono` replacing `env_logger`. |
| Unit tests (`tests/linux_*`) | Cover each trait implementation. |
| GitHub Actions workflow (`.github/workflows/ci.yml`) | Build & test on Windows + Ubuntu. |
| Updated `README.md` | Linux installation & usage guide; full logging config reference. |
| Release assets | `catlock-windows.exe`, `catlock-linux` binaries, checksums. |
| `src/logging.rs` *(new)* | Multi-output logger: `fern` dispatcher, ISO-8601 timestamps via `chrono`, console + file output, runtime log level — all driven by `LoggingSettings` in `config.json`. |
| `LoggingSettings` in `src/settings.rs` | Five configurable keys: `show_console`, `log_to_console`, `log_to_file`, `log_level`, `log_timestamps`. `#[serde(default)]` preserves backwards compatibility with old config files. |
| Double-toggle guard in `src/main.rs` (B1) | `hotkey_rx` is drained immediately after `WM_CATLOCK_TOGGLE` to discard the duplicate `GlobalHotKeyEvent`. |
| `overlay_alpha` in `config.json` & tray submenu (F1) | Configurable opacity, live preview. |
| Notification module `src/notify.rs` (F2) | Lock/unlock desktop notifications, Win + Linux. |
| Signed Windows binary + `release.yml` signing step (F3) | Code-signed `.exe` via SignPath or self-signed. |
| `src/updater.rs` + tray "Check for updates" item (F4) | Background GitHub Releases API poll, 24 h cache. |

---
## 7️⃣ Next Action

**Completed since v1.0.0 (shipped in v1.0.1):**
- [x] **Logging** — Configurable console/file/level/timestamps via `config.json` (`src/logging.rs`)
- [x] **Silent startup** — Console hidden by default at startup via `windows_subsystem = "windows"`
- [x] **PawSense Auto-Detect** — Implemented for both Windows & Linux (`F5`)
- [x] **B1** — Double-toggle bug fixed: `hotkey_rx` drained after `WM_CATLOCK_TOGGLE`
- [x] **F2** — System notifications on lock/unlock

**Immediate priorities (v1.1.0):**
- 🎉 **Roadmap Complete!** All planned features have been implemented or explicitly descoped to preserve the app's lightweight nature.

**Open questions:**
- None!

---
*Roadmap authored by Antigravity (AI coding assistant), based on the current repository state.*
