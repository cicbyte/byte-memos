/// 应用主题：亮/暗两套 + 液态玻璃所需的彩色渐变背景
///
/// 色彩方向：青瓷 / 宣纸（纸墨文具气质，避开 AI 产品惯用的靛紫渐变）。
/// 玻璃需要背后有内容/颜色才出效果：Scaffold 背景用柔和多色渐变，
/// 正文文字色对比度 ≥ 4.5:1（可读性红线）。
library;

import 'package:flutter/material.dart';

/// 主色·深青瓷（按钮/激活态/链接）
const kPrimary = Color(0xFF0E7A6F);

/// 强调色·湖青（渐变收尾/标签）
const kAccent = Color(0xFF3D9C8F);

/// 全屏背景渐变（玻璃的「底色舞台」）：亮=暖纸+淡青，暗=石墨墨绿
List<Color> bgGradient(Brightness brightness) => brightness == Brightness.dark
    ? const [Color(0xFF191C1B), Color(0xFF17201E), Color(0xFF131719)]
    : const [Color(0xFFF8F6F1), Color(0xFFEEF2EE), Color(0xFFF1ECE2)];

class AppTheme {
  static ThemeData light() => _base(Brightness.light);

  static ThemeData dark() => _base(Brightness.dark);

  static ThemeData _base(Brightness brightness) {
    final dark = brightness == Brightness.dark;
    final scheme = ColorScheme.fromSeed(
      seedColor: kPrimary,
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
        bodyColor: dark ? const Color(0xFFE6E8EA) : const Color(0xFF1F2328),
        displayColor: dark ? const Color(0xFFE6E8EA) : const Color(0xFF1F2328),
      ),
      dividerColor: dark ? Colors.white10 : Colors.black.withValues(alpha: 0.06),
      snackBarTheme: const SnackBarThemeData(behavior: SnackBarBehavior.floating),
    );
  }
}
