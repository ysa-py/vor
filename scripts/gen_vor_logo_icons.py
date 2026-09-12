#!/usr/bin/env python3
"""
Vor brand icon generator — replaces the old vector "Vector Shield" icon set
with the real Vor logo (1024x1024 master: dark-navy circular HUD emblem,
cyan/purple V shield).

Generates, from ONE master image:
  Android (app + license-manager):
    - mipmap-{mdpi..xxxhdpi}/ic_launcher.png          48/72/96/144/192
    - mipmap-{mdpi..xxxhdpi}/ic_launcher_round.png    (circular alpha mask)
    - mipmap-{mdpi..xxxhdpi}/ic_launcher_foreground.png  108/162/216/324/432
      (full-bleed on the 108dp adaptive canvas — the master is a circular
       composition with a dark background, so launcher circle/squircle masks
       crop only the outer HUD ring edge, by design)
    - drawable-{mdpi..xxxhdpi}/ic_launcher_monochrome.png (luminance->alpha
      silhouette for Android 13+ themed icons)
    - drawable-nodpi/vor_logo.png (512 — in-app branding: About, license gate)
    - adaptive XMLs rewritten: foreground+monochrome point at mipmap bitmaps,
      background = solid #FF020514 (matches the master's corner color)
  Desktop:
    - vor.png (512), installer/vor.ico + docs/logo.ico (16..256 multi-size ICO)
    - internal/server/web/favicon.png (256, RGB) + favicon.ico (16..256)
  iOS:
    - Assets.xcassets/AppIcon.appiconset/icon-1024.png (opaque RGB 1024)

Idempotent: safe to re-run (overwrites outputs deterministically).
"""
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter

REPO = Path("/home/z/my-project/vor-repo")
MASTER = Path("/home/z/my-project/upload/1789162676326.png")

# Adaptive-canvas density scale: 108dp canvas -> px per density bucket.
FG_SIZES = {"mdpi": 108, "hdpi": 162, "xhdpi": 216, "xxhdpi": 324, "xxxhdpi": 432}
LEGACY_SIZES = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}
ICO_SIZES = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]

BG_COLOR = (2, 5, 20)  # master corner color #020514


def load_master() -> Image.Image:
    m = Image.open(MASTER)
    if m.mode != "RGB":
        m = m.convert("RGB")
    if m.size != (1024, 1024):
        m = m.resize((1024, 1024), Image.LANCZOS)
    return m


def circular_mask(size: int) -> Image.Image:
    """High-quality supersampled circular alpha mask."""
    ss = size * 4
    m = Image.new("L", (ss, ss), 0)
    d = ImageDraw.Draw(m)
    d.ellipse((0, 0, ss - 1, ss - 1), fill=255)
    return m.resize((size, size), Image.LANCZOS)


def make_round(img: Image.Image, size: int) -> Image.Image:
    sq = img.resize((size, size), Image.LANCZOS).convert("RGBA")
    sq.putalpha(circular_mask(size))
    return sq


def make_monochrome(img: Image.Image, size: int) -> Image.Image:
    """Luminance -> alpha white silhouette for Android 13+ themed icons.

    The master's background is near-black navy (~brightness 9), the emblem is
    bright (cyan/purple/white, brightness 120-255). A soft S-curve keeps glow
    gradients semi-transparent and crushes the background to nothing.
    """
    src = img.resize((size, size), Image.LANCZOS).convert("RGB")
    out = Image.new("RGBA", (size, size), (255, 255, 255, 0))
    a = Image.eval(src.convert("L"), lambda v: max(0, min(255, int((v - 34) * 255 / 150))))
    # slight blur to smooth the silhouette edges
    a = a.filter(ImageFilter.GaussianBlur(radius=max(1.0, size / 216)))
    out.putalpha(a)
    return out


ADAPTIVE_XML = """<?xml version="1.0" encoding="utf-8"?>
<!-- Vor adaptive launcher icon: the real Vor logo (dark-navy circular HUD
     emblem, cyan/purple V shield) as a full-bleed bitmap foreground.
     Regenerate all densities via scripts/gen_vor_logo_icons.py. -->
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@drawable/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
    <monochrome android:drawable="@mipmap/ic_launcher_monochrome" />
</adaptive-icon>
"""

BG_XML = """<?xml version="1.0" encoding="utf-8"?>
<!-- Adaptive-icon background: solid deep navy matching the logo master's
     own background (#020514). The foreground bitmap is full-bleed opaque,
     so this only shows if the bitmap ever fails to cover the canvas. -->
<color xmlns:android="http://schemas.android.com/apk/res/android"
    android:color="#FF020514" />
"""


