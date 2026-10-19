#!/usr/bin/env python3
"""Draw Lexicon's app icon and write every file the desktop builds use (needs Pillow).

The artwork: a white page with three lines of text and a red spelling squiggle under the last
one, on a blue tile (the same motif as android/app/src/main/res/drawable/ic_launcher_foreground.xml).
It is drawn here in code and also written as assets/app-icon/wordcraft.svg. The file names keep
WordCraft's, so the build scripts and packaging find them unchanged.

Usage: python3 tools/icons.py
"""

import math
import os

from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIR = os.path.join(ROOT, "assets", "app-icon")
APP_ID = "ai.storyteller.wordcraft"

BLUE = (0x2B, 0x47, 0xB5, 255)
FOLD = (0xC9, 0xD3, 0xF5, 255)
WHITE = (255, 255, 255, 255)
RED = (0xE0, 0x31, 0x31, 255)

# Geometry on a 512 grid.
TILE_R = 112
PAGE = (136, 96, 376, 416)
FOLD_W = 64
LINES = [(166, 190, 346, 214), (166, 240, 346, 264), (166, 290, 286, 314)]
SQUIGGLE = [(166 + 20 * i, 352 + (14 if i % 2 else -2)) for i in range(7)]
SQUIGGLE_W = 12


def squiggle_points():
    return SQUIGGLE


def svg():
    x0, y0, x1, y1 = PAGE
    page = f"M{x0 + 10},{y0}H{x1 - FOLD_W}L{x1},{y0 + FOLD_W}V{y1 - 10}a10,10 0 0 1 -10,10H{x0 + 10}a10,10 0 0 1 -10,-10V{y0 + 10}a10,10 0 0 1 10,-10z"
    fold = f"M{x1 - FOLD_W},{y0}V{y0 + FOLD_W - 10}a10,10 0 0 0 10,10H{x1}z"
    lines = "".join(f'<rect x="{a}" y="{b}" width="{c - a}" height="{d - b}" rx="6" fill="#2B47B5"/>' for a, b, c, d in LINES)
    pts = " ".join(f"{x},{y}" for x, y in squiggle_points())
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
<!-- Lexicon app icon, drawn for this repository (tools/icons.py). -->
<rect width="512" height="512" rx="{TILE_R}" fill="#2B47B5"/>
<path d="{page}" fill="#FFFFFF"/>
<path d="{fold}" fill="#C9D3F5"/>
{lines}
<polyline points="{pts}" fill="none" stroke="#E03131" stroke-width="{SQUIGGLE_W}" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
"""


def draw(size, body=1.0):
    """The icon at `size` px; `body` < 1 leaves a transparent margin (macOS's grid)."""
    ss = 4  # supersampling
    big = size * ss
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    scale = big * body / 512
    off = big * (1 - body) / 2

    def p(x, y):
        return (off + x * scale, off + y * scale)

    d.rounded_rectangle([p(0, 0), p(512, 512)], radius=TILE_R * scale, fill=BLUE)
    x0, y0, x1, y1 = PAGE
    d.rounded_rectangle([p(x0, y0), p(x1, y1)], radius=10 * scale, fill=WHITE)
    # The folded corner: cut the page's corner and draw the fold.
    d.polygon([p(x1 - FOLD_W, y0 - 1), p(x1 + 1, y0 - 1), p(x1 + 1, y0 + FOLD_W)], fill=BLUE)
    d.polygon([p(x1 - FOLD_W, y0), p(x1 - FOLD_W, y0 + FOLD_W), p(x1, y0 + FOLD_W)], fill=FOLD)
    for a, b, c, e in LINES:
        d.rounded_rectangle([p(a, b), p(c, e)], radius=6 * scale, fill=BLUE)
    pts = [p(x, y) for x, y in squiggle_points()]
    w = max(1, round(SQUIGGLE_W * scale))
    d.line(pts, fill=RED, width=w, joint="curve")
    for x, y in (pts[0], pts[-1]):
        r = w / 2
        d.ellipse([x - r, y - r, x + r, y + r], fill=RED)
    return img.resize((size, size), Image.LANCZOS)


def main():
    with open(os.path.join(DIR, "wordcraft.svg"), "w") as f:
        f.write(svg())
    hicolor_svg = os.path.join(DIR, "hicolor", "scalable", "apps", f"{APP_ID}.svg")
    with open(hicolor_svg, "w") as f:
        f.write(svg())
    draw(1024).save(os.path.join(DIR, "wordcraft-1024.png"))
    mac = 824 / 1024
    draw(512, mac).save(os.path.join(DIR, "wordcraft-macos-512.png"))
    for s in (16, 24, 32, 48, 64, 128, 256, 512):
        path = os.path.join(DIR, "hicolor", f"{s}x{s}", "apps", f"{APP_ID}.png")
        draw(s).save(path)
    sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    draw(256).save(os.path.join(DIR, "wordcraft.ico"), sizes=[(s, s) for s in sizes])
    draw(1024, mac).save(os.path.join(DIR, "wordcraft.icns"))
    print("icons written to", DIR)


if __name__ == "__main__":
    main()
