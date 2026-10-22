#!/usr/bin/env python3
"""Simulate a Kitty graphics terminal and verify the embedded logo is sent."""
import fcntl
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import sys
import termios
import time


def main():
    binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/kaiser").resolve()
    master, slave = pty.openpty()
    original = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 960, 640))
    env = dict(os.environ, TERM="xterm-kitty")
    for key in ("TMUX", "WEZTERM_EXECUTABLE", "KONSOLE_VERSION"):
        env.pop(key, None)
    process = subprocess.Popen([str(binary), "--demo"], stdin=slave, stdout=slave, stderr=slave, env=env)
    captured = bytearray()

    def wait_for(predicate):
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.05)[0]:
                captured.extend(os.read(master, 65536))
            if predicate():
                return
            assert process.poll() is None, "Kaiser exited unexpectedly"
        raise AssertionError("Timed out waiting for terminal image output")

    try:
        wait_for(lambda: b"a=T" in captured and b"LOCAL" in captured)
        assert b"f=32" in captured, "Logo must be actual RGBA image data"
        captured.clear()
        os.write(master, b":notebook\r")
        wait_for(lambda: b"Select notebook" in captured)
        os.write(master, b"\x1b")
        time.sleep(0.15)
        os.write(master, b":q\r")
        wait_for(lambda: process.poll() is not None)
        assert process.returncode == 0
        assert termios.tcgetattr(slave) == original, "Terminal settings were not restored"
        print("PASS: image protocol detection → original PNG → notebook picker → quit → terminal restore")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)


if __name__ == "__main__":
    main()
