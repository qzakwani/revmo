# Handoff: revmo launcher UI (terminal direction)

## Overview
revmo is a cross-platform desktop app launcher summoned by a global hotkey (default Alt+Space). One text input handles apps, files, calculator, shell commands, clipboard history, window switching and system actions. This handoff covers the selected "terminal" visual direction: dense monospace, flat surfaces, mode prefixes, inverted amber selection. It also includes five candidate logo/app-icon marks (none selected yet).

## About the Design Files
`revmo.dc.html` is a **design reference built in HTML** (a prototype of look and layout, not production code). Recreate it in the launcher's real stack (e.g. Tauri/Electron + web UI, GTK, Qt, egui, iced) using its idioms. If no stack exists yet, pick one that supports a frameless always-on-top window with global hotkey registration on Linux, macOS and Windows. Open the HTML in a browser to see the screens side by side; `extra.dc.html` holds rejected directions and can be ignored.

## Fidelity
**High-fidelity.** Colors, type, spacing and copy are final unless noted. Match them closely.

## Window
- Frameless, always-on-top, centered horizontally, top edge ~15% from screen top. Hidden on Esc or focus loss.
- Width 560px. Height fits content (min 44px input + footer, grows with results, cap ~8 rows + footer).
- Background `#0f1012`, border 1px `#2c2e33`, radius 4px, shadow `0 20px 50px rgba(0,0,0,.5)`.
- Light theme: background `#f7f6f2`, border `#c9c6bd`, divider `#e2e0d8`, footer bg `#edebe5`, shadow `0 20px 50px rgba(40,35,25,.25)`.
- Follows system theme by default; user override in settings.

## Screens

### 1. Empty state (just opened)
Purpose: tell the user what to type.
- **Input row** (height 44, padding 0 14, bottom border 1px `#2c2e33`): amber `›` prompt glyph (15px, weight 500), placeholder "type to search, or a prefix for a mode" in `#6a6b70`, block cursor 8×16 amber immediately after.
- **Mode legend** (padding 12 14, 2-col grid `auto 1fr`, column-gap 16, row-gap 5, 12.5px): prefix in amber, description in `#d9d9dc`:
  - `>` run shell command
  - `=` calculate
  - `~` find files
  - `:` switch window
  - `;` clipboard history
  - `!` system (lock, sleep, restart, logout) — parenthetical in `#6a6b70`
- **Recent line** (top border, padding 10 14, 12px `#6a6b70`): `recent ── firefox · code · obsidian · > git status · = 44*12`. Previous prefixed queries render in `#9a9a9e`. Each item clickable: re-runs it.
- **Footer** (bg `#141518`, top border, padding 7 14, 11px `#6a6b70`, gap 14): `↑↓ move`, `↵ run`, `tab actions`, `esc close`; right-aligned `revmo 0.1` in `#4a4c52`.

### 2. Search results
Purpose: mixed-source results as the user types; no prefix = search everything.
- Input row shows typed text in `#d9d9dc`, cursor after it. Right side: `7 / 1` (total / selected index) in 11px `#6a6b70`.
- **Result rows**: list padding 6 0; each row is a grid `40px 1fr 190px 44px`, padding 5 14, 12.5px mono, min-height 26.
  - col 1: source tag (`app`, `win`, `file`, `clip`, `sys`) in `#6a6b70`
  - col 2: title; matched substring bold (600) and amber
  - col 3: secondary (path, workspace, relative time, "needs sudo"), `#6a6b70`, ellipsis
  - col 4: right-aligned shortcut `↵` for selected, `⌘2`…`⌘9` for the rest (Alt on Linux/Windows). `⌘N` triggers row N directly.
- **Selected row**: background amber `oklch(0.78 0.14 60)` (≈ `#f0a940`), all text `#1b1206`, secondary at 75% opacity, match bold stays 600 without a color change. Full-width inverted bar, no radius.
- Footer becomes per-source counts: `apps 2 · win 1 · files 2 · clip 1 · sys 1`, with `tab actions` right-aligned.
- Ranking: exact prefix > frecency > fuzzy. Apps first unless another source has a stronger match.

### 3. Mode view (calculator shown, light theme)
Purpose: a prefix locks the input into a single source.
- When a prefix is typed, the `›` glyph is replaced by a **mode badge**: 11px, uppercase, bg amber-dark `oklch(0.55 0.14 60)` (≈ `#a86a1e`), text `#f7f6f2`, padding 2 6, radius 2. Label: CALC, SHELL, FILES, WIN, CLIP, SYS. Backspace on empty input removes the prefix.
- **Calc answer block** (padding 16 14 14, bottom border): `= 1280` at 36px/1 weight 500; hint `↵ copy · ⌘↵ paste to active window` 11.5px `#6e6e72`.
- "also matched" label (11px uppercase, letter-spacing .06em, `#6e6e72`) then secondary rows using the same grid minus the 190px column: `>` run in shell, `hex` 0x500, `%` percentage.
- Footer: `mode: calc · esc clear` (first Esc clears, second closes).
- Shell mode (`>`): Enter runs and closes; `⌘↵` runs and keeps the window open showing stdout in a mono block below the input (max 8 lines, scrollable).

