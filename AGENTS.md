# Working on Logia

Logia has a macOS recording/captions/copy preview. Prioritize usable product increments over extending experiments. Execute agreed work continuously: build, test, review, fix, and document outcomes. Pause for material direction changes, necessary user input/access, or destructive actions. Do not reopen settled decisions at routine task boundaries.

## Product invariants

- Local dictation is the core workflow. macOS, Windows, and Linux are intended targets; report actual build and desktop-test coverage separately.
- Real live captions are required in the first personal preview.
- The recording surface is a compact non-activating floating card with no window chrome, not the application window resized. Keep the default transcript viewport shallow even during long speech; the full transcript remains retained and reachable by scrolling or deliberate expansion. Never automatically shrink during recording or pauses; an explicit compact action may reverse deliberate expansion. The latest review proposes two visible lines, with five lines only on expansion. The dot is an idle indicator only. D07's collapse during recording and twelve-word rolling caption are withdrawn; its overlay, palette, typography, and separation from the deliberate editing/recovery window remain required. Presentation must never invent text or delay authoritative finalization/cancellation.
- Corrections to one behavior do not revoke the rest of an accepted design. Preserve handoff pointers and reconcile conflicting specifications before changing the product surface. Show the corrected overlay to the user before moving on to other features.
- Preserve the complete D07 lifecycle, not just its recording card: yellow idle dot, Preparing/Armed capsule, growing transcript, unchanged card through pauses, Finishing, green success confirmation and return to idle, or persistent recovery. Use one persistent pip. Approval of one state's layout does not authorize omission of other states or transitions.
- Global dictation pastes at the current text cursor when the finished transcript is delivered, not at a destination locked at recording start (user clarification, 2026-09-22). Use ordinary paste across apps; missing Accessibility field/write support alone must not force copy-only. Capture the current process/window and available field/selection evidence immediately before paste, and refuse changes during that dispatch check. If no eligible destination exists, copy the transcript. Never send Enter/control payloads or blindly retry an uncertain insertion. Missing field metadata cannot prove the exact logical field or password status; dispatch is not a receipt.
- One resident recognizer and one active inference worker. Bound audio storage, inference queues, and preview work.
- Cancel must stop owned inference. Reject stale results and reap a worker before starting its replacement.
- History is off until explicit opt-in; when enabled, expire at seven days and keep at most 500 entries. Private/canceled sessions create no new content history.
- Diagnostics must not contain recordings, transcripts, field contents, window titles, or credentials.

## Development

- Make focused changes with explicit behavior and evidence. Distinguish plans, simulations, unit tests, and native observations.
- Test consequential behavior first: wrong-target classification, malformed input, queue bounds, stale output, cancellation, and persistence boundaries.
- Prefer modules under 200 handwritten production lines; split by responsibility and justify exceptions.
- Run relevant checks before reporting success. The target probe uses `bash scripts/test-target-probe.sh`.
- Install with `pnpm app:install` from `apps/desktop`; never merge-copy into an existing `.app`. Verify the installed signature, executable hash, and normal LaunchServices launch before reporting an installed fix. Keep one runnable installed bundle; archive development copies. Ad-hoc signing does not preserve permission identity across changed binaries, even with a fixed identifier.
- Use independent review for substantive safety and architecture work, then verify findings before applying them.
- Preserve attribution, pin dependencies, and verify artifact checksums. No upstream app credentials, updater identity, or personal data may be imported.
- Keep research notes, handovers, local evidence, model binaries, audio, transcripts, and build files out of public commits. Stage explicit reviewed paths rather than the whole workspace.
- Publication is authorized for tested milestones in this repository. Do not force-push, erase remote work, or publish unrelated local material.

## Local context

**Current-cursor correction, 2026-09-22 — `review/global-paste-implementation-2026-09-22.md`.** The user explicitly clarified that global paste means wherever the text cursor is at delivery, or copied if nowhere. This supersedes recording-start target locking and the original-window proposal in the earlier design note. Keep the strict identity probe separate from production. Do not reintroduce AX writability or per-app transport gates.

