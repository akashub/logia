#!/usr/bin/env python3
"""Native positive/negative controls using an owned synthetic AppKit fixture."""
import argparse
import json
import pathlib
import selectors
import subprocess
import tempfile


def line_with_timeout(stream, seconds=10):
    with selectors.DefaultSelector() as selector:
        selector.register(stream, selectors.EVENT_READ)
        if not selector.select(seconds):
            raise RuntimeError("Native helper did not respond within the deadline")
        line = stream.readline()
        if not line:
            raise RuntimeError("Native helper exited before acknowledging the test")
        return line.strip()


def stop(process):
    if process.poll() is None:
        process.terminate()
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=3)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", type=pathlib.Path, required=True)
    parser.add_argument("--only", help="Run a single named control")
    args = parser.parse_args()
    binary = args.bin_dir.resolve()
    cases = [
        ("unchanged-text-area", "first", "first", {("same", "identity-matched")}),
        ("same-window-field-switch", "first", "second", {("changed", "identity-changed")}),
        ("recreated-field", "first", "recreate", {("changed", "identity-changed"), ("unknown", "retained-identity-unavailable")}),
        ("secure-field", "secure", "secure", {("unknown", "initial:secure-field")}),
        ("readonly-field", "readonly", "readonly", {("unknown", "initial:value-not-settable")}),
    ]
    if args.only:
        cases = [case for case in cases if case[0] == args.only]
        if not cases:
            parser.error("Unknown control name")
    print("Opening synthetic test window; checks run automatically.", flush=True)
    # Unbuffered pipes keep selector readiness aligned with actual unread lines.
    fixture = subprocess.Popen([str(binary / "target-fixture")], stdin=subprocess.PIPE,
                               stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, bufsize=0)
    failures = 0
    try:
        with tempfile.TemporaryDirectory(prefix="logia-native-check-") as directory:
            for name, before, after, expected in cases:
                fixture.stdin.write((before + "\n").encode())
                line_with_timeout(fixture.stdout)
                output = pathlib.Path(directory) / (name + ".json")
                probe = subprocess.Popen([
                    str(binary / "target-probe"), "--case", name,
                    "--capture-delay-ms", "0", "--recheck-delay-ms", "1500",
                    "--output", str(output),
                ], stdout=subprocess.PIPE, stderr=subprocess.PIPE, bufsize=0)
                try:
                    while True:
                        message = line_with_timeout(probe.stderr).decode()
                        if "Accessibility access is unavailable" in message:
                            print(message)
                            return 2
                        if "Initial observation taken" in message:
                            break
                    fixture.stdin.write((after + "\n").encode())
                    line_with_timeout(fixture.stdout)
                    probe.communicate(timeout=10)
                    if probe.returncode != 0:
                        raise RuntimeError(f"Probe failed with exit {probe.returncode}")
                    record = json.loads(output.read_text())
                    passed = (record["verdict"], record["reason"]) in expected
                    failures += not passed
                    print(f"{'PASS' if passed else 'FAIL'} {name}: {record['verdict']} ({record['reason']})", flush=True)
                finally:
                    stop(probe)
    finally:
        stop(fixture)
    print(f"{len(cases) - failures}/{len(cases)} native controls passed. "
          "This checks the synthetic AppKit fixture, not other applications.")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
