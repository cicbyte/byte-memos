# -*- coding: utf-8 -*-
"""生成扩展图标：圆角方形 + 靛→紫渐变 + 白色笔记折角图形（16/48/128）"""

from PIL import Image, ImageDraw
import os

OUT = os.path.join(os.path.dirname(__file__), "..", "icons")
os.makedirs(OUT, exist_ok=True)

C1 = (99, 102, 241)   # indigo #6366f1
C2 = (139, 92, 246)   # violet #8b5cf6


def gradient(size):
    """对角线性渐变图"""
    img = Image.new("RGB", (size, size))
    for y in range(size):
        for x in range(size):
            t = (x + y) / (2 * (size - 1)) if size > 1 else 0
            img.putpixel((x, y), tuple(int(a + (b - a) * t) for a, b in zip(C1, C2)))
    return img


def rounded_mask(size, radius):
    m = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(m)
    d.rounded_rectangle([0, 0, size - 1, size - 1], radius=radius, fill=255)
    return m


def make(size):
    s = size * 8  # 超采样抗锯齿
    base = gradient(s)
    mask = rounded_mask(s, int(s * 0.22))
    icon = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    icon.paste(base, (0, 0), mask)

    d = ImageDraw.Draw(icon)
    # 白色笔记卡片（带右下折角）
    pad = s * 0.24
    card = [pad, pad, s - pad, s - pad]
    fold = s * 0.14
    card_poly = [
        (card[0], card[1]), (card[2] - fold, card[1]),
        (card[2], card[1] + fold), (card[2], card[3]),
        (card[0], card[3]),
    ]
    d.polygon(card_poly, fill=(255, 255, 255, 235))
    # 折角三角
    d.polygon(
        [(card[2] - fold, card[1]), (card[2] - fold, card[1] + fold), (card[2], card[1] + fold)],
        fill=(224, 231, 255, 255),
    )
    # 三条内容线
    line_color = (99, 102, 241, 255)
    lx0 = card[0] + s * 0.09
    lx1 = card[2] - s * 0.09
    ly = card[1] + s * 0.22
    for i in range(3):
        w = max(int(s * 0.035), 2)
        x_end = lx1 if i == 2 else lx1 - s * 0.1
        d.rounded_rectangle([lx0, ly + i * s * 0.15, x_end, ly + i * s * 0.15 + w], radius=w // 2, fill=line_color)

    icon = icon.resize((size, size), Image.LANCZOS)
    icon.save(os.path.join(OUT, f"icon{size}.png"))
    print(f"icon{size}.png ok")


for sz in (16, 48, 128):
    make(sz)
