#!/usr/bin/env python3
"""Draw Lexicon's app icon and write every file the desktop builds use (needs Pillow).

The artwork follows the owner's picture: a blue-to-pink document with lines of text and a blue
tile with the letter Л. It is drawn here in code and also written as
assets/app-icon/wordcraft.svg and as Android's adaptive-icon foreground
(android/app/src/main/res/drawable/ic_launcher_foreground.xml). The file names keep
WordCraft's, so the build scripts and packaging find them unchanged.

Usage: python3 tools/icons.py
"""

import os

from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIR = os.path.join(ROOT, "assets", "app-icon")
APP_ID = "ai.storyteller.wordcraft"

# The artwork on a 512 grid, back to front. "rrect": (box, radius, top colour, bottom colour);
# "poly": (points, colour).
# The artwork, after the owner's picture: a document with a folded corner, its colour running
# from blue to pink, a lighter page behind it, white lines of text, and a blue tile with the
# letter Л on its left edge. No background tile: the document is the icon.
BACKGROUND = "#FFFFFF"  # Android's adaptive-icon background
BLUE = (0x5B, 0x8E, 0xEE)
PINK = (0xE8, 0x8E, 0xBE)
BACK_BLUE = (0xA9, 0xC3, 0xF6)
BACK_PINK = (0xF2, 0xB9, 0xD6)
FOLD = (0xF7, 0xD9, 0xE9)
TILE_TOP = (0x5F, 0x95, 0xF2)
TILE_BOTTOM = (0x34, 0x6A, 0xDB)
BACK = (124, 58, 370, 432, 28)  # x0, y0, x1, y1, radius
PAGE = (156, 82, 410, 458, 28)
FOLD_W = 74
LINES = [(250, 178, 372), (250, 222, 372), (250, 266, 372), (250, 310, 372), (250, 354, 372), (250, 398, 338)]
LINE_W = 16
TILE = (78, 196, 226, 344, 30)
TILE_RIM = 9
LETTER = [(126, 232), (186, 232), (186, 312), (166, 312), (166, 249), (147, 249), (144, 278), (137, 298), (124, 309), (106, 313), (104, 296), (114, 292), (121, 280), (124, 262)]


def hexc(c):
    return "#%02X%02X%02X" % c


def folded_page_path(x0, y0, x1, y1, r, fold, fmt):
    """The page outline with its top-right corner cut for the fold."""
    return fmt(
        [("M", x0 + r, y0), ("L", x1 - fold, y0), ("L", x1, y0 + fold), ("L", x1, y1 - r), ("Q", x1, y1, x1 - r, y1), ("L", x0 + r, y1), ("Q", x0, y1, x0, y1 - r), ("L", x0, y0 + r), ("Q", x0, y0, x0 + r, y0)]
    )


def svg_path(cmds, m=lambda v: v):
    out = ""
    for c in cmds:
        out += c[0] + ",".join(str(round(m(v), 2)) for v in c[1:])
    return out + "z"


def rrect_path(x0, y0, x1, y1, r, m=lambda v: v):
    return svg_path([("M", x0 + r, y0), ("L", x1 - r, y0), ("Q", x1, y0, x1, y0 + r), ("L", x1, y1 - r), ("Q", x1, y1, x1 - r, y1), ("L", x0 + r, y1), ("Q", x0, y1, x0, y1 - r), ("L", x0, y0 + r), ("Q", x0, y0, x0 + r, y0)], m)


def svg():
    x0, y0, x1, y1, r = PAGE
    page = folded_page_path(x0, y0, x1, y1, r, FOLD_W, svg_path)
    fold = f"M{x1 - FOLD_W},{y0}V{y0 + FOLD_W - 12}q0,12 12,12H{x1}z"
    lines = "".join(f'<line x1="{a}" y1="{y}" x2="{b}" y2="{y}" stroke="#FFFFFF" stroke-opacity="0.9" stroke-width="{LINE_W}" stroke-linecap="round"/>' for a, y, b in LINES)
    tx0, ty0, tx1, ty1, tr = TILE
    pts = " ".join(f"{x},{y}" for x, y in LETTER)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
