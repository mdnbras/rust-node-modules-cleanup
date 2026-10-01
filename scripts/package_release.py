"""Package one native release executable, preserving Unix executable permissions."""

import argparse
import os
from pathlib import Path
import tarfile
import zipfile

TARGETS = (
    "x86_64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", choices=TARGETS)
    target = parser.parse_args().target
    root = Path(__file__).resolve().parent.parent
    windows = target.endswith("windows-msvc")
    binary_name = "rust-node-modules-cleanup" + (".exe" if windows else "")
    binary = root / "target" / target / "release" / binary_name
    files = [binary, *(root / name for name in ("README.md", "LICENSE", "LICENSE-UPSTREAM"))]
    for path in files:
        if not path.is_file():
            raise FileNotFoundError(path)
    if not windows:
        os.chmod(binary, 0o755)
    dist = root / "dist"
    dist.mkdir(exist_ok=True)
    basename = f"rust-node-modules-cleanup-{target}"
    if windows:
        archive = dist / f"{basename}.zip"
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as bundle:
            for path in files:
                bundle.write(path, arcname=path.name)
    else:
        archive = dist / f"{basename}.tar.gz"
        with tarfile.open(archive, "w:gz") as bundle:
            for path in files:
                bundle.add(path, arcname=path.name)
    print(archive)


if __name__ == "__main__":
    main()
