# Working on Logia

Logia has a macOS recording/captions/copy preview. Prioritize usable product increments over extending experiments. Execute agreed work continuously: build, test, review, fix, and document outcomes. Pause for material direction changes, necessary user input/access, or destructive actions. Do not reopen settled decisions at routine task boundaries.

## Product invariants

- Local dictation is the core workflow. macOS, Windows, and Linux are intended targets; report actual build and desktop-test coverage separately.
- Real live captions are required in the first personal preview.
- Keep the full paragraph visible during pauses and after Stop. The user's microphone feedback supersedes the earlier collapsing in-window bubble. Presentation may pace received words briefly, but must never invent text or delay authoritative finalization/cancellation.
- Verify the intended application, window, and field before insertion. Unknown identity means copy-only. Never auto-submit or blindly retry uncertain insertion.
- One resident recognizer and one active inference worker. Bound audio storage, inference queues, and preview work.
- Cancel must stop owned inference. Reject stale results and reap a worker before starting its replacement.
- History is off until explicit opt-in; when enabled, expire at seven days and keep at most 500 entries. Private/canceled sessions create no new content history.
- Diagnostics must not contain recordings, transcripts, field contents, window titles, or credentials.

## Development

- Make focused changes with explicit behavior and evidence. Distinguish plans, simulations, unit tests, and native observations.
- Test consequential behavior first: wrong-target classification, malformed input, queue bounds, stale output, cancellation, and persistence boundaries.
- Prefer modules under 200 handwritten production lines; split by responsibility and justify exceptions.
- Run relevant checks before reporting success. The target probe uses `bash scripts/test-target-probe.sh`.
- Use independent review for substantive safety and architecture work, then verify findings before applying them.
- Preserve attribution, pin dependencies, and verify artifact checksums. No upstream app credentials, updater identity, or personal data may be imported.
- Keep research notes, handovers, local evidence, model binaries, audio, transcripts, and build files out of public commits. Stage explicit reviewed paths rather than the whole workspace.
- Publication is authorized for tested milestones in this repository. Do not force-push, erase remote work, or publish unrelated local material.

## Local context

Ignored local planning files may be present under `docs/`, `research/`, and `review/`. They supplement contributor docs but are not required to build or test. When present, `review/local-working-context.md` and `review/astra-resume.md` carry session continuity; earlier planning-only restrictions are superseded by continuous implementation authority.
