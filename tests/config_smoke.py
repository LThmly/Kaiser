#!/usr/bin/env python3
"""Exercise config editing, terminal suspension, reload, and theme commands."""
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import struct
import subprocess
import sys
import tempfile
import termios
import time
from smoke_support import seed_library


def main():
    binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/kaiser").resolve()
    master, slave = pty.openpty()
    original = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    env = {key: value for key, value in os.environ.items() if not key.startswith("KAISER_")}
    env.update(TERM="xterm-256color", COLORTERM="truecolor")
    env.pop("NO_COLOR", None)
    with tempfile.TemporaryDirectory(prefix="kaiser-config-smoke-") as directory:
        root = Path(directory)
        config = root / "config.toml"
        editor = root / "mock editor.py"
        log = root / "editor.jsonl"
        editor.write_text('''import json, os, sys, termios
from pathlib import Path
path = Path(sys.argv[-1])
assert os.isatty(0) and os.isatty(1)
assert termios.tcgetattr(0)[3] & termios.ICANON
text = path.read_text()
if path.name == "config.toml":
    text = text.replace('leader = "space"', 'leader = ","').replace('sidebar = true', 'sidebar = false')
else:
    text = text.replace("#123ABC", "#BADA55")
path.write_text(text)
with (path.parent.parent if path.parent.name == "themes" else path.parent).joinpath("editor.jsonl").open("a") as stream:
    stream.write(json.dumps(str(path)) + "\\n")
''')

        def settings(command):
            return ('theme = "smoke"\n[keys]\nleader = "space"\n[ui]\nsidebar = true\n'
                    '[editor]\ncommand = ' + json.dumps(command) + '\n')

        config.write_text(settings([sys.executable, str(editor)]))
        (root / "themes").mkdir()
        (root / "themes/smoke.toml").write_text('accent = "#123ABC"\n')
        checked = subprocess.run([str(binary), "--check-config", "--config", str(config), "--data-dir", str(root / "data")], env=env, capture_output=True, text=True)
        assert checked.returncode == 0, checked.stderr
        assert "Configuration valid" in checked.stdout
        assert not (root / "data").exists(), "Validation must not create study data"
        invalid = root / "invalid.toml"
        invalid.write_text('[model.usage]\nmax_summary_requests = 0\n')
        checked = subprocess.run([str(binary), "--check-config", "--config", str(invalid), "--data-dir", str(root / "data")], env=env, capture_output=True, text=True)
        assert checked.returncode != 0
        assert "model.usage.max_summary_requests" in checked.stderr
        seed_library(root / "data/state.json")
        process = subprocess.Popen([str(binary), "--data-dir", str(root / "data"), "--config", str(config)], stdin=slave, stdout=slave, stderr=slave, env=env)
        captured = bytearray()

        def drain():
            while select.select([master], [], [], 0)[0]:
                captured.extend(os.read(master, 65536))

        def wait_for(predicate, label):
            deadline = time.monotonic() + 5
            while time.monotonic() < deadline:
                drain()
                if predicate():
                    return
                assert process.poll() is None, f"Kaiser exited during {label}"
                time.sleep(0.02)
            raise AssertionError(f"Timed out during {label}: {bytes(captured[-1000:])!r}")

        def active():
            try:
                return json.loads((root / "data/state.json").read_text())["active"]
            except (FileNotFoundError, json.JSONDecodeError):
                return None

        def editing_done(count):
            return log.exists() and len(log.read_text().splitlines()) >= count and not termios.tcgetattr(slave)[3] & termios.ICANON

        def send(value):
            os.write(master, value)

        try:
            wait_for(lambda: b"Kaiser" in captured, "startup")
            send(b":config-open\r")
            wait_for(lambda: editing_done(1), "config editor and terminal resume")
            assert 'leader = ","' in config.read_text()
            send(b":config-reload\r,PSelect notebook\rBiology\r")
            wait_for(lambda: active() == 1, "live leader reload and notebook selection")
            send(b":theme-open\r")
            wait_for(lambda: editing_done(2), "theme editor and terminal resume")
            send(b":theme-reload\r")
            wait_for(lambda: b"38;2;186;218;85" in captured, "live theme color reload")
            drain()
            captured.clear()
            send(b":theme kaiser-l\t")
            wait_for(lambda: b"38;2;185;166;255" in captured, "live theme preview before Enter")
            drain()
            captured.clear()
            send(b"\x1b")
            wait_for(lambda: b"38;2;186;218;85" in captured, "restoring theme after cancelling preview")
            send(b":theme kaiser-l\t\t\x1b[Z\t\r")
            wait_for(lambda: b"38;2;185;166;255" in captured, "filtered forward/reverse theme completion with wraparound")
            assert 'theme = "smoke"' in config.read_text()
            config.write_text(config.read_text() + "\n[broken\n")
            send(b":config-reload\r,PSelect notebook\rDistributed\r")
            wait_for(lambda: active() == 0, "keeping valid settings after failed reload")
            config.write_text(settings([str(root / "missing-editor")]).replace('leader = "space"', 'leader = ","'))
            drain()
            captured.clear()
            send(b":config-reload\r:config-open\r")
            wait_for(lambda: b"No such file" in captured, "failed editor launch")
            assert not termios.tcgetattr(slave)[3] & termios.ICANON
            send(b"\x1b[102;9uReading\r")
            wait_for(lambda: active() == 2, "navigation after failed editor launch")
            send(b":q\r")
            wait_for(lambda: process.poll() is not None, "quit and terminal restoration")
            assert process.returncode == 0
            assert termios.tcgetattr(slave) == original
            print("PASS: config-open → reload → theme-open → theme-reload → theme completion → invalid config recovery → failed editor recovery → terminal restore")
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


if __name__ == "__main__":
    main()
