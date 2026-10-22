#!/usr/bin/env python3
"""Generate Lexicon's original mint document-and-orbit icon (Pillow required)."""
import os
from PIL import Image, ImageDraw
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DIR = os.path.join(ROOT, "assets", "app-icon")
APP_ID = "ai.storyteller.wordcraft"
MINT = "#BFF3F2"
DARK = "#282D39"
PAGE = [(186,172),(278,172),(320,214),(320,334),(186,334),(186,172)]
FOLD = [(278,172),(278,214),(320,214)]
LETTER = [(226,222),(226,292),(274,292)]
ORBITS = [((166,269),(52,248),(132,128),(322,126)), ((344,226),(451,241),(402,379),(160,374))]

def svg():
    return '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">
<circle cx="256" cy="256" r="248" fill="#282D39"/>
<g fill="none" stroke="#BFF3F2" stroke-width="10" stroke-linecap="round" stroke-linejoin="round">
<path d="M186 172H278L320 214V334H186Z M278 172V214H320 M226 222V292H274"/>
<path d="M166 269C52 248 132 128 322 126 M344 226C451 241 402 379 160 374"/>
<circle cx="340" cy="126" r="18"/><circle cx="142" cy="374" r="18"/>
</g></svg>
'''

def android_foreground():
    return '''<?xml version="1.0" encoding="utf-8"?>
<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="512" android:viewportHeight="512">
<group android:scaleX="0.76" android:scaleY="0.76" android:translateX="61.44" android:translateY="61.44">
<path android:fillColor="@android:color/transparent" android:strokeColor="#BFF3F2" android:strokeWidth="10" android:strokeLineCap="round" android:strokeLineJoin="round" android:pathData="M186,172H278L320,214V334H186Z M278,172V214H320 M226,222V292H274 M166,269C52,248 132,128 322,126 M344,226C451,241 402,379 160,374 M358,126A18,18 0,1 1,322,126A18,18 0,1 1,358,126 M160,374A18,18 0,1 1,124,374A18,18 0,1 1,160,374"/>
</group></vector>
'''

def draw(size, body=1.0):
    ss = 4
    image = Image.new("RGBA", (size*ss,size*ss))
    d = ImageDraw.Draw(image)
    scale = size*ss*body/512
    offset = size*ss*(1-body)/2
    def p(x,y): return (offset+x*scale,offset+y*scale)
    def circle(x,y,r,fill,outline=None,width=1):
        d.ellipse([p(x-r,y-r),p(x+r,y+r)],fill=fill,outline=outline,width=max(1,round(width*scale)))
    circle(256,256,248,DARK)
    def line(points):
        d.line([p(*v) for v in points],fill=MINT,width=max(1,round(10*scale)),joint="curve")
        for x,y in points: circle(x,y,5,MINT)
    for points in (PAGE,FOLD,LETTER): line(points)
    for a,b,c,e in ORBITS:
        points=[]
        for i in range(101):
            t=i/100;u=1-t
            points.append(tuple(u*u*u*a[j]+3*u*u*t*b[j]+3*u*t*t*c[j]+t*t*t*e[j] for j in (0,1)))
        line(points)
    for x,y in [(340,126),(142,374)]: circle(x,y,18,DARK,MINT,10)
    return image.resize((size,size),Image.Resampling.LANCZOS)

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
