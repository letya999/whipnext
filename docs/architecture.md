---
description: "whipnext in its environment: pack, overlay, settings, inject."
last_verified: "2026-08-21"
---

# Architecture

Two zoom levels: the system among other processes, then the parts inside the binary. Types and functions stay in the code.

## System in the environment

```mermaid
flowchart LR
    U["User"]
    S["whipnext"]
    H["Focused harness"]
    R["Herdr"]
    P["Pack + catalog"]

    U -->|"click overlay / settings"| S
    S -->|"type phrase"| H
    S -->|"prompt argv"| R
    S -->|"read models, phrases, wav"| P

    style S fill:#1168bd,color:#fff
```

## Parts

```mermaid
flowchart TD
    U["User"]
    subgraph S["whipnext"]
        UI["Settings WebView"]
        OV["Overlay engine"]
        PK["Pack + catalog"]
        WH["Whip state machine"]
        IN["Inject"]
    end
    H["Harness window / Herdr pane"]
    ST["~/.whipnext/settings.json"]

    U -->|"IPC JSON"| UI
    UI -->|"live Settings"| OV
    UI -->|"import/replace"| PK
    OV -->|"click"| WH
    WH -->|"payload"| IN
    IN -->|"keys or herdr prompt"| H
    UI -->|"save"| ST
    OV -->|"read"| ST
    PK -->|"skins, phrases"| OV
```

| Part | Responsibility | Tech | Talks to |
| --- | --- | --- | --- |
| Settings UI | Catalog, phrases, harness table, overlay scale | `assets/ui` + `src/ui.rs` + WebView host | IPC JSON, pack, settings file |
| Overlay engine | Skin blit, click, tick | `src/overlay.rs` + Win32/minifb host | pack, whip, inject |
| Pack | List/merge models, import, replace, phrases | `src/pack.rs`, `src/video.rs`, `src/gltf.rs` | `assets/pack`, `~/.whipnext/catalog` |
| Whip | Idle → crack → inject | `src/whip.rs` | injector + audio traits |
| Inject | Sanitize phrase, pid keys/paste, Herdr fallback | `src/inject.rs` | OS SendInput / clipboard / `herdr prompt` |

## Key flow: click injects a phrase

```mermaid
sequenceDiagram
    autonumber
    actor U as User
    participant OV as Overlay
    participant WH as Whip
    participant IN as Inject
    participant H as Harness

    U->>OV: click opaque pixel
    OV->>WH: handle_click
    WH->>WH: crack frames + hit sound
    WH->>IN: submit sanitized phrase
    IN->>H: type keys, layout keys, or paste phrase
    WH->>OV: return to idle
```

If there is no target, whip records nothing and the overlay still plays. If pid inject fails, Herdr pane is the fallback. Slash or empty phrase becomes `next`.

## Data

- Bundled pack: `assets/pack/models/<id>/`, `assets/pack/sounds/`, `assets/pack/phrases.json`.
- User catalog: `~/.whipnext/catalog/` (same layout). Same id overrides bundled. A folder import keeps the original files under `models/<id>/_source/`; the manifest records the copied model folder.
- Settings: `~/.whipnext/settings.json` (model, per-harness prefs, phrase/sound overrides, scale).
- Snapshot to the WebView includes `layout` (overlay size, tick, scale bounds) from `src/layout.rs`, plus each model's copied folder and file list. Editing a bundled model first creates a catalog override.

Media the loader actually plays: PNG/JPEG, GIF, SVG (via sibling PNG), glTF/GLB (software raster on overlay, WebGL preview in settings), video (`mp4`/`mov`/`webm`/`avi` decoded by ffmpeg into frames). Extensions that are not in that set fail import.

## External dependencies

| Dependency | Why | If it fails | Backup |
| --- | --- | --- | --- |
| WebView2 / WebKit / WebKitGTK | Settings window | Settings UI will not open | CLI `--detect` / `--demo` |
| xdotool + X11 | Linux cursor, focus, and input | Linux overlay cannot target a session | Use Windows/macOS, or `--demo` |
| macOS Accessibility | CoreGraphics keyboard input and System Events focus | macOS input/focus is unavailable | Grant Accessibility permission, or `--demo` |
| ffmpeg | Video skins | Video import/load errors | Use image/GIF/glTF characters |
| Herdr | Pane inject when pid typing fails | pid path still used | None for pane-only targets |
| Focused harness | Place to type | Overlay hides or click no-ops | Focus a listed app |

## Intentionally absent

- Slash-command launcher. Reason: payload is a phrase, default `next`.
- Full glTF runtime or `obj`/`fbx` loaders. Reason: overlay is a layered bitmap.
- Separate settings daemon. Reason: one process; closing settings stops the overlay.
- Memory-bank trees (`specs/`, `.work/`, `docs/adr`). Reason: four filled guides plus `AGENTS.md` are the docs surface.
