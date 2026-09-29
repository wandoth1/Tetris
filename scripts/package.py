#!/usr/bin/env python3
"""Package native binaries; standard library only. No downloads or credentials."""
import argparse
import os
from pathlib import Path
import plistlib
import shutil
import struct
import subprocess
import sys
import zipfile
import zlib

ROOT = Path(__file__).resolve().parents[1]


def icon_png(size: int = 512) -> bytes:
    """Original geometric icon; no fonts or external artwork."""
    rows = bytearray()
    blocks = [(1, 0, (73, 236, 225)), (0, 1, (116, 136, 255)),
              (1, 1, (165, 109, 245)), (2, 1, (244, 107, 177))]
    for y in range(size):
        rows.append(0)
        for x in range(size):
            u, v = x / size, y / size
            radius = 0.15
            dx, dy = max(0, abs(u - 0.5) - (0.5 - radius)), max(0, abs(v - 0.5) - (0.5 - radius))
            a = 255 if dx * dx + dy * dy <= radius * radius else 0
            rgb = (10, 16, 31)
            for bx, by, co in blocks:
                left, top = 0.19 + bx * 0.21, 0.27 + by * 0.21
                if left <= u <= left + 0.19 and top <= v <= top + 0.19:
                    border = min(u - left, left + 0.19 - u, v - top, top + 0.19 - v)
                    factor = 1.0 if border < 0.012 else 0.63
                    rgb = tuple(int(c * factor) for c in co)
                    if top + 0.018 < v < top + 0.028 and border > 0.018:
                        rgb = tuple(min(255, c + 30) for c in rgb)
            rows.extend((*rgb, a))
    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(bytes(rows), 9)) + chunk(b'IEND', b'')


def archive(folder: Path, destination: Path) -> None:
    with zipfile.ZipFile(destination, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for path in sorted(folder.rglob('*')):
            if path.is_file():
                # ZipInfo.from_file retains POSIX execute bits for the Mac binary.
                z.write(path, path.relative_to(folder))


def main() -> None:
    p = argparse.ArgumentParser()
    p.add_argument('--target', required=True)
    p.add_argument('--platform', required=True, choices=['Windows-x64', 'macOS-AppleSilicon', 'macOS-Intel'])
    args = p.parse_args()
    dist = ROOT / 'dist'
    stage = dist / args.platform
    if stage.exists():
        shutil.rmtree(stage)
    stage.mkdir(parents=True)
    binary = ROOT / 'target' / args.target / 'release' / ('tetris.exe' if args.platform == 'Windows-x64' else 'tetris')
    if not binary.is_file():
        raise FileNotFoundError(binary)
    if args.platform == 'Windows-x64':
        shutil.copy2(binary, stage / 'Tetris.exe')
    else:
        if sys.platform != 'darwin':
            raise RuntimeError('macOS packaging/signing must run on a Mac')
        app = stage / 'Tetris Neon Pulse.app'
        contents = app / 'Contents'
        (contents / 'MacOS').mkdir(parents=True)
        (contents / 'Resources').mkdir()
        exe = contents / 'MacOS' / 'tetris'
        shutil.copy2(binary, exe)
        exe.chmod(0o755)
        info = {'CFBundleName': 'Tetris Neon Pulse', 'CFBundleDisplayName': 'Tetris Neon Pulse',
                'CFBundleIdentifier': 'com.wandoth.neonpulse', 'CFBundleExecutable': 'tetris',
                'CFBundleVersion': '1', 'CFBundleShortVersionString': '0.1.0',
                'CFBundlePackageType': 'APPL', 'CFBundleIconFile': 'AppIcon.icns',
                'LSMinimumSystemVersion': '11.0', 'NSHighResolutionCapable': True,
                'NSPrincipalClass': 'NSApplication', 'NSHumanReadableCopyright': '2026 wandoth1'}
        with (contents / 'Info.plist').open('wb') as f:
            plistlib.dump(info, f)
        png = icon_png()
        icon_chunk = b'ic09' + struct.pack('>I', len(png) + 8) + png
        (contents / 'Resources' / 'AppIcon.icns').write_bytes(b'icns' + struct.pack('>I', len(icon_chunk) + 8) + icon_chunk)
        subprocess.run(['codesign', '--force', '--sign', '-', str(app)], check=True)
        subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
        subprocess.run(['file', str(exe)], check=True)
    shutil.copy2(ROOT / 'README.md', stage / 'LEEME.md')
    shutil.copy2(ROOT / 'LICENSE', stage / 'LICENSE')
    lock = ROOT / 'Cargo.lock'
    if lock.exists():
        shutil.copy2(lock, stage / 'Cargo.lock')
    (stage / 'BUILD.txt').write_text('Commit: ' + os.environ.get('GITHUB_SHA', 'local') + '\nTarget: ' + args.target + '\n', encoding='utf-8')
    output = dist / f'Tetris-NeonPulse-{args.platform}.zip'
    archive(stage, output)
    print(output)


if __name__ == '__main__':
    main()
