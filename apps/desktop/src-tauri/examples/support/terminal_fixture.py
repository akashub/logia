"""Owned raw-input receiver. Never invokes a shell with received text."""
import json
import os
from pathlib import Path
import select
import sys
import termios
import time
import tty

folder = Path(sys.argv[1])
expected = "Logia terminal café.".encode()
saved = termios.tcgetattr(sys.stdin)
received = bytearray()
try:
    print(f"\033]0;Logia target {folder.name}\007", end="", flush=True)
    print("Logia terminal delivery check — synthetic input only", flush=True)
    tty.setraw(sys.stdin.fileno())
    # This receiver does not enable bracketed paste. It measures exact bytes.
    sys.stdout.write("\033[?2004l")
    sys.stdout.flush()
    (folder / "ready").touch()
    deadline = time.monotonic() + 45
    while time.monotonic() < deadline and not (folder / "stop").exists():
        if select.select([sys.stdin], [], [], 0.1)[0]:
            received.extend(os.read(sys.stdin.fileno(), 4096))
            (folder / "result.json").write_text(json.dumps({
                "exact": bytes(received) == expected,
                "submitted": b"\n" in received or b"\r" in received,
                "bytes": len(received),
            }))
finally:
    termios.tcsetattr(sys.stdin, termios.TCSADRAIN, saved)
    print("\nLogia check finished.", flush=True)
    (folder / "exited").touch()
