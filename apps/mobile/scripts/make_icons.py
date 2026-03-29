# -*- coding: utf-8 -*-
"""生成移动端 launcher 图标：青瓷渐变圆角方 + 白色笔记折角（与扩展图标同构，配色改为青瓷/湖青）"""

from PIL import Image, ImageDraw
import os

ANDROID = os.path.join(os.path.dirname(__file__), "..", "android", "app", "src", "main", "res")

C1 = (14, 122, 111)    # 深青瓷 #0E7A6F
C2 = (61, 156, 143)    # 湖青 #3D9C8F


def gradient(size):
    img = Image.new("RGB", (size, size))
    for y in range(size):
        for x in range(size):
            t = (x + y) / (2 * (size - 1)) if size > 1 else 0
            img.putpixel((x, y), tuple(int(a + (b - a) * t) for a, b in zip(C1, C2)))
    return img


def make(size):
    s = size * 8  # 超采样
    mask = Image.new("L", (s, s), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, s - 1, s - 1], radius=int(s * 0.24), fill=255)
    icon = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    icon.paste(gradient(s), (0, 0), mask)

    d = ImageDraw.Draw(icon)
    pad = s * 0.24
    card = [pad, pad, s - pad, s - pad]
    fold = s * 0.14
    d.polygon(
        [(card[0], card[1]), (card[2] - fold, card[1]), (card[2], card[1] + fold), (card[2], card[3]), (card[0], card[3])],
        fill=(255, 255, 255, 235),
    )
    d.polygon(
        [(card[2] - fold, card[1]), (card[2] - fold, card[1] + fold), (card[2], card[1] + fold)],
        fill=(219, 240, 236, 255),
    )
    line_color = (14, 122, 111, 255)
    lx0 = card[0] + s * 0.09
    lx1 = card[2] - s * 0.09
    ly = card[1] + s * 0.22
    for i in range(3):
        w = max(int(s * 0.035), 2)
        x_end = lx1 if i == 2 else lx1 - s * 0.1
        d.rounded_rectangle([lx0, ly + i * s * 0.15, x_end, ly + i * s * 0.15 + w], radius=w // 2, fill=line_color)

    return icon.resize((size, size), Image.LANCZOS)


# 各密度 ic_launcher.png
for dpi, size in [("mdpi", 48), ("hdpi", 72), ("xhdpi", 96), ("xxhdpi", 144), ("xxxhdpi", 192)]:
    out_dir = os.path.join(ANDROID, f"mipmap-{dpi}")
    os.makedirs(out_dir, exist_ok=True)
    make(size).save(os.path.join(out_dir, "ic_launcher.png"))
    print(f"mipmap-{dpi}/ic_launcher.png ({size})")

# 移除自适应图标 xml，回落到 PNG（保留视觉一致）
anydpi = os.path.join(ANDROID, "mipmap-anydpi-v26")
if os.path.isdir(anydpi):
    for f in os.listdir(anydpi):
        os.remove(os.path.join(anydpi, f))
    print("cleared mipmap-anydpi-v26")
