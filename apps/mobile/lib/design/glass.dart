/// 液态玻璃设计系统（REQUIREMENTS.md §5）
///
/// 玻璃是层级语言：ultraThin / regular / thick 三档材质令牌，
/// 渐变描边模拟边缘折射，顶部 specular 模拟光源在上。
///
/// 性能硬约束：
/// - 每屏 BackdropFilter ≤ 2（底栏 + 一个浮层），列表卡片一律 [GlassTier.flat]
/// - 用户可在「我的 → 外观」切换玻璃强度（auto / full / off），off 时全部退化为纯半透明
library;

import 'dart:ui' as ui;

import 'package:flutter/material.dart';

/// 玻璃材质档位
enum GlassTier {
  /// blur 8px · 白 8% —— 静态浮层（分组头、卡片容器）
  ultraThin,

  /// blur 20px · 白 12% —— 底部导航、FAB
  regular,

  /// blur 32px · 白 18% —— 模态弹层
  thick,

  /// 无 blur 纯半透明 —— 列表卡片 / 降级兜底（视觉语言靠描边保留）
  flat,
}

/// 单档玻璃参数（亮色基；暗色由 [GlassContainer] 按 Brightness 映射）
class GlassSpec {
  const GlassSpec({
    required this.blur,
    required this.lightAlpha,
    required this.darkAlpha,
    required this.saturation,
  });

  final double blur;

  /// 亮色模式：白色叠加不透明度
  final double lightAlpha;

  /// 暗色模式：黑色叠加不透明度
  final double darkAlpha;

  /// 颜色饱和度提升（玻璃透色的关键）
  final double saturation;

  static const GlassSpec ultraThin = GlassSpec(blur: 8, lightAlpha: 0.08, darkAlpha: 0.06, saturation: 1.2);
  static const GlassSpec regular = GlassSpec(blur: 20, lightAlpha: 0.12, darkAlpha: 0.10, saturation: 1.4);
  static const GlassSpec thick = GlassSpec(blur: 32, lightAlpha: 0.18, darkAlpha: 0.16, saturation: 1.6);
  static const GlassSpec flat = GlassSpec(blur: 0, lightAlpha: 0.06, darkAlpha: 0.05, saturation: 1.0);
}

/// 玻璃强度偏好（我的 → 外观）
enum GlassPreference {
  /// 跟随性能自动（默认全效果）
  auto,

  /// 强制全效果
  full,

  /// 省电：全部退化为 flat
  off;

  String get label => switch (this) {
        auto => '自动',
        full => '全开',
        off => '省电',
      };

  static GlassPreference fromName(String? s) => GlassPreference.values.firstWhere(
        (v) => v.name == s,
        orElse: () => GlassPreference.auto,
      );
}

/// 玻璃容器：圆角 + 背景模糊 + 透色叠加 + 渐变描边 + 顶部 specular
///
/// 用法：
/// ```dart
/// GlassContainer(
///   tier: GlassTier.regular,
///   borderRadius: BorderRadius.all(Radius.circular(20)),
///   child: ...,
/// )
/// ```
class GlassContainer extends StatelessWidget {
  const GlassContainer({
    super.key,
    required this.child,
    this.tier = GlassTier.regular,
    this.borderRadius = const BorderRadius.all(Radius.circular(20)),
    this.preference = GlassPreference.auto,
    this.padding,
    this.margin,
    this.showSpecular = true,
  });

  final Widget child;

  final GlassTier tier;

  final BorderRadius borderRadius;

  /// 用户偏好（默认 auto；off 时 blur 全部退化为 flat）
  final GlassPreference preference;

  final EdgeInsetsGeometry? padding;

  final EdgeInsetsGeometry? margin;

  /// 是否绘制顶部高光（嵌套玻璃可关掉内层，避免高光叠加）
  final bool showSpecular;

