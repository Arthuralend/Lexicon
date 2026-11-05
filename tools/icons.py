#!/usr/bin/env python3
"""Write Office Pro Max's app icons (needs Pillow).

- The suite icon (two documents, W and X, on a dark tile with an orange corner) is the owner's
  picture, cut out of its white background: assets/app-icon/source/office-pro-max.png. It is the
  installer's and the disk image's icon.
- The two apps' icons are drawn here in the same style: the documents app (a page of text with a
  W tile) and the sheets app (a page with a table and an X tile).

Outputs keep the old file names (wordcraft-*.png, wordcraft.ico/.icns, hicolor/…), so the build
scripts and packaging find the documents app's icon unchanged; the sheets app's are sheets-*.

Usage: python3 tools/icons.py
"""

import os

from PIL import Image, ImageDraw, ImageFilter, ImageFont

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIR = os.path.join(ROOT, "assets", "app-icon")
RES = os.path.join(ROOT, "android", "app", "src", "main", "res")
APP_ID = "ai.storyteller.wordcraft"
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"

DARK_TOP = (0x4A, 0x4A, 0x4D)
DARK_BOTTOM = (0x2C, 0x2C, 0x2F)
ORANGE = (0xF6, 0x8C, 0x12)
ORANGE_LIGHT = (0xFB, 0xA9, 0x2A)
PAPER = (0xEE, 0xEB, 0xE7)
FOLD = (0xDC, 0xD7, 0xD1)
INK = (0x2B, 0x2B, 0x2D)
TILE_R = 0.215  # of the side


def vgradient(w, h, top, bottom):
    g = Image.new("RGBA", (1, 256))
    for y in range(256):
        t = y / 255
        g.putpixel((0, y), tuple(round(a + (b - a) * t) for a, b in zip(top, bottom)) + (255,))
    return g.resize((w, h))


