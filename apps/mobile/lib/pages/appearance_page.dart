/// 外观设置页（我的 → 外观进入）：玻璃强度 / 主题模式
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';

class AppearancePage extends ConsumerWidget {
  const AppearancePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    final glassPref = ref.watch(glassPrefProvider);
    final themeMode = ref.watch(themeModeProvider);
    final pref = ref.watch(glassPrefProvider);

    Widget segCard({required String title, required String hint, required Widget child}) => GlassContainer(
          tier: GlassTier.flat,
          preference: pref,
          borderRadius: BorderRadius.circular(18),
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: TextStyle(fontSize: 13.5, fontWeight: FontWeight.w700, color: fg)),
              if (hint.isNotEmpty) ...[
                const SizedBox(height: 4),
                Text(hint, style: TextStyle(fontSize: 11, color: fg.withValues(alpha: 0.5))),
              ],
              const SizedBox(height: 12),
              child,
            ],
          ),
        );

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Container(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: bgGradient(Theme.of(context).brightness),
          ),
        ),
        child: SafeArea(
          bottom: false,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              GlassContainer(
                tier: GlassTier.regular,
                preference: pref,
                padding: EdgeInsets.zero,
                child: Row(
                  children: [
                    IconButton(
                      icon: const Icon(Icons.arrow_back_rounded),
                      onPressed: () => Navigator.pop(context),
                    ),
                    Text('外观', style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600, color: fg)),
                  ],
                ),
              ),
              Expanded(
                child: ListView(
                  padding: const EdgeInsets.fromLTRB(16, 14, 16, 40),
                  children: [
                    segCard(
                      title: '玻璃强度',
                      hint: '省电：关闭所有实时模糊，保留透明度层级',
                      child: SegmentedButton<GlassPreference>(
                        segments: [
                          for (final p in GlassPreference.values)
                            ButtonSegment(value: p, label: Text(p.label, style: const TextStyle(fontSize: 12))),
                        ],
                        selected: {glassPref},
                        onSelectionChanged: (s) => ref.read(glassPrefProvider.notifier).set(s.first),
                        showSelectedIcon: false,
                        style: ButtonStyle(
                          visualDensity: VisualDensity.compact,
                          backgroundColor: WidgetStateProperty.resolveWith((states) =>
                              states.contains(WidgetState.selected) ? kPrimary.withValues(alpha: 0.85) : null),
                          foregroundColor: WidgetStateProperty.resolveWith((states) =>
                              states.contains(WidgetState.selected) ? Colors.white : fg.withValues(alpha: 0.6)),
                        ),
                      ),
                    ),
                    const SizedBox(height: 14),
                    segCard(
                      title: '主题',
                      hint: '',
                      child: SegmentedButton<ThemeMode>(
                        segments: const [
                          ButtonSegment(value: ThemeMode.system, label: Text('跟随系统', style: TextStyle(fontSize: 12))),
                          ButtonSegment(value: ThemeMode.light, label: Text('浅色', style: TextStyle(fontSize: 12))),
                          ButtonSegment(value: ThemeMode.dark, label: Text('深色', style: TextStyle(fontSize: 12))),
                        ],
                        selected: {themeMode},
                        onSelectionChanged: (s) => ref.read(themeModeProvider.notifier).set(s.first),
                        showSelectedIcon: false,
                        style: ButtonStyle(
                          visualDensity: VisualDensity.compact,
                          backgroundColor: WidgetStateProperty.resolveWith((states) =>
                              states.contains(WidgetState.selected) ? kPrimary.withValues(alpha: 0.85) : null),
                          foregroundColor: WidgetStateProperty.resolveWith((states) =>
                              states.contains(WidgetState.selected) ? Colors.white : fg.withValues(alpha: 0.6)),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
