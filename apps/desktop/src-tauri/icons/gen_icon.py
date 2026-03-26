"""
生成 byte-memos 任务栏图标（蓝色方块底 + 白色圆角笔记卡片 + 三条蓝色文字行）

输出：
  icon.png  — 512x512 高清 PNG
  icon.ico  — 多尺寸 ICO（16/24/32/48/64/128/256），供 Tauri 打包

设计：蓝色圆角方块底（#3B82F6）铺满画布，中间白色圆角笔记卡片，
卡片上三条蓝色横线（最后一行短，模拟文字行），与软件内 Logo.vue 的
「圆角笔记 + 三条文字行」语义一致。蓝底保证任何 Windows 任务栏背景下都醒目；
小尺寸下行距收窄、线条加粗，保持可辨。
"""
from PIL import Image, ImageDraw

PRIMARY = (59, 130, 246, 255)   # #3B82F6 蓝色底
WHITE = (255, 255, 255, 255)    # 白色笔记卡片


def make_logo(size: int) -> Image.Image:
    """
    生成指定尺寸的图标：蓝底圆角方块 + 白色圆角笔记 + 三条蓝色文字行。

    所有尺寸统一设计——蓝底保证小尺寸也清晰；16/24px 行距压缩为两行。
    """
    SS = 4  # 超采样倍数
    S = size * SS
    canvas = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(canvas)
    k = S / 64

    # 1. 蓝色圆角方块底，铺满画布（圆角随尺寸：大尺寸圆角大，小尺寸接近直角）
    # Windows 图标圆角惯例：约画布 18%~22% 的半径。小尺寸（16px）适当收小避免变形。
    if size <= 24:
        corner_r = 12 * k   # 小尺寸稍小圆角
    else:
        corner_r = 14 * k
    d.rounded_rectangle([0, 0, S, S], radius=corner_r, fill=PRIMARY)

    # 2. 白色圆角笔记卡片（居中，64 坐标系 12..52，与 Logo.vue viewBox 一致）
    card = [12 * k, 10 * k, 52 * k, 54 * k]
    card_r = 8 * k
    d.rounded_rectangle(card, radius=card_r, fill=WHITE)

    # 3. 三条蓝色文字行（露出底色 = 镂空感），最后一行短：
    #    与 Logo.vue mask 的 (21,20,w22) (21,30,w22) (21,40,w13) 对齐。
    #    小尺寸（≤24px）只画两行且加粗，避免糊成一团。
    line_h = 4 * k
    line_r = 2 * k
    if size <= 24:
        rows = [(21, 22, 22), (21, 36, 13)]          # (x, y, w)，两行、更粗
        line_h = 5.5 * k
        line_r = 2.5 * k
    else:
        rows = [(21, 20, 22), (21, 30, 22), (21, 40, 13)]
    for (x, y, w) in rows:
        d.rounded_rectangle(
            [x * k, y * k, (x + w) * k, y * k + line_h],
            radius=line_r,
            fill=PRIMARY,
        )

    return canvas.resize((size, size), Image.LANCZOS)


def main():
    import os
    out_dir = os.path.dirname(os.path.abspath(__file__))

    # 高清 PNG（512）
    big = make_logo(512)
    big.save(os.path.join(out_dir, "icon.png"))
    print("[OK] icon.png (512x512)")

    # ICO 多尺寸：传一张 256x256 高清图 + sizes 列表，Pillow 自动缩放嵌入各尺寸
    ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    ico_src = make_logo(256)
    ico_src.save(
        os.path.join(out_dir, "icon.ico"),
        format="ICO",
        sizes=ico_sizes,
    )
    print(f"[OK] icon.ico ({[s[0] for s in ico_sizes]})")


if __name__ == "__main__":
    main()