### 4. Actions pane (Tab on a result)
Purpose: secondary actions without leaving the keyboard.
- Layout becomes 2 columns: `1fr 210px`, divider 1px `#2c2e33`.
- Left: result list, selected row loses the amber fill and gets bg `#1e2024` + 2px amber left border; other rows dim to `#9a9a9e`. Below the list, a detail block (11px/1.6 `#6a6b70`): path, size, modified.
- Right: action list, padding 6 0; rows padding 5 14, 12.5px. Selected action = full amber inverted row. Shortcuts right-aligned `#6a6b70`. Destructive actions (trash, kill, force quit) in `#e07b6a`.
- File actions: open ↵, reveal in folder ⌘↵, open with… ⌘o, copy path ⌘c, open terminal here ⌘t, trash ⌘⌫.
- App actions: open, new window, focus existing, quit. Window actions: focus, move to current workspace, close. Clip actions: paste, copy, pin, delete.
- Footer: `actions: <title>` left, `tab back` right. ↑↓ move within the action list; Tab/Esc return.

### 5. Onboarding (first launch, 3 steps)
Purpose: set hotkey, pick sources, done.
- Card 500px wide, centered on screen, same surface styles, padding 22 24 18, gap 16.
- Step strip (11px, `#6a6b70`, gap 10): `revmo setup` in amber, then `[1/3] hotkey  [2/3] sources  [3/3] done`; current step `#d9d9dc`.
- Heading 18px/1.3 weight 500: "Press the keys you want to summon revmo with."
- **Key capture box** (padding 18, border 1px `#2c2e33`, bg `#141518`): `›` glyph, keycaps `alt` `+` `space` (16px weight 500, padding 4 10, border `#3a3c42`, bg `#1e2024`), amber cursor. Live-updates as the user presses a chord.
- Status lines (12px/1.6 `#6a6b70`): `conflicts: none detected` (turn `#e07b6a` with the conflicting app name if the chord is taken), `default: alt+space · change later with ! settings`.
- Buttons: primary `continue ↵` amber bg, `#1b1206` text, 12.5px weight 500, padding 7 14, no radius; `skip` text-only `#9a9a9e`.
- Step 2 (sources): checklist of the six sources with the same row grid, Space toggles, all on by default; file-search shows index paths. Step 3: shows the hotkey in large keycaps and `press it now to finish`.

## Interactions
- Typing filters results on every keystroke; debounce ≤ 30 ms, results must feel instant.
- ↑↓ wrap; Enter runs the selected row's default action and closes the window.
- Tab opens/closes the actions pane; Esc clears input, then closes.
- No animations on open/close beyond an 80 ms opacity fade; selection moves instantly (no easing).
- Cursor blinks 530 ms step (not eased) and stops while typing.
- Window hides on focus loss. Re-summon restores empty state (not previous query).

## State
- `query: string`, `mode: null | 'shell' | 'calc' | 'files' | 'win' | 'clip' | 'sys'`
- `results: Result[]` where `Result = { id, source, title, matchRanges, secondary, actions[], run(action) }`
- `selectedIndex`, `actionsOpen: boolean`, `selectedActionIndex`
- `theme: 'system' | 'dark' | 'light'`, `hotkey: string`
- persisted: frecency table, recent queries (last 5), clipboard history, onboarding complete

## Design tokens
Colors (dark): bg `#0f1012`, bg-raised `#141518`, bg-selected-soft `#1e2024`, border `#2c2e33`, border-strong `#3a3c42`, text `#d9d9dc`, text-dim `#9a9a9e`, text-muted `#6a6b70`, text-faint `#4a4c52`, accent `oklch(0.78 0.14 60)` ≈ `#f0a940`, on-accent `#1b1206`, danger `#e07b6a`.
Colors (light): bg `#f7f6f2`, bg-raised `#edebe5`, border `#c9c6bd`, divider `#e2e0d8`, text `#1b1b1d`, text-muted `#6e6e72`, text-faint `#9a9a9e`, accent `oklch(0.55 0.14 60)` ≈ `#a86a1e`, on-accent `#f7f6f2`, danger `#b8463a`.
Type: IBM Plex Mono everywhere (fallback: system mono). Sizes 11 / 12 / 12.5 / 15 (input) / 16 (keycaps) / 18 (onboarding heading) / 36 (calc answer). Weights 400 body, 500 emphasis, 600 match highlight. Line-height 1 for single-line rows, 1.6 for status text.
Spacing: 4-based; row padding 5×14, panel padding 12–18, gap 10/14/16.
Radius: window 4, badge 2, everything else 0.
Shadow: `0 20px 50px rgba(0,0,0,.5)` dark / `rgba(40,35,25,.25)` light.

## Logo / icon candidates (unselected)
Section 2 of `revmo.dc.html` shows five marks; implement whichever is chosen as SVG at 16/32/64/128/256/512 plus a monochrome 16px tray glyph:
- 2a prompt: amber `›` + light block cursor on `#0f1012` tile (radius 10 at 72px ≈ 14%)
- 2b monogram: mono `r` with a 10px amber square at the baseline
- 2c rev: 270° arc stroke 5/40 with an amber arrowhead
- 2d double chevron: `»` on a full-amber tile
- 2e keycap: `rv` inside an outlined keycap (thicker bottom edge)

## Assets
No raster assets. Font: IBM Plex Mono (OFL) — bundle it, do not rely on Google Fonts at runtime. App icons in the mock are placeholders; use the OS's icon theme / .desktop / .app bundle icons.

## Files
- `revmo.dc.html` — selected direction (screens + logo candidates)
- `extra.dc.html` — rejected directions, for reference only
