"""Derive web-ready logo assets from the artist's original JPEGs.

Background removal: near-white pixels connected to the image border become
transparent; pixels on the boundary of that region are un-blended from white
so the black outline keeps its anti-aliasing instead of a white halo.
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage

src_dir, out_dir = Path(sys.argv[1]), Path(sys.argv[2])
out_dir.mkdir(parents=True, exist_ok=True)


def cutout(path):
    rgb = np.asarray(Image.open(path).convert("RGB")).astype(np.float32)
    lo = rgb.min(axis=2)
    near_white = lo > 235
    labels, _ = ndimage.label(near_white)
    edge_labels = set(np.unique(np.concatenate(
        [labels[0], labels[-1], labels[:, 0], labels[:, -1]]))) - {0}
    bg = np.isin(labels, list(edge_labels))
    # Boundary band: within 2px of the background, outside it.
    band = ndimage.binary_dilation(bg, iterations=2) & ~bg
    alpha = np.where(bg, 0.0, 1.0)
    a_band = np.clip((255.0 - lo) / 255.0 * 1.15, 0.0, 1.0)
    alpha = np.where(band, a_band, alpha)
    a = np.maximum(alpha, 1e-3)[..., None]
    color = np.clip((rgb - (1.0 - a) * 255.0) / a, 0, 255)
    color = np.where(band[..., None], color, rgb)
    rgba = np.dstack([color, alpha * 255.0]).round().astype(np.uint8)
    img = Image.fromarray(rgba, "RGBA")
    return img.crop(img.getbbox())


def square(img, size, pad, bg=(0, 0, 0, 0)):
    inner = int(size * (1 - 2 * pad))
    scale = inner / max(img.size)
    fit = img.resize((round(img.width * scale), round(img.height * scale)),
                     Image.LANCZOS)
    canvas = Image.new("RGBA", (size, size), bg)
    canvas.alpha_composite(fit, ((size - fit.width) // 2,
                                 (size - fit.height) // 2))
    return canvas


big = cutout(src_dir / "braveheart-logo-big.jpg")
small = cutout(src_dir / "braveheart-logo-small.jpg")

# Transparent masters for docs/README (big) and small spaces (small).
square(big, 1024, 0.04).save(out_dir / "logo.png", optimize=True)
square(small, 512, 0.04).save(out_dir / "logo-small.png", optimize=True)

# Avatar: opaque white, padded so a circular crop keeps the whole head.
square(big, 1024, 0.12, (255, 255, 255, 255)).convert("RGB").save(
    out_dir / "avatar.png", optimize=True)

# Favicons from the simpler mark: detail survives at 32px.
for px in (32, 64, 180):
    square(small, px, 0.02).save(out_dir / f"favicon-{px}.png", optimize=True)

# GitHub social preview, 1280x640 (2:1), logo left, name right.
W, H = 1280, 640
card = Image.new("RGBA", (W, H), (255, 255, 255, 255))
mark = square(big, 560, 0.0)
card.alpha_composite(mark, (60, (H - 560) // 2))
draw = ImageDraw.Draw(card)
font_paths = ["/System/Library/Fonts/Supplemental/Arial Bold.ttf",
              "/System/Library/Fonts/Helvetica.ttc"]
font = next(ImageFont.truetype(p, 92) for p in font_paths if Path(p).exists())
sub = next(ImageFont.truetype(p, 36) for p in font_paths if Path(p).exists())
draw.text((670, 250), "Braveheart", font=font, fill=(18, 10, 10))
draw.text((674, 370), "A UCI chess engine in Rust", font=sub,
          fill=(60, 60, 60))
card.convert("RGB").save(out_dir / "social-preview.png", optimize=True)
print("ok", big.size, small.size)