def app_icon(size, kind, body=1.0, tile=True):
    """The documents (`W`) or sheets (`X`) app's icon at `size` px."""
    ss = 4
    big = size * ss
    s = big * body
    off = big * (1 - body) / 2
    img = Image.new("RGBA", (big, big), (0, 0, 0, 0))

    def p(x, y):
        return (off + x * s, off + y * s)

    if tile:
        mask = Image.new("L", (big, big), 0)
        ImageDraw.Draw(mask).rounded_rectangle([p(0, 0), p(1, 1)], radius=TILE_R * s, fill=255)
        img = Image.composite(vgradient(big, big, DARK_TOP, DARK_BOTTOM), img, mask)
        # The orange corner band, inside the tile.
        band = Image.new("L", (big, big), 0)
        ImageDraw.Draw(band).polygon([p(0.58, -0.01), p(1.01, -0.01), p(1.01, 0.40)], fill=255)
        band = Image.composite(band, Image.new("L", (big, big), 0), mask)
        img = Image.composite(vgradient(big, big, ORANGE_LIGHT, ORANGE), img, band)

    # The page with its folded corner, on a soft shadow.
    x0, y0, x1, y1, r, fold = 0.25, 0.17, 0.80, 0.86, 0.045, 0.12
    page = Image.new("L", (big, big), 0)
    ImageDraw.Draw(page).rounded_rectangle([p(x0, y0), p(x1, y1)], radius=r * s, fill=255)
    ImageDraw.Draw(page).polygon([p(x1 - fold, y0 - 0.01), p(x1 + 0.01, y0 - 0.01), p(x1 + 0.01, y0 + fold)], fill=0)
    sh = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    sh.putalpha(page.point(lambda v: v * 110 // 255).transform(page.size, Image.AFFINE, (1, 0, 0, 0, 1, -0.018 * s)).filter(ImageFilter.GaussianBlur(0.02 * s)))
    img = Image.alpha_composite(img, sh)
    img = Image.composite(Image.new("RGBA", (big, big), PAPER + (255,)), img, page)
    d = ImageDraw.Draw(img)
    d.polygon([p(x1 - fold, y0), p(x1 - fold, y0 + fold), p(x1, y0 + fold)], fill=FOLD + (255,))

    if kind == "W":
        w = max(1, round(0.045 * s))
        for i, (a, b) in enumerate([(0.33, 0.70), (0.33, 0.70), (0.33, 0.70), (0.33, 0.62)]):
            y = 0.53 + i * 0.075
            d.line([p(a, y), p(b, y)], fill=INK + (255,), width=w)
            for x in (a, b):
                cx, cy = p(x, y)
                d.ellipse([cx - w / 2, cy - w / 2, cx + w / 2, cy + w / 2], fill=INK + (255,))
    else:
        gx0, gy0, gx1, gy1 = 0.32, 0.52, 0.73, 0.80
        lw = max(1, round(0.016 * s))
        d.rounded_rectangle([p(gx0, gy0), p(gx1, gy1)], radius=0.015 * s, outline=INK + (255,), width=lw)
        for i in (1, 2):
            x = gx0 + (gx1 - gx0) * i / 3
            d.line([p(x, gy0), p(x, gy1)], fill=INK + (255,), width=lw)
        for i in (1, 2, 3):
            y = gy0 + (gy1 - gy0) * i / 4
            d.line([p(gx0, y), p(gx1, y)], fill=INK + (255,), width=lw)

    # The orange letter tile on the page's top left.
    tx0, ty0, tx1, ty1 = 0.17, 0.23, 0.44, 0.50
    tmask = Image.new("L", (big, big), 0)
    ImageDraw.Draw(tmask).rounded_rectangle([p(tx0, ty0), p(tx1, ty1)], radius=0.06 * s, fill=255)
    tsh = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    tsh.putalpha(tmask.point(lambda v: v * 120 // 255).transform(tmask.size, Image.AFFINE, (1, 0, 0, 0, 1, -0.012 * s)).filter(ImageFilter.GaussianBlur(0.015 * s)))
    img = Image.alpha_composite(img, tsh)
    img = Image.composite(vgradient(big, big, ORANGE_LIGHT, ORANGE), img, tmask)
    font = ImageFont.truetype(FONT, round(0.21 * s))
    cx, cy = p((tx0 + tx1) / 2, (ty0 + ty1) / 2)
    ImageDraw.Draw(img).text((cx, cy), kind, font=font, fill=INK + (255,), anchor="mm")
    return img.resize((size, size), Image.LANCZOS)


def suite_icon(size, body=1.0):
    src = Image.open(os.path.join(DIR, "source", "office-pro-max.png")).convert("RGBA")
    inner = round(size * body)
    out = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    out.alpha_composite(src.resize((inner, inner), Image.LANCZOS), ((size - inner) // 2, (size - inner) // 2))
    return out


def write_set(prefix, draw):
    mac = 824 / 1024
    draw(1024).save(os.path.join(DIR, f"{prefix}-1024.png"))
    draw(512, mac).save(os.path.join(DIR, f"{prefix}-macos-512.png"))
    sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    draw(256).save(os.path.join(DIR, f"{prefix}.ico"), sizes=[(s, s) for s in sizes])
    draw(1024, mac).save(os.path.join(DIR, f"{prefix}.icns"))


def android(name, kind):
    """An adaptive icon: the dark tile with its orange corner as the background, the page as the
    foreground (in the 66 dp safe zone), and a legacy icon for older launchers."""
    for dpi, scale in [("mdpi", 1), ("hdpi", 1.5), ("xhdpi", 2), ("xxhdpi", 3), ("xxxhdpi", 4)]:
        folder = os.path.join(RES, f"mipmap-{dpi}")
        os.makedirs(folder, exist_ok=True)
        px = round(108 * scale)
        app_icon(px, kind, body=1.0).save(os.path.join(folder, f"{name}_background.png"))
        fg = app_icon(px, kind, body=0.78, tile=False)
        fg.save(os.path.join(folder, f"{name}_foreground.png"))
        app_icon(round(48 * scale), kind).save(os.path.join(folder, f"{name}.png"))
    # The background layer is the tile alone (no page): redraw it without the page.
    for dpi, scale in [("mdpi", 1), ("hdpi", 1.5), ("xhdpi", 2), ("xxhdpi", 3), ("xxxhdpi", 4)]:
        px = round(108 * scale)
        ss = 4
        big = px * ss
        img = vgradient(big, big, DARK_TOP, DARK_BOTTOM)
        band = Image.new("L", (big, big), 0)
        ImageDraw.Draw(band).polygon([(0.58 * big, 0), (big, 0), (big, 0.40 * big)], fill=255)
        img = Image.composite(vgradient(big, big, ORANGE_LIGHT, ORANGE), img, band)
        img.resize((px, px), Image.LANCZOS).save(os.path.join(RES, f"mipmap-{dpi}", f"{name}_background.png"))
    anydpi = os.path.join(RES, "mipmap-anydpi-v26")
    os.makedirs(anydpi, exist_ok=True)
    with open(os.path.join(anydpi, f"{name}.xml"), "w") as f:
        f.write(
            f"""<?xml version="1.0" encoding="utf-8"?>
<!-- Generated by tools/icons.py. -->
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@mipmap/{name}_background" />
    <foreground android:drawable="@mipmap/{name}_foreground" />
</adaptive-icon>
"""
        )


def main():
    write_set("wordcraft", lambda size, body=1.0: app_icon(size, "W", body))
    write_set("sheets", lambda size, body=1.0: app_icon(size, "X", body))
    write_set("suite", suite_icon)
    for s in (16, 24, 32, 48, 64, 128, 256, 512):
        app_icon(s, "W").save(os.path.join(DIR, "hicolor", f"{s}x{s}", "apps", f"{APP_ID}.png"))
    android("ic_launcher", "W")
    android("ic_sheets", "X")
    print("icons written to", DIR)


if __name__ == "__main__":
    main()