def rounded_rect_mask(size: int, radius: int) -> Image.Image:
    """Supersampled rounded-rect alpha mask (distinct silhouette per the
    Material launcher-icon guidance — also satisfies lint IconLauncherShape)."""
    ss = size * 4
    m = Image.new("L", (ss, ss), 0)
    d = ImageDraw.Draw(m)
    d.rounded_rectangle((0, 0, ss - 1, ss - 1), radius=radius * 4, fill=255)
    return m.resize((size, size), Image.LANCZOS)


def gen_android_module(res_dir: Path, master: Image.Image, logo_name: str) -> None:
    for dens, px in FG_SIZES.items():
        mip = res_dir / f"mipmap-{dens}"
        mip.mkdir(parents=True, exist_ok=True)
        master.resize((px, px), Image.LANCZOS).save(mip / "ic_launcher_foreground.png", optimize=True)
        make_monochrome(master, px).save(mip / "ic_launcher_monochrome.png", optimize=True)
    # NOTE: no legacy mipmap-*ic_launcher(.png) bitmaps are generated: with
    # minSdk 26 the unqualified mipmap-anydpi adaptive XML ALWAYS wins over any
    # density bitmap, so legacy PNGs would be dead weight (and lint flags the
    # XML/bitmap pair). Remove any stale ones from older builds when re-running.
    for dens, _px in LEGACY_SIZES.items():
        for stale in (res_dir / f"mipmap-{dens}" / "ic_launcher.png",
                      res_dir / f"mipmap-{dens}" / "ic_launcher_round.png"):
            if stale.exists():
                stale.unlink()
    anydpi = res_dir / "mipmap-anydpi"
    anydpi.mkdir(parents=True, exist_ok=True)
    stale_v26 = res_dir / "mipmap-anydpi-v26"
    if stale_v26.is_dir():
        import shutil
        shutil.rmtree(stale_v26)
    (anydpi / "ic_launcher.xml").write_text(ADAPTIVE_XML)
    (anydpi / "ic_launcher_round.xml").write_text(ADAPTIVE_XML)
    dr = res_dir / "drawable"
    dr.mkdir(parents=True, exist_ok=True)
    (dr / "ic_launcher_background.xml").write_text(BG_XML)
    # obsolete vector layers — replaced by density bitmaps
    for stale in ("ic_launcher_foreground.xml", "ic_launcher_monochrome.xml"):
        p = dr / stale
        if p.exists():
            p.unlink()
    # in-app branding bitmap (density-agnostic single size keeps APK lean)
    nodpi = res_dir / "drawable-nodpi"
    nodpi.mkdir(parents=True, exist_ok=True)
    master.resize((512, 512), Image.LANCZOS).convert("RGB").save(nodpi / logo_name, optimize=True)
    print(f"  [ok] {res_dir}")


def gen_desktop(master: Image.Image) -> None:
    d = REPO / "desktop"
    master.resize((512, 512), Image.LANCZOS).convert("RGB").save(d / "vor.png", optimize=True)
    ico_frames = [master.resize(s, Image.LANCZOS).convert("RGBA") for s in ICO_SIZES]
    ico_frames[-1].save(d / "installer" / "vor.ico", format="ICO", sizes=ICO_SIZES)
    ico_frames[-1].save(d / "docs" / "logo.ico", format="ICO", sizes=ICO_SIZES)
    web = d / "internal" / "server" / "web"
    master.resize((256, 256), Image.LANCZOS).convert("RGB").save(web / "favicon.png", optimize=True)
    ico_frames[-1].save(web / "favicon.ico", format="ICO", sizes=ICO_SIZES)
    print("  [ok] desktop (vor.png, 2x ico, web favicons)")


def gen_ios(master: Image.Image) -> None:
    p = REPO / "ios" / "Vor" / "Assets.xcassets" / "AppIcon.appiconset" / "icon-1024.png"
    master.convert("RGB").save(p, optimize=True)
    print("  [ok] ios AppIcon 1024")


def main() -> None:
    master = load_master()
    print("master:", MASTER, master.size, master.mode)
    print("android/app:")
    gen_android_module(REPO / "android" / "app" / "src" / "main" / "res", master, "vor_logo.png")
    print("android/license-manager:")
    gen_android_module(REPO / "android" / "license-manager" / "src" / "main" / "res", master, "vor_logo.png")
    print("desktop:")
    gen_desktop(master)
    print("ios:")
    gen_ios(master)
    print("DONE — all icon sets regenerated from the Vor logo master.")


if __name__ == "__main__":
    main()
