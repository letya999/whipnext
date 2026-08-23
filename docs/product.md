---
description: "Problem, users, and what whipnext changes when a coding agent is waiting."
last_verified: "2026-08-21"
---

# Product

## Problem

A coding-agent session stops and waits for a short go-ahead. The user is already looking at the editor or terminal. Opening a palette, typing a slash command, or switching windows costs more than the payload itself.

## Who it is for

| Who | What they are trying to do | What gets in the way | How they cope today |
| --- | --- | --- | --- |
| Someone driving Grok, Claude, Codex, or another listed harness | Tell the focused session to continue | Hands leave the mouse; slash commands differ per tool | Type `next` by hand, or use a tool-specific shortcut |

If typing `next` is already comfortable, this product is optional. It exists for the click-and-stay-on-the-cursor path.

## What changes

Clicking the overlay character plays the hit animation and sound, then types that character's phrase into the focused harness. The default phrase is `next`. Closing the settings window also stops the overlay.

## Why not a ready-made launcher

| Alternative | Why it does not fit |
| --- | --- |
| Slash-command palettes | Each harness uses a different command language. whipnext never sends `/...`. |
| AutoHotkey snippets | No per-character skin, no pack drop-in, no harness detect. |
| Do nothing | The user keeps typing `next` and never gets a visible confirm that the click landed. |

## Main flow

Who: the person at the keyboard. Goal: continue the focused agent.

| Step | User | System | What they see |
| --- | --- | --- | --- |
| 1 | Focus a supported harness | Overlay shows the assigned character | Character sits on the cursor |
| 2 | Click the character | Hit animation, hit sound, inject phrase | Agent receives the phrase (default `next`) |
| 3 | Assemble or replace a character in settings | Files copy into `~/.whipnext/catalog/` | New still/idle/action (images or video) and optional hit WAV |

## Error paths

| Situation | What the user sees | Can they fix it |
| --- | --- | --- |
| Unsupported picker type (`fbx`, `obj`, broken video) | Import error, not a fake 3D/video tile | Yes: pick PNG/GIF/SVG/glTF/playable video/WAV |
| No focused harness | Overlay hidden when "only matched" is on | Yes: focus an enabled harness or turn the toggle off |
| Slash phrase | Injected text becomes `next` | Yes: use a non-empty line that does not start with `/` |

## Invariants

- Default payload is `next`. A character phrase may be any non-empty text except a slash command.
- Toggles apply immediately. Closing settings exits the overlay.
- A pack character is a folder with `manifest.json` plus media. Adding one does not require a new Rust id.
- User settings live in `~/.whipnext/`, not in the repo.

## Success

- A click on the overlay continues the focused session without opening a command palette.
- A new character is assembled from images or video plus a hit sound, then used on a harness.
