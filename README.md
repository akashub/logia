# Logia

Local voice dictation for the desktop, with live captions and careful delivery to the intended text field.

**Early development:** implemented experiments include a read-only macOS target-identity probe and bounded Rust recognition transport. There is no dictation application or running recognizer yet. Windows and Linux desktop support is planned; it has not been validated.

## Target-identity probe

The probe compares the focused application, window, and accessible text-field identity at two points in time. Changed or unavailable identity cannot qualify for automatic insertion. It does not record audio, read field contents, write the clipboard, or insert text.

Requirements for this experiment: macOS 14 or newer, Swift 6 or newer, and Apple developer tools.

```sh
swift build --package-path spikes/target-identity
bash scripts/test-target-probe.sh
swift run --package-path spikes/target-identity target-probe --help
```

Start with the automatic native check:

```sh
bash scripts/check-target-identity.sh
```

It briefly opens a synthetic window, checks unchanged/switched/recreated fields plus secure/read-only rejection, and closes it. Leave that window focused while it runs. A generic `unknown` cannot pass the positive controls. These checks have passed on macOS 26.4; they establish behavior in the AppKit fixture, not compatibility with Chrome, Electron, or other applications.

For an observation in another application, launch the probe, focus a sample field during the first delay, then perform a focus change during the second delay:

```sh
swift run --package-path spikes/target-identity target-probe \
  --case same-window-switch \
  --capture-delay-ms 3000 \
  --recheck-delay-ms 3000 \
  --output /tmp/logia-target-case.json
```

The output file must not already exist. If Accessibility access is unavailable, the probe exits with instructions; it does not request permission automatically. Unit tests need no Accessibility access. The synthetic fixture at `spikes/target-identity/fixtures/fields.html` provides two fields and an element-recreation case.

Identity comparison is a feasibility experiment, not proof of safe insertion. Native results must be checked against observed behavior; matching references cannot establish atomic verification and delivery.

Capture failures include their stage and Accessibility error code. Missing subroles are allowed for ordinary text areas; secure fields and unreadable metadata remain excluded. Process identity uses kernel start time so standalone helpers do not require Launch Services registration.

## Recognition transport

`spikes/recognition` contains versioned bounded messages, PCM framing, a bounded audio queue, stale-result rejection, and preview coalescing. These are transport primitives; process supervision and engine integration are still being built.

```sh
cargo test --manifest-path spikes/recognition/Cargo.toml --locked
cargo clippy --manifest-path spikes/recognition/Cargo.toml --locked --all-targets -- -D warnings
```

## Direction

1. Establish field-identity limits and a bounded, cancelable recognition worker.
2. Build a macOS personal preview with genuine live captions and explicit recovery.
3. Add local beta features, then validate Windows and Linux desktop integrations.

See [CONTRIBUTING.md](CONTRIBUTING.md) for development and data-handling rules.
