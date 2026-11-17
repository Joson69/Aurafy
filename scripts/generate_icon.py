"""Package the approved Aurafy logo into Windows icon sizes (Pillow required).

This tightly frames visible artwork and resizes it; it does not redraw the logo.
"""
from pathlib import Path
from PIL import Image

ASSETS = Path(__file__).resolve().parents[1] / "assets"
ASSETS.mkdir(exist_ok=True)
logo = Image.open(ASSETS / "aurafy-logo.png").convert("RGBA")
if logo.width != logo.height:
    raise ValueError("Aurafy logo must be square")
# Ignore near-invisible generated alpha specks when measuring the visible logo.
# Keep the original artwork intact; crop only during Windows icon packaging.
bounds = logo.getchannel("A").point(lambda alpha: 255 if alpha > 16 else 0).getbbox()
if bounds is None:
    raise ValueError("Logo has no visible pixels")
logo = logo.crop(bounds)
side = max(logo.size)
canvas = Image.new("RGBA", (side, side))
canvas.alpha_composite(logo, ((side-logo.width)//2, (side-logo.height)//2))
logo = canvas
sizes = (16, 20, 24, 32, 40, 48, 64, 128, 256)
frames = [logo.resize((size, size), Image.Resampling.LANCZOS) for size in sizes]
frames[-1].save(ASSETS / "aurafy.png")
frames[-1].save(ASSETS / "aurafy.ico", sizes=[f.size for f in frames], append_images=frames[:-1])
frames[sizes.index(64)].save(ASSETS / "aurafy-header.png")
frames[sizes.index(32)].save(ASSETS / "tray.png")