<!-- Lexicon app icon, drawn for this repository after the owner's picture (tools/icons.py). -->
<defs>
<linearGradient id="p" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="{hexc(BLUE)}"/><stop offset="1" stop-color="{hexc(PINK)}"/></linearGradient>
<linearGradient id="b" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="{hexc(BACK_BLUE)}"/><stop offset="1" stop-color="{hexc(BACK_PINK)}"/></linearGradient>
<linearGradient id="t" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{hexc(TILE_TOP)}"/><stop offset="1" stop-color="{hexc(TILE_BOTTOM)}"/></linearGradient>
</defs>
<path d="{rrect_path(*BACK)}" fill="url(#b)"/>
<path d="{page}" fill="url(#p)"/>
<path d="{fold}" fill="{hexc(FOLD)}"/>
{lines}
<rect x="{tx0}" y="{ty0}" width="{tx1 - tx0}" height="{ty1 - ty0}" rx="{tr}" fill="url(#t)" stroke="#FFFFFF" stroke-width="{TILE_RIM}"/>
<polygon points="{pts}" fill="#FFFFFF"/>
</svg>
"""


def android_foreground():
    """The adaptive icon's foreground (108 dp, content inside the 66 dp safe zone)."""
    s = 0.155

    def m(v):
        return 54 + (v - 256) * s

    def mid(a, b):
        return hexc(tuple((p + q) // 2 for p, q in zip(a, b)))

    x0, y0, x1, y1, r = PAGE
    page = folded_page_path(x0, y0, x1, y1, r, FOLD_W, lambda c: svg_path(c, m))
    fold = svg_path([("M", x1 - FOLD_W, y0), ("L", x1 - FOLD_W, y0 + FOLD_W), ("L", x1, y0 + FOLD_W)], m)
    tile = rrect_path(*TILE, m=m)
    letter = "M" + "L".join(f"{round(m(x), 2)},{round(m(y), 2)}" for x, y in LETTER) + "z"
    lines = "\n".join(
        f'    <path android:strokeColor="#FFFFFF" android:strokeWidth="{round(LINE_W * s, 2)}" android:strokeLineCap="round" android:pathData="M{round(m(a), 2)},{round(m(y), 2)}L{round(m(b), 2)},{round(m(y), 2)}" />'
        for a, y, b in LINES
    )
    return f"""<?xml version="1.0" encoding="utf-8"?>
<!-- Lexicon's icon, drawn for this repository after the owner's picture (generated by
     tools/icons.py): a blue-to-pink document with lines of text and a blue tile with Л. -->
<vector xmlns:android="http://schemas.android.com/apk/res/android"
    android:width="108dp"
    android:height="108dp"
    android:viewportWidth="108"
    android:viewportHeight="108">
    <path android:fillColor="{mid(BACK_BLUE, BACK_PINK)}" android:pathData="{rrect_path(*BACK, m=m)}" />
    <path android:fillColor="{mid(BLUE, PINK)}" android:pathData="{page}" />
    <path android:fillColor="{hexc(FOLD)}" android:pathData="{fold}" />
{lines}
    <path android:fillColor="{mid(TILE_TOP, TILE_BOTTOM)}" android:strokeColor="#FFFFFF" android:strokeWidth="{round(TILE_RIM * s, 2)}" android:pathData="{tile}" />
    <path android:fillColor="#FFFFFF" android:pathData="{letter}" />
</vector>
"""


def gradient(w, h, a, b, horizontal):
    n = 256
    g = Image.new("RGBA", (n, 1) if horizontal else (1, n))
    for i in range(n):
        t = i / (n - 1)
        c = tuple(round(p + (q - p) * t) for p, q in zip(a, b)) + (255,)
        g.putpixel((i, 0) if horizontal else (0, i), c)
    return g.resize((max(1, w), max(1, h)))


def draw(size, body=1.0):
    """The icon at `size` px; `body` < 1 leaves a transparent margin (macOS's grid)."""
    ss = 4  # supersampling
    big = size * ss
    scale = big * body / 512
    off = big * (1 - body) / 2

    def p(x, y):
        return (off + x * scale, off + y * scale)

    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))

    def fill(mask, a, b, horizontal=True):
        nonlocal img
        layer = gradient(big, big, a, b, horizontal)
        img = Image.composite(layer, img, mask)

    def rrect_mask(box, r):
        m = Image.new("L", (big, big), 0)
        x0, y0, x1, y1 = box
        ImageDraw.Draw(m).rounded_rectangle([p(x0, y0), p(x1, y1)], radius=r * scale, fill=255)
        return m

    def shadow(mask, dy, blur, alpha):
        nonlocal img
        sh = Image.new("RGBA", (big, big), (0x30, 0x30, 0x60, 0))
        a = mask.point(lambda v: v * alpha // 255).transform(mask.size, Image.AFFINE, (1, 0, 0, 0, 1, -dy * scale)).filter(ImageFilter.GaussianBlur(blur * scale))
        sh.putalpha(a)
        img = Image.alpha_composite(img, sh)

    # The page behind.
    bx0, by0, bx1, by1, br = BACK
    back = rrect_mask((bx0, by0, bx1, by1), br)
    shadow(back, 8, 12, 60)
    fill(back, BACK_BLUE, BACK_PINK)
    # The page, its corner cut for the fold.
    x0, y0, x1, y1, r = PAGE
    page = rrect_mask((x0, y0, x1, y1), r)
    ImageDraw.Draw(page).polygon([p(x1 - FOLD_W, y0 - 2), p(x1 + 2, y0 - 2), p(x1 + 2, y0 + FOLD_W)], fill=0)
    shadow(page, 10, 14, 90)
    fill(page, BLUE, PINK)
    d = ImageDraw.Draw(img)
    d.polygon([p(x1 - FOLD_W, y0), p(x1 - FOLD_W, y0 + FOLD_W), p(x1, y0 + FOLD_W)], fill=FOLD + (255,))
    w = max(1, round(LINE_W * scale))
    for a, y, b in LINES:
        d.line([p(a, y), p(b, y)], fill=(255, 255, 255, 235), width=w)
        for x in (a, b):
            cx, cy = p(x, y)
            d.ellipse([cx - w / 2, cy - w / 2, cx + w / 2, cy + w / 2], fill=(255, 255, 255, 235))
    # The tile with Л, on a white rim.
    tx0, ty0, tx1, ty1, tr = TILE
    rim = rrect_mask((tx0 - TILE_RIM, ty0 - TILE_RIM, tx1 + TILE_RIM, ty1 + TILE_RIM), tr + TILE_RIM)
    shadow(rim, 6, 10, 90)
    img = Image.composite(Image.new("RGBA", (big, big), (255, 255, 255, 255)), img, rim)
    fill(rrect_mask((tx0, ty0, tx1, ty1), tr), TILE_TOP, TILE_BOTTOM, horizontal=False)
    ImageDraw.Draw(img).polygon([p(x, y) for x, y in LETTER], fill=(255, 255, 255, 255))
    return img.resize((size, size), Image.LANCZOS)


def main():
    with open(os.path.join(DIR, "wordcraft.svg"), "w") as f:
        f.write(svg())
    hicolor_svg = os.path.join(DIR, "hicolor", "scalable", "apps", f"{APP_ID}.svg")
    with open(hicolor_svg, "w") as f:
        f.write(svg())
    fg = os.path.join(ROOT, "android", "app", "src", "main", "res", "drawable", "ic_launcher_foreground.xml")
    with open(fg, "w") as f:
        f.write(android_foreground())
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