**Latest delivery recovery, 2026-09-21 — `review/delivery-recovery-2026-09-21.md`.** The user confirms Chrome insertion works; do not repeat Chrome microphone tests to resolve flaws in the local test driver. Apple Terminal's readonly AXTextArea was then identified as a separate unsupported-target bug. A narrow Terminal.app paste path now passes native exact-text/no-Enter, changed-tab, cancel, and multiline controls. Clipboard ownership and cancel outcomes remain protected; hidden webviews have explicit background scheduling. Preserve the earlier handoffs below.

**Latest recovery, 2026-09-19 — `review/astra-recovery-2026-09-18.md`.** Nested installation, invalid outer signature, duplicate bundles, and permission-controller races were repaired. One verified installed app now uses a persistent local signing identity and reports both permissions allowed. Native speech-to-field acceptance remains; browser tests do not establish it. Preserve the older Claude handoffs below.

Ignored local planning files may be present under `docs/`, `research/`, and `review/`. They supplement contributor docs but are not required to build or test. When present, `review/local-working-context.md` and `review/astra-resume.md` carry session continuity; earlier planning-only restrictions are superseded by continuous implementation authority.

<!-- claude:begin -->
**START HERE — `review/claude-handoff-to-astra-2026-09-18.md`.** Astra is taking back over. That file states the user's three live complaints with diagnoses (microphone permission invalidated by ad-hoc signing identity churn; no permission wizard; overlay status row too dense), everything shipped this session, and the two things blocking the model catalogue from delivering an actual model choice. Read it before the older Claude notes below.

**Claude driver session, 2026-09-11 (later) — `review/claude-vocabulary-2026-09-11.md`.** The idle dot now defaults to off at the user's request. R07 vocabulary rules are built: exact spoken-phrase → replacement pairs, whole-phrase matching, final text only, nothing invented. Matching lives in `src-tauri/src/dictionary.rs` because delivery happens in the parent process — rules applied only in the webview would fix what the user reads and not what gets typed; the settings preview calls that same implementation. Two bugs fixed on the way: a stale acknowledgment could abandon a session, and the microphone waited on a React commit (2170 ms → 13 ms from shortcut to `start_recording`).

**Claude driver session, 2026-09-11 — `review/claude-overlay-safety-2026-09-11.md`.** Implemented the five contracts in `tests/overlay-safety.cjs`, which were written and left failing. The microphone now opens only after the overlay acknowledges, from an effect after commit, that it is rendering the current session; a bare event emit is not evidence. Found two real bugs on the way: the lost-bridge timer could never fire (restarted by the once-a-second elapsed tick), and an assertion in `overlay.cjs` could never fail. Also fixed a preference-clobbering race and wired all three browser suites into `test:ui`. No product surface changed; all suites green.

**Claude driver session, 2026-09-10 — read `review/claude-driver-log-2026-09-10.md` before building.** It unblocked the build (`tauri.conf.json` lacked `macOSPrivateApi` while `Cargo.toml` enabled `macos-private-api`, so every cargo command failed), repaired the `window_smoke` example and `tests/ui.cjs`, added a `test:unit` script for the orphaned preferences tests, corrected the stale README, and removed the twelve-word truncation from `docs/mockups/logia-ui-breathing-claude.html` that caused the overlay detour. No product surface was changed. All suites verified green in that log; `grep -rn "claude 2026-09-10" apps docs` lists every edit site.

Earlier context: `review/claude-handoff-2026-09-10.md` (how the D07 overlay was dropped and what both agents got wrong) and `review/claude-d07-correction.md` (the binding spec).
<!-- claude:end -->

**Correction status, 2026-09-10:** the invariant above is now amended. The native recording surface still needs replacement; the explicitly simulated visual checkpoint and implementation sequence are in `review/overlay-correction-implementation.md`. Preserve this handoff trail.
