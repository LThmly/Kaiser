#!/usr/bin/env python3
"""Reject private runtime data and agent instructions in the Git publication set."""
import argparse
import fnmatch
from pathlib import Path, PurePosixPath
import re
import subprocess


PRIVATE_DIRECTORIES = {
    ".agents", ".aws", ".claude", ".codex", ".cursor", ".gemini", ".opencode",
    ".roo", ".clinerules", ".kaiser", "auth", "notebooks", "__pycache__", "target",
}
PRIVATE_NAMES = (
    "agents*.md", "claude*.md", "codex*.md", "gemini*.md", "memory.md",
    "copilot-instructions.md", ".cursorrules", ".windsurfrules", ".aider*",
    ".env*", "state.json", "config.local.toml", "config.*.local.toml", "*.pyc",
)
SECRET_PATTERNS = (
    re.compile(r"\bsk-(?:proj-|svcacct-)?[A-Za-z0-9_-]{24,}"),
    re.compile(r"\bgh[pousr]_[A-Za-z0-9]{30,}"),
    re.compile(r"\bgithub_pat_[A-Za-z0-9_]{30,}"),
    re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"),
)


def forbidden(path):
    parts = PurePosixPath(path.lower()).parts
    return (
        any(part in PRIVATE_DIRECTORIES for part in parts[:-1])
        or any(fnmatch.fnmatchcase(parts[-1], pattern) for pattern in PRIVATE_NAMES)
        or path.lower().startswith(("data/", "sources/", ".github/instructions/"))
        or path.lower().endswith("ai/usage.json")
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    args = parser.parse_args()
    root = args.repo.resolve()
    result = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        check=True, capture_output=True,
    )
    files = [name.decode("utf-8") for name in result.stdout.split(b"\0") if name]
    if not files:
        raise SystemExit("No tracked files to audit; stage the publication files first.")
    problems = []
    for name in files:
        path = root / name
        if forbidden(name) or path.is_symlink():
            problems.append(f"Excluded publication path: {name}")
        elif path.stat().st_size < 2_000_000:
            text = path.read_bytes().decode("utf-8", errors="replace")
            if any(pattern.search(text) for pattern in SECRET_PATTERNS):
                problems.append(f"Possible credential in: {name}")
    if problems:
        raise SystemExit("\n".join(problems))
    print(f"PASS: {len(files)} tracked files; no private data, agent instructions, or credential patterns")


if __name__ == "__main__":
    main()
