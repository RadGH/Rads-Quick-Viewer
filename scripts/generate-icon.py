from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter


ROOT = Path(__file__).resolve().parents[1]
PUBLIC = ROOT / "public"
ICONS = ROOT / "src-tauri" / "icons"


def draw_icon(size: int = 1024) -> Image.Image:
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    scale = size / 1024

    def p(points):
        return [(round(x * scale), round(y * scale)) for x, y in points]

    def b(box):
        return tuple(round(value * scale) for value in box)

    shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    shadow_draw = ImageDraw.Draw(shadow, "RGBA")
    shadow_draw.rounded_rectangle(b((134, 134, 890, 890)), radius=round(190 * scale), fill=(0, 0, 0, 86))
    shadow = shadow.filter(ImageFilter.GaussianBlur(round(34 * scale)))
    image.alpha_composite(shadow)

    draw = ImageDraw.Draw(image, "RGBA")
    draw.rounded_rectangle(b((118, 106, 886, 874)), radius=round(178 * scale), fill=(24, 35, 48, 255))
    draw.rounded_rectangle(
        b((118, 106, 886, 874)),
        radius=round(178 * scale),
        outline=(255, 248, 220, 90),
        width=round(14 * scale),
    )

    # Three large, simple forms remain readable at 16px and 32px.
    draw.polygon(p([(278, 304), (710, 214), (824, 478), (392, 568)]), fill=(52, 181, 174, 255))
    draw.polygon(p([(222, 584), (414, 292), (796, 706), (432, 802)]), fill=(255, 91, 76, 255))
    draw.polygon(p([(512, 252), (742, 512), (512, 772), (282, 512)]), fill=(255, 190, 67, 255))

    draw.polygon(p([(512, 382), (628, 512), (512, 642), (396, 512)]), fill=(255, 249, 218, 255))
    draw.line(p([(320, 320), (704, 240), (805, 472)]), fill=(255, 255, 255, 110), width=round(18 * scale))
    draw.line(p([(292, 598), (414, 800), (790, 704)]), fill=(255, 255, 255, 86), width=round(16 * scale))

    return image


def main() -> None:
    PUBLIC.mkdir(exist_ok=True)
    ICONS.mkdir(parents=True, exist_ok=True)

    icon = draw_icon()
    icon.save(PUBLIC / "icon.png")
    icon.resize((128, 128), Image.Resampling.LANCZOS).save(ICONS / "128x128.png")
    icon.resize((32, 32), Image.Resampling.LANCZOS).save(ICONS / "32x32.png")
    icon.save(ICONS / "icon.ico", sizes=[(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])


if __name__ == "__main__":
    main()
