#!/usr/bin/env python3
"""Check showcase views and prove demo mode leaves stored data untouched."""
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time


def main():
    binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/kaiser").resolve()
    env = {key: value for key, value in os.environ.items() if not key.startswith("KAISER_")}
    env.update(TERM="xterm-256color", KAISER_API_URL="intentionally-not-a-url", KAISER_MODEL="never-connect")
    with tempfile.TemporaryDirectory(prefix="kaiser-demo-smoke-") as directory:
        root = Path(directory)
        config = root / "config.toml"
        config.write_text('theme = "kaiser-bagels"\n')
        data = root / "data"
        for name in ["state.json", "auth/accounts.json", "ai/usage.json"]:
            path = data / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("PRIVATE SENTINEL; intentionally invalid JSON")
        before = {path.relative_to(root): path.read_bytes() for path in root.rglob("*") if path.is_file()}
        base = [str(binary), "--config", str(config), "--data-dir", str(data)]
        for view, expected in [("home", "Distributed Systems"), ("chat", "overfitting"),
                               ("flashcards", "ANSWER / DEFINITION"), ("notes", "training loop")]:
            result = subprocess.run(base + ["--demo-view", view, "--snapshot", "--size", "140x42"], env=env, capture_output=True, text=True, timeout=10)
            assert result.returncode == 0, result.stderr
            assert expected in result.stdout, (view, result.stdout)
            assert "DEMO" in result.stdout
            assert "PRIVATE SENTINEL" not in result.stdout
        master, slave = pty.openpty()
        original = termios.tcgetattr(slave)
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 42, 140, 0, 0))
        process = subprocess.Popen(base + ["--demo-view", "flashcards"], env=env, stdin=slave, stdout=slave, stderr=slave)
        captured = bytearray()

        def visible_text():
            return re.sub(rb"\s+", b"", re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", captured))

        def wait_for(predicate):
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                while select.select([master], [], [], 0)[0]:
                    captured.extend(os.read(master, 65536))
                if predicate():
                    return
                assert process.poll() is None, "Demo exited unexpectedly"
                time.sleep(0.02)
            visible = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", captured)
            raise AssertionError(f"Demo timed out: {bytes(visible[-2000:])!r}")

        try:
            wait_for(lambda: b"ANSWER / DEFINITION" in captured and b"24 flashcards" in captured)
            os.write(master, b":config-reload\r")
            wait_for(lambda: b"Configreloade" in visible_text())
            os.write(master, b":chat\riExplain hidden layers\r")
            # Terminal diffs can put cursor/style sequences between displayed words.
            wait_for(lambda: b"Offlinesourcelookup" in visible_text())
            os.write(master, b":q\r")
            wait_for(lambda: process.poll() is not None)
            assert process.returncode == 0
            assert termios.tcgetattr(slave) == original
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)
        after = {path.relative_to(root): path.read_bytes() for path in root.rglob("*") if path.is_file()}
        assert after == before, "Demo changed saved data or configuration"
        print("PASS: showcase views → offline chat → no private data/config/usage changes → terminal restore")


if __name__ == "__main__":
    main()
