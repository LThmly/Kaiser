#!/usr/bin/env python3
"""Exercise the real terminal event loop and persisted study flow (Unix only)."""
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
import zipfile
from smoke_support import seed_library


def main():
    binary = Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/kaiser").resolve()
    master, slave = pty.openpty()
    original = termios.tcgetattr(slave)
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    env = {key: value for key, value in os.environ.items() if not key.startswith("KAISER_")}
    env["TERM"] = "xterm-256color"
    env["COLORTERM"] = "truecolor"
    env.pop("NO_COLOR", None)
    with tempfile.TemporaryDirectory(prefix="kaiser-smoke-") as directory:
        config = Path(directory) / "config.toml"
        config.write_text('theme = "smoke"\n')
        themes = Path(directory) / "themes"
        themes.mkdir()
        (themes / "smoke.toml").write_text('base = "tallgeese"\naccent = "#123ABC"\n')
        seed_library(Path(directory) / "state.json")
        process = subprocess.Popen([str(binary), "--data-dir", directory, "--config", str(config)], stdin=slave, stdout=slave, stderr=slave, env=env)
        path = Path(directory) / "state.json"
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
            raise AssertionError(f"Timed out during {label}: {bytes(captured[-1400:])!r}")

        def state():
            try:
                return json.loads(path.read_text())["notebooks"][0]
            except (FileNotFoundError, json.JSONDecodeError):
                return {}

        def send(value):
            os.write(master, value)

        try:
            wait_for(lambda: b"Kaiser" in captured, "startup")
            assert b"replication.md" not in captured, "A notebook opened before selection"
            assert b"Study queue" not in captured, "Removed queue is still visible"
            send(b":ai-usage\r")
            wait_for(lambda: b"USAGE " in captured, "local AI usage dashboard")
            send(b"\x1b[6~\x1b[H\x1b")
            time.sleep(0.05)
            send(b":")
            wait_for(lambda: b"Save and quit" in captured, "automatic colon command suggestions")
            send(b"\x1b")
            time.sleep(0.05)
            send(b" n")  # Leader + lowercase n must create, not select.
            wait_for(lambda: b"New notebook" in captured, "new notebook shortcut")
            send(b"\x1b")
            time.sleep(0.05)
            send(b"\x1b[102;9u\t\t\t\r")
            wait_for(lambda: b"replication.md" in captured, "selecting a notebook")
            drain()
            captured.clear()
            send(b"i\x1b[110;9u")  # Open new-notebook menu while typing in a notebook.
            wait_for(lambda: b"New notebook" in captured, "new notebook shortcut inside notebook")
            send(b"\x1b")
            time.sleep(0.05)
            send(b" e\x1b[B\re")
            wait_for(lambda: b"Shape a study card" in captured, "opening a card from the Flashcards panel")
            send(b"\x1b")
            time.sleep(0.05)
            send(b" c")
            send(b"iWhat is replication lag?\r")
            wait_for(lambda: len(state().get("messages", [])) == 4, "source lookup")
            send(b" m\x13")  # Draft, then Ctrl+S.
            wait_for(lambda: len(state().get("cards", [])) == 4, "saving a draft")
            assert state()["cards"][-1]["draft"]
            send(b" d\x1b[Be\x01")  # Draft list, down, edit, Ctrl+A.
            wait_for(lambda: not state()["cards"][-1]["draft"], "approving a card")
            drain()
            captured.clear()
            send(b" F\r\x1b[D\r")
            wait_for(lambda: b"ANSWER / DEFINITION" in captured, "flipping and moving between flashcards")
            # A portable Office import runs through the real command line and worker.
            document = Path(directory) / "Lecture notes.DOCX"
            with zipfile.ZipFile(document, "w") as archive:
                archive.writestr("word/document.xml", '<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>Replication copies data across nodes.</w:t></w:r></w:p></w:body></w:document>')
            send((":cd " + str(Path(directory).resolve()) + "\r").encode())
            wait_for(lambda: state().get("directory") == str(Path(directory).resolve()), "notebook working directory")
            send((":import " + document.name + "\r").encode())
            wait_for(lambda: len(state().get("sources", [])) == 4, "portable DOCX import")
            assert state()["sources"][-1]["name"] == "Lecture notes.DOCX"
            assert "Replication copies data" in state()["sources"][-1]["content"]
            send(b":notes\r p%c")
            markdown = "# Concepts\n\n**Replication** copies data.\n- [x] Reviewed"
            send(markdown.replace("\n", "\r").encode())
            wait_for(lambda: state().get("notes") == markdown, "cursor-based Markdown notes saved")
            send(b"\x1b")
            time.sleep(0.1)
            drain()
            captured.clear()
            send(b" p")
            wait_for(lambda: "☑".encode() in captured, "rendered Markdown task marker")
            assert b"**Replication**" not in captured
            send(b":read\r")
            wait_for(lambda: b"Source reader" in captured, "separate source popup")
            send(b"\x1b")
            time.sleep(0.1)
            assert state()["notes"] == markdown
            saved_at = path.stat().st_mtime_ns
            send(b":w\r")
            wait_for(lambda: path.stat().st_mtime_ns > saved_at, "explicit command-line save")
            drain()
            captured.clear()
            send(b"\x1b[104;9u")  # <super> + h
            wait_for(lambda: b"Select a notebook" in captured, "returning to home")
            send(b":new Disposable\r")
            wait_for(lambda: len(json.loads(path.read_text())["notebooks"]) == 4, "temporary notebook created")
            drain()
            captured.clear()
            send(b" fDisposable\x04")
            wait_for(lambda: b"Permanently removes notebook data." in captured, "notebook deletion confirmation")
            assert len(json.loads(path.read_text())["notebooks"]) == 4
            send(b"\x1b")
            time.sleep(0.15)
            assert len(json.loads(path.read_text())["notebooks"]) == 4
            drain()
            captured.clear()
            send(b"\x04")
            wait_for(lambda: b"Permanently removes notebook data." in captured, "confirming deletion after cancellation")
            send(b"y")
            wait_for(lambda: len(json.loads(path.read_text())["notebooks"]) == 3, "confirmed notebook deletion persisted")
            assert all(book["name"] != "Disposable" for book in json.loads(path.read_text())["notebooks"])
            send(b"\x1b")
            time.sleep(0.15)
            send(b":q\r")
            # Keep consuming terminal output while the final redraw/restoration runs.
            wait_for(lambda: process.poll() is not None, "quit and terminal restoration")
            drain()
            assert process.returncode == 0
            assert b"38;2;18;58;188" in captured, "Custom theme accent was not rendered"
            assert termios.tcgetattr(slave) == original, "Terminal settings were not restored"
            saved = state()
            assert saved["cards"][-1]["citation"]["source_name"]
            assert saved["cards"][0]["history"] == []
            print("PASS: study workflow → Markdown notes → source popup → notebook deletion/cancellation → save → terminal restore")
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()
            os.close(master)
            os.close(slave)


if __name__ == "__main__":
    main()
