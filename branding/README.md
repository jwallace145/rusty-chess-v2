# Branding

> [!IMPORTANT]
> **The logo is not free to use.** The artwork is copyright © the artist and
> is used by this project under licence. It is **not** covered by the
> repository's MIT licence, and no rights to it are granted to anyone else.
> Forks and redistributions must remove it. See [LICENSE.md](LICENSE.md).

The rusty-chess logo was drawn for this project by an artist commissioned by
the owner. There are two versions of the mark:

| Mark | File | Use it for |
| --- | --- | --- |
| Big (the main logo) | [`logo.png`](logo.png) | Anywhere with room for detail: the README, docs pages, a site header. |
| Small (simplified) | [`logo-small.png`](logo-small.png) | Small or low-detail spots: favicons, badges, icons under about 64 px. |

Use the big mark wherever it fits, and the small one only when size or detail
is limited.

## Files

| File | Size | Background | Purpose |
| --- | --- | --- | --- |
| `logo.png` | 1024×1024 | Transparent | Main logo; works in GitHub's light and dark themes. |
| `logo-small.png` | 512×512 | Transparent | Simplified mark. |
| `avatar.png` | 1024×1024 | White | Profile pictures (GitHub org or user avatar, other sites); padded so a circular crop keeps the whole head. |
| `social-preview.png` | 1280×640 | White | GitHub repository social preview (the link card on Slack, X, Discord and so on). |
| `favicon-32.png`, `favicon-64.png`, `favicon-180.png` | 32, 64, 180 px | Transparent | Favicons and the Apple touch icon, from the small mark. |
| `original/` | 1206×1609 | White | The artist's original files, unchanged apart from the lowercase extension. |

Every PNG is derived from `original/` by
[`scripts/make_logos.py`](scripts/make_logos.py): it makes the white
background transparent (only white connected to the image border, so the
eyes and teeth stay white), crops, centres on a square canvas and resizes. To
regenerate:

```sh
python3 -m venv .venv && .venv/bin/pip install pillow numpy scipy
.venv/bin/python branding/scripts/make_logos.py branding/original branding
```

The social preview's title uses Arial Bold from macOS.

## Licence

All rights reserved. The artist keeps the copyright, and the project owner
holds a licence to use the artwork, commercial use included. Neither the
licence nor any other right to the artwork passes to anyone who uses, forks or
redistributes this repository. The full terms are in [LICENSE.md](LICENSE.md).
