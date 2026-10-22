#!/usr/bin/env python3
"""Verify Kitty logo output and navigation, including queued terminal input."""
import fcntl
import os
from pathlib import Path
import pty
import re
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time
from smoke_support import ESCAPE


def exercise(binary, queued_input):
    master, slave = pty.openpty()
    original = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 960, 640))
    env = {key: value for key, value in os.environ.items() if not key.startswith("KAISER_")}
    env.update(TERM="xterm-kitty", TERM_PROGRAM="kitty")
    for key in ("TMUX", "WEZTERM_EXECUTABLE", "KONSOLE_VERSION"):
        env.pop(key, None)
    with tempfile.TemporaryDirectory(prefix="kaiser-logo-smoke-") as directory:
        root = Path(directory)
        config = root / "config.toml"
        config.write_text('theme = "kaiser-main"\n')
        process = subprocess.Popen(
            [str(binary), "--demo", "--config", str(config), "--data-dir", str(root / "data")],
            stdin=slave, stdout=slave, stderr=slave, env=env,
        )
        captured = bytearray()

        def wait_for(predicate, stage):
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                while select.select([master], [], [], 0)[0]:
                    captured.extend(os.read(master, 65536))
                if predicate():
                    return
                assert process.poll() is None, f"Kaiser exited during {stage}: {process.returncode}"
                time.sleep(0.02)
            visible = re.sub(rb"\x1b\[[0-?]*[ -/]*[@-~]", b"", captured[-1500:])
            raise AssertionError(f"Timed out during {stage} (queued={queued_input}): {bytes(visible)!r}")

        try:
            wait_for(lambda: b"a=T" in captured and b"LOCAL" in captured, "logo rendering")
            assert b"f=32" in captured, "Logo must be actual RGBA image data"
            captured.clear()
            os.write(master, b":notebook\r")
            wait_for(lambda: b"Select notebook" in captured, "notebook picker")
            if queued_input:
                # Reproduce a busy runner: both keys arrive before Kaiser can read either.
                os.kill(process.pid, signal.SIGSTOP)
                os.waitpid(process.pid, os.WUNTRACED)
            # Kaiser enables Kitty's disambiguated keyboard protocol. A bare ESC
            # followed by ':' can be decoded as Alt+: if the input gets queued.
            os.write(master, ESCAPE + b":q\r")
            if queued_input:
                os.kill(process.pid, signal.SIGCONT)
            wait_for(lambda: process.poll() is not None, "quit and terminal restoration")
            assert process.returncode == 0
            assert termios.tcgetattr(slave) == original, "Terminal settings were not restored"
            assert not (root / "data").exists(), "Demo must not create study data"
            assert config.read_text() == 'theme = "kaiser-main"\n'
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


def main():
    binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/kaiser").resolve()
    exercise(binary, queued_input=False)
    exercise(binary, queued_input=True)
    print("PASS: original PNG → notebook picker → queued Escape/quit → terminal restore; isolated config/data")


if __name__ == "__main__":
    main()
