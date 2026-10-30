# Android UI architecture (0.1.3+)

## File layout

```text
apps/wireglass-android/ui/
  components.slint   StageCard, StatusPill, PrimaryButton, BottomNav, ExitDialog, StageRow
  app.slint          MainWindow shell: FocusScope back, header, body, bottom nav, overlays
src/lib.rs           Validation, navigation handlers, wireglass::run, exit
```

## Screen map

| Screen | Role |
| --- | --- |
| `home` | One CTA + tips |
| `run` | Sticky target + Run; **only results Flickable** (no overlap) |
| `help` | How-to + valid/invalid examples |
| `about` | Versions + GitHub link + permissions/honesty |

## v0.2.0 additions
- Verdict strip derived from stage statuses.
- Live stage progress (pending → running → final).
- Cancel during loading.
- Share (ACTION_SEND) + copy helpers.
- Recent targets (local, max 5).
- Home last-result teaser + recent targets.
- Help FAQ.

Bottom nav: Home · Check · Help · About (always visible).

## Android Back

`FocusScope` accepts `Key.Back`:

1. If exit dialog open → dismiss  
2. Else if not home → go home  
3. Else → show **Leave Wireglass?** (Stay / Exit)  
4. Exit → `slint::quit_event_loop()`

## Validation (`normalize_target`)

- Trim; reject whitespace  
- `http://` / `https://` only if scheme present  
- Bare host → `https://…`  
- Domain labels / IPv4 / rough IPv6 brackets  
- Inline `field-error` + banner on failure  

## Competitor patterns (adapted)

- 1.1.1.1 → one primary action  
- SSL analyzers → target panel + empty/loading/done  
- Diagnostics → stage cards + status pills  

## Build

```bash
./scripts/linux/build-android.sh          # release arm64-v8a only
```
