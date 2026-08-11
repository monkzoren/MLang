#!/usr/bin/env python3
"""Bake the ⌶ text-op font: a grayscale bitmap strip the VM draws from.

The GUI ops render text deterministically on every platform because the
glyphs are pixels in a committed asset, not a runtime font stack. This
script regenerates compiler/src/font.bin from system TTFs; the .bin is
what ships, so running the script is only needed to change the glyph set.

Cell: 8×16 px, baseline at y=12. Coverage: ASCII, every character that
appears in the repo's .ml sources, and a set of UI glyphs — rasterized
from DejaVu Sans Mono, falling back to DejaVu Sans, FreeMono, and GNU
Unifont for exotic code points. Characters no font covers are skipped;
the VM draws a hollow box for anything missing from the strip.

Format (little-endian):
  "MFNT" u8=1 | cell_w u8 | cell_h u8 | baseline u8 | count u32
  then count × { codepoint u32 | cell_w×cell_h alpha bytes }
sorted by codepoint (the VM binary-searches it).

Usage: python3 bake.py   (writes ../src/font.bin)
"""
import glob
import os
import struct

from fontTools.ttLib import TTFont
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "src", "font.bin")
REPO = os.path.join(HERE, "..", "..")

CELL_W, CELL_H, BASELINE = 8, 16, 12

# (path, pixel size, baseline) — tried in order per character.
FONTS = [
    ("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf", 13, BASELINE),
    ("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 12, BASELINE),
    ("/usr/share/fonts/truetype/freefont/FreeMono.ttf", 14, BASELINE),
    ("/usr/share/fonts/opentype/unifont/unifont.otf", 16, 14),
]

UI_EXTRAS = (
    "─│┌┐└┘├┤┬┴╌╍▏▕▔▁▂▃▄▅▆▇█▉▊▋▌▍▎░▒▓"
    "▪▫■□●○◦◆◇×✕✓✗▸▹►▾▿▼◂◃◄▴▵▲‹›«»…·•—–"
    "→←↑↓⇥⌫⌦⎋↵⌖§¶©"
)


def wanted_chars():
    chars = set(chr(c) for c in range(32, 127))
    chars |= set(UI_EXTRAS)
    for pattern in ("examples/*.ml", "std/*.ml"):
        for path in glob.glob(os.path.join(REPO, pattern)):
            with open(path, encoding="utf-8") as f:
                chars |= set(f.read())
    chars -= {"\n", "\r", "\t"}
    return chars


def load_fonts():
    loaded = []
    for path, size, baseline in FONTS:
        if not os.path.exists(path):
            continue
        cmap = TTFont(path).getBestCmap()
        pil = ImageFont.truetype(path, size)
        loaded.append((pil, set(cmap.keys()), baseline))
    return loaded


def render(ch, fonts):
    for pil, cmap, baseline in fonts:
        if ord(ch) not in cmap:
            continue
        img = Image.new("L", (CELL_W, CELL_H), 0)
        draw = ImageDraw.Draw(img)
        # center horizontally so proportional-font fallbacks sit in the cell
        width = draw.textlength(ch, font=pil)
        x = max(0, (CELL_W - width) // 2)
        draw.text((x, baseline), ch, font=pil, fill=255, anchor="ls")
        return img.tobytes()
    return None


def main():
    fonts = load_fonts()
    glyphs = []
    skipped = []
    for ch in sorted(wanted_chars()):
        data = render(ch, fonts)
        if data is None:
            skipped.append(ch)
        else:
            glyphs.append((ord(ch), data))
    glyphs.sort()
    with open(OUT, "wb") as f:
        f.write(b"MFNT")
        f.write(struct.pack("<BBBBI", 1, CELL_W, CELL_H, BASELINE, len(glyphs)))
        for cp, data in glyphs:
            f.write(struct.pack("<I", cp))
            f.write(data)
    size = os.path.getsize(OUT)
    print(f"baked {len(glyphs)} glyphs → {OUT} ({size} bytes)")
    if skipped:
        print(f"skipped (no coverage): {''.join(skipped)}")


if __name__ == "__main__":
    main()
