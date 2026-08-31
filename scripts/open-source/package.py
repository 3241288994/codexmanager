#!/usr/bin/env python3
"""Create a deterministic, source-only CodexManager release archive."""

from __future__ import annotations

import argparse
import gzip
import os
from pathlib import Path
import tarfile


SKIPPED_DIRECTORIES = {
    ".claude",
    ".git",
    ".idea",
    ".next",
    ".playwright",
    ".playwright-cli",
    ".tmp_stage_ps",
    ".tmp_stage_sh",
    ".vscode",
    ".worktrees",
    "__pycache__",
    "blob-report",
    "coverage",
    "data",
    "dist",
    "dist-open-source",
    "gen",
    "generated-images",
    "node_modules",
    "out",
    "playwright-report",
    "portable",
    "test-results",
    "target",
}
SKIPPED_FILE_NAMES = {
    ".DS_Store",
    "Thumbs.db",
    "next-env.d.ts",
}
SKIPPED_SUFFIXES = (
    ".db",
    ".sqlite",
    ".log",
    ".pem",
    ".p12",
    ".pfx",
    ".key",
    ".pyc",
    ".secret",
    ".token",
    ".tsbuildinfo",
)
SKIPPED_ARCHIVE_SUFFIXES = (".tar.gz", ".zip", ".sha256")


def should_skip(relative_path: Path) -> bool:
    parts = relative_path.parts
    if any(part in SKIPPED_DIRECTORIES for part in parts[:-1]):
        return True
    name = relative_path.name
    if name in SKIPPED_FILE_NAMES:
        return True
    if name == ".env" or (name.startswith(".env.") and name != ".env.example"):
        return True
    if name.startswith("codex resume") and name.endswith(".txt"):
        return True
    if name in {
        "auth.json",
        "codexmanager.rpc-token",
        "codexmanager-web-access-password",
        ".web-access-password",
        ".app.json",
    }:
        return True
    return name.endswith(SKIPPED_SUFFIXES) or name.endswith(SKIPPED_ARCHIVE_SUFFIXES)


def source_files(root: Path, output: Path):
    output = output.resolve()
    for current, directories, filenames in os.walk(root, followlinks=False):
        current_path = Path(current)
        relative_dir = current_path.relative_to(root)
        directories[:] = sorted(
            name
            for name in directories
            if name not in SKIPPED_DIRECTORIES
            and not should_skip(relative_dir / name / ".directory")
        )
        for name in sorted(filenames):
            path = current_path / name
            relative_path = path.relative_to(root)
            if should_skip(relative_path) or path.resolve() == output:
                continue
            yield path, relative_path


def normalized_info(
    archive: tarfile.TarFile, path: Path, relative_path: Path, epoch: int
) -> tarfile.TarInfo:
    info = archive.gettarinfo(str(path), arcname=f"./{relative_path.as_posix()}")
    info.uid = 0
    info.gid = 0
    info.uname = ""
    info.gname = ""
    info.mtime = epoch
    return info


def create_archive(root: Path, output: Path, epoch: int) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=epoch) as compressed:
            with tarfile.open(
                fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT
            ) as archive:
                archive.dereference = False
                for path, relative_path in source_files(root, output):
                    info = normalized_info(archive, path, relative_path, epoch)
                    if info.isfile():
                        with path.open("rb") as source:
                            archive.addfile(info, source)
                    else:
                        archive.addfile(info)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    root = args.root.resolve()
    output = args.output.resolve()
    if not root.is_dir():
        parser.error(f"root does not exist: {root}")
    if output.exists():
        parser.error(f"refusing to overwrite existing archive: {output}")
    try:
        epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "0"))
    except ValueError:
        parser.error("SOURCE_DATE_EPOCH must be an integer")
    if epoch < 0:
        parser.error("SOURCE_DATE_EPOCH must be non-negative")

    create_archive(root, output, epoch)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