  GlassSpec get _baseSpec => switch (tier) {
        GlassTier.ultraThin => GlassSpec.ultraThin,
        GlassTier.regular => GlassSpec.regular,
        GlassTier.thick => GlassSpec.thick,
        GlassTier.flat => GlassSpec.flat,
      };

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final effective = preference == GlassPreference.off ? GlassSpec.flat : _baseSpec;
    final alpha = dark ? effective.darkAlpha : effective.lightAlpha;
    final overlay = dark ? Colors.black : Colors.white;

    return Container(
      margin: margin,
      // 渐变描边（边缘折射）：CustomPaint 画在 ClipRRect 外圈
      foregroundDecoration: _GradientBorderDecoration(
        radius: borderRadius,
        dark: dark,
      ),
      child: ClipRRect(
        borderRadius: borderRadius,
        child: Stack(
          children: [
            // 背景：模糊 + 饱和度增强 + 透色叠加
            if (effective.blur > 0)
              BackdropFilter(
                filter: ui.ImageFilter.compose(
                  outer: ui.ImageFilter.blur(sigmaX: effective.blur, sigmaY: effective.blur),
                  inner: _saturationMatrix(effective.saturation),
                ),
                child: _fill(alpha, overlay),
              )
            else
              _fill(alpha, overlay),

            // 顶部 specular：光源在上
            if (showSpecular)
              Positioned(
                top: 0,
                left: 0,
                right: 0,
                child: IgnorePointer(
                  child: Container(
                    height: 1.5,
                    decoration: BoxDecoration(
                      gradient: LinearGradient(
                        begin: Alignment.topCenter,
                        end: Alignment.bottomCenter,
                        colors: [
                          (dark ? Colors.white : Colors.white).withValues(alpha: dark ? 0.18 : 0.35),
                          Colors.transparent,
                        ],
                      ),
                      borderRadius: BorderRadius.vertical(top: Radius.circular(borderRadius.topLeft.x)),
                    ),
                  ),
                ),
              ),

            Padding(
              padding: padding ?? EdgeInsets.zero,
              child: child,
            ),
          ],
        ),
      ),
    );
  }

  Widget _fill(double alpha, Color overlay) => Container(color: overlay.withValues(alpha: alpha));
}

/// 饱和度矩阵（sat=1 原样；>1 增强；标准 ITU-R BT.709 亮度系数）
ui.ColorFilter _saturationMatrix(double sat) {
  return ui.ColorFilter.matrix([
    0.213 + 0.787 * sat, 0.715 - 0.715 * sat, 0.072 - 0.072 * sat, 0, 0,
    0.213 - 0.213 * sat, 0.715 + 0.285 * sat, 0.072 - 0.072 * sat, 0, 0,
    0.213 - 0.213 * sat, 0.715 - 0.715 * sat, 0.072 + 0.928 * sat, 0, 0,
    0, 0, 0, 1, 0,
  ]);
}

/// 渐变描边装饰（上亮下暗，模拟光线在玻璃边缘的弯折）
class _GradientBorderDecoration extends Decoration {
  const _GradientBorderDecoration({required this.radius, required this.dark});

  final BorderRadius radius;
  final bool dark;

  @override
  BoxPainter createBoxPainter([VoidCallback? onChanged]) {
    return _GradientBorderPainter(radius: radius, dark: dark);
  }
}

class _GradientBorderPainter extends BoxPainter {
  _GradientBorderPainter({required this.radius, required this.dark});

  final BorderRadius radius;
  final bool dark;

  @override
  void paint(Canvas canvas, Offset offset, ImageConfiguration configuration) {
    final rect = offset & (configuration.size ?? Size.zero);
    final rrect = radius.toRRect(Rect.fromLTWH(rect.left + 0.5, rect.top + 0.5, rect.width - 1, rect.height - 1));

    final shader = LinearGradient(
      begin: Alignment.topCenter,
      end: Alignment.bottomCenter,
      colors: dark
          ? [Colors.white.withValues(alpha: 0.28), Colors.white.withValues(alpha: 0.05)]
          : [Colors.white.withValues(alpha: 0.65), Colors.white.withValues(alpha: 0.15)],
    ).createShader(rect);

    final paint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1
      ..shader = shader;

    canvas.drawRRect(rrect, paint);
  }
}
