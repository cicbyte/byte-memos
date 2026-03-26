"""
生成 byte-link 任务栏图标（蓝色方块底 + 白色书签旗帜 + 顶部圆孔）

输出：
  icon.png  — 512x512 高清 PNG
  icon.ico  — 多尺寸 ICO（16/24/32/48/64/128/256），供 Tauri 打包

设计：蓝色圆角方块底（#3B82F6）铺满画布，上面放白色书签旗帜，
顶部一个蓝色圆孔（露出底色），与软件内 Logo.vue 的「书签 + 圆孔」语义一致。
蓝底保证任何 Windows 任务栏背景下都醒目；书签轮廓在小尺寸也清晰。
"""
from PIL import Image, ImageDraw

PRIMARY = (59, 130, 246, 255)   # #3B82F6 蓝色底
WHITE = (255, 255, 255, 255)    # 白色书签


def make_logo(size: int) -> Image.Image:
    """
    生成指定尺寸的图标：蓝底圆角方块 + 白色书签（带圆孔）。

    所有尺寸统一设计——蓝底保证了小尺寸也清晰，不需要简化版。
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

    # 2. 白色书签旗帜（64 坐标系多边形 → 缩放到画布；底部 V 形凹槽收于 (32,44)）
    bookmark = [
        (16 * k, 10 * k),
        (48 * k, 10 * k),
        (48 * k, 54 * k),
        (32 * k, 44 * k),
        (16 * k, 54 * k),
    ]
    d.polygon(bookmark, fill=WHITE)

    # 3. 顶部圆孔：蓝色圆盖在书签上（= 露出底色，形成孔），呼应 Logo.vue 的镂空圆孔
    hole_r = 4.2 * k
    d.ellipse(
        [(32 * k - hole_r), (20 * k - hole_r), (32 * k + hole_r), (20 * k + hole_r)],
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
