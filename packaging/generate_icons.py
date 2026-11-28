#!/usr/bin/env python3
"""Regenerate every Hangman icon from the design below. Needs rsvg-convert and ImageMagick.

Usage: python3 packaging/generate_icons.py
"""
import struct
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

DEFS = """<defs>
<linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#343c55"/><stop offset="1" stop-color="#12141b"/></linearGradient>
<linearGradient id="gold" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffd877"/><stop offset="1" stop-color="#eca63c"/></linearGradient>
</defs>"""

# Gallows with a "?" letter tile hanging from the rope.
ART = """<g fill="none" stroke-linecap="round" stroke-linejoin="round" stroke-width="68">
<path stroke="#a8743f" d="M170 856H600"/><path stroke="#bb844c" d="M280 850V176H690"/><path stroke="#a8743f" d="M280 350L450 180"/>
</g>
<path d="M668 180V420" stroke="#e0cc9c" stroke-width="30" stroke-linecap="round"/>
<g transform="rotate(8 668 420)">
<rect x="498" y="420" width="340" height="360" rx="64" fill="#8f5f1d"/>
<rect x="498" y="400" width="340" height="354" rx="64" fill="url(#gold)"/>
<path d="M604 522C604 436 738 438 738 522C738 582 668 582 668 640" fill="none" stroke="#252a3a" stroke-width="54" stroke-linecap="round"/>
<circle cx="668" cy="705" r="33" fill="#252a3a"/>
</g>"""

# Tile only, for 16px where the gallows turns to mush.
SMALL_ART = """<rect x="160" y="128" width="704" height="768" rx="144" fill="url(#gold)"/>
<path d="M366 380C366 200 658 204 658 380C658 510 512 508 512 628" fill="none" stroke="#252a3a" stroke-width="124" stroke-linecap="round"/>
<circle cx="512" cy="780" r="70" fill="#252a3a"/>"""


def svg(art, radius=228):
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">\n'
        f'{DEFS}\n<rect width="1024" height="1024" rx="{radius}" fill="url(#bg)"/>\n{art}\n</svg>\n'
    )


def mac_svg(art):
    """macOS icons sit on an 824px rounded square inside a 1024px canvas, with a soft shadow."""
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">\n'
        f"{DEFS}\n"
        '<rect x="100" y="112" width="824" height="824" rx="185" fill="#000" opacity=".3"/>\n'
        f'<g transform="translate(100 100) scale({824 / 1024})">\n'
        f'<rect width="1024" height="1024" rx="230" fill="url(#bg)"/>\n{art}\n</g>\n</svg>\n'
    )


def render(source, size, out):
    subprocess.run(["rsvg-convert", "-w", str(size), "-h", str(size), "-o", str(out)],
                   input=source.encode(), check=True)
    return out


def main():
    (ROOT / "packaging/linux/hangman.svg").write_text(svg(ART))
    render(svg(SMALL_ART), 16, ROOT / "packaging/linux/hangman-16.png")

    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)

        # Windows: multi-size .ico, squarer corners.
        pngs = [render(svg(SMALL_ART if s == 16 else ART, 96), s, tmp / f"win{s}.png")
                for s in (16, 24, 32, 48, 64, 128, 256)]
        subprocess.run(["magick", *map(str, pngs), str(ROOT / "packaging/windows/hangman.ico")], check=True)

        # macOS: .icns is a header followed by tagged PNG entries.
        entries = b""
        for tag, size in ((b"icp4", 16), (b"icp5", 32), (b"ic11", 32), (b"ic12", 64), (b"ic07", 128),
                          (b"ic13", 256), (b"ic08", 256), (b"ic14", 512), (b"ic09", 512), (b"ic10", 1024)):
            art = SMALL_ART if size == 16 else ART
            data = render(mac_svg(art), size, tmp / "mac.png").read_bytes()
            entries += tag + struct.pack(">I", len(data) + 8) + data
        (ROOT / "packaging/macos/hangman.icns").write_bytes(
            b"icns" + struct.pack(">I", len(entries) + 8) + entries)

        # Raw RGBA for the running window's icon (Windows and X11), so the app needs no PNG decoder.
        window = render(svg(ART, 96), 64, tmp / "window.png")
        subprocess.run(["magick", str(window), "-depth", "8",
                        f"RGBA:{ROOT / 'crates/hangman/assets/window-icon.rgba'}"], check=True)


if __name__ == "__main__":
    main()
