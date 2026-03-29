/// 应用主题：亮/暗两套 + 液态玻璃所需的彩色渐变背景
///
/// 玻璃需要背后有内容/颜色才出效果：Scaffold 背景用柔和多色渐变，
/// 正文文字色对比度 ≥ 4.5:1（可读性红线）。
library;

import 'package:flutter/material.dart';

/// 靛→紫品牌色（与桌面端 / 扩展一致）
const kIndigo = Color(0xFF6366F1);
const kViolet = Color(0xFF8B5CF6);

/// 全屏背景渐变（玻璃的「底色舞台」）
List<Color> bgGradient(Brightness brightness) => brightness == Brightness.dark
    ? const [Color(0xFF17181C), Color(0xFF1B1630), Color(0xFF141824)]
    : const [Color(0xFFEEF0FB), Color(0xFFEDE7FB), Color(0xFFE5EDFA)];

class AppTheme {
  static ThemeData light() => _base(Brightness.light);

  static ThemeData dark() => _base(Brightness.dark);

  static ThemeData _base(Brightness brightness) {
    final dark = brightness == Brightness.dark;
    final scheme = ColorScheme.fromSeed(
      seedColor: kIndigo,
      brightness: brightness,
    );
    return ThemeData(
      useMaterial3: true,
      colorScheme: scheme,
      scaffoldBackgroundColor: Colors.transparent, // 背景由 shell 的渐变提供
      appBarTheme: AppBarTheme(
        backgroundColor: Colors.transparent,
        foregroundColor: dark ? Colors.white : Colors.black87,
        elevation: 0,
        centerTitle: false,
        titleTextStyle: TextStyle(
          fontSize: 22,
          fontWeight: FontWeight.w700,
          color: dark ? Colors.white : Colors.black87,
        ),
      ),
      textTheme: Typography.blackMountainView.copyWith(
        bodyLarge: const TextStyle(color: Colors.black87),
        bodyMedium: const TextStyle(color: Colors.black87),
        bodySmall: const TextStyle(color: Colors.black54),
      ).apply(
        bodyColor: dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328),
        displayColor: dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328),
      ),
      dividerColor: dark ? Colors.white10 : Colors.black.withValues(alpha: 0.06),
      snackBarTheme: const SnackBarThemeData(behavior: SnackBarBehavior.floating),
    );
  }
}
