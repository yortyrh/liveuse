# Banner

`banner.html` is the source of the LiveUse banner: a 1200×600 page drawn with the tokens in
`web/src/app/globals.css` (ground `#eef0f2`, brand blue `#2e62d9`, pointer `#3ee0e8`, spark
`#f5b429`, Archivo and Martian Mono) and the LuLu render from `web/public/case/assembled.png`. It
follows the site hero. Edit the HTML, then render it.

## Where the image is used

| File                              | Size     | Used by                                                       |
| --------------------------------- | -------- | ------------------------------------------------------------- |
| `.github/social-preview.png`      | 1280×640 | GitHub social preview (uploaded by hand)                      |
| `web/src/app/opengraph-image.png` | 1200×630 | `og:image` and `twitter:image` of liveuse.sitewebr.com        |
| `yorty.info` portfolio            | 1800×900 | `src/assets/projects/liveuse-banner.jpg`, the project card    |

The README keeps `web/public/demo.gif` as its main image.

GitHub has no API for the repository social preview. Upload `.github/social-preview.png` by hand in
**Settings → General → Social preview**.

## Render

Run from this folder. Chrome headless loads the Google Fonts; raise `--virtual-time-budget` if the
text comes out in a system font. The canvas centers itself, so a taller window adds an even band.

```sh
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
shot() { "$CHROME" --headless --hide-scrollbars --virtual-time-budget=6000 "$@" "file://$PWD/banner.html"; }

# GitHub social preview, 1280×640
shot --window-size=1200,600 --force-device-scale-factor=1.0667 --screenshot="$PWD/../../.github/social-preview.png"

# Site Open Graph image, 1200×630
shot --window-size=1200,630 --force-device-scale-factor=1 --screenshot="$PWD/../../web/src/app/opengraph-image.png"

# Portfolio card, 1800×900 JPG: copy it to yorty.info/src/assets/projects/, then delete it here
shot --window-size=1200,600 --force-device-scale-factor=1.5 --screenshot="$PWD/card.png"
sips -s format jpeg -s formatOptions 92 card.png --out liveuse-banner.jpg && rm card.png
```

## Before you change it

- Follow `PRODUCT.md`: the app is LiveUse, the device is LuLu, the chip appears only as a spec
  ("XIAO SAMD21 inside"), and LuLu is always shown in its case.
- Say only what ships today: macOS and the XIAO SAMD21. Linux, Windows and other boards stay out.
- Keep the agent log in the `computer` tool-call format used by the site hero.
- After replacing the images, refresh the LinkedIn and X caches for liveuse.sitewebr.com.
