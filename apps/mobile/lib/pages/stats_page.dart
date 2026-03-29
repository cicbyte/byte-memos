/// 统计页：玻璃卡片拼贴（总量/标签/置顶/回收站）+ 标签云
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';
import '../state/memos_state.dart';

class StatsPage extends ConsumerWidget {
  const StatsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    final stats = ref.watch(statsProvider);
    final tags = ref.watch(tagsProvider).valueOrNull ?? [];
    final online = ref.watch(connProvider).isOnline;

    if (!online) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.wifi_off_rounded, size: 40, color: Theme.of(context).hintColor),
            const SizedBox(height: 10),
            Text('离线状态暂无统计', style: TextStyle(fontSize: 13, color: Theme.of(context).hintColor)),
          ],
        ),
      );
    }

    return stats.when(
      loading: () => const Center(child: CircularProgressIndicator(strokeWidth: 2)),
      error: (e, _) => Center(child: Text('加载失败', style: TextStyle(fontSize: 13, color: Theme.of(context).hintColor))),
      data: (s) => ListView(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 190),
        children: [
          // 数字拼贴 2×2
          GridView.count(
            crossAxisCount: 2,
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            mainAxisSpacing: 10,
            crossAxisSpacing: 10,
            childAspectRatio: 1.55,
            children: [
              _StatCard(icon: Icons.note_rounded, color: kIndigo, label: '备忘录', value: '${s.memoCount}', fg: fg),
              _StatCard(icon: Icons.tag_rounded, color: kViolet, label: '标签', value: '${s.tagCount}', fg: fg),
              _StatCard(icon: Icons.push_pin_rounded, color: const Color(0xFFF59E0B), label: '置顶', value: '${s.pinnedCount}', fg: fg),
              _StatCard(icon: Icons.delete_rounded, color: const Color(0xFFFF4444), label: '回收站', value: '${s.trashedCount}', fg: fg),
            ],
          ),
          const SizedBox(height: 20),

          // 标签云
          if (tags.isNotEmpty) ...[
            Padding(
              padding: const EdgeInsets.only(left: 4, bottom: 10),
              child: Text('标签云', style: TextStyle(fontSize: 12.5, fontWeight: FontWeight.w600, color: Theme.of(context).hintColor)),
            ),
            GlassContainer(
              tier: GlassTier.flat,
              preference: ref.watch(glassPrefProvider),
              borderRadius: BorderRadius.circular(16),
              padding: const EdgeInsets.all(14),
              child: Wrap(
                spacing: 8,
                runSpacing: 10,
                children: [
                  for (final t in tags)
                    Container(
                      padding: const EdgeInsets.symmetric(horizontal: 11, vertical: 5),
                      decoration: BoxDecoration(
                        color: kViolet.withValues(alpha: 0.10 + (t.count / 30).clamp(0, 0.18)),
                        borderRadius: BorderRadius.circular(999),
                      ),
                      child: Text(
                        '#${t.tag} ${t.count}',
                        style: TextStyle(
                          fontSize: 11.5 + (t.count / 10).clamp(0, 3),
                          color: fg.withValues(alpha: 0.85),
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _StatCard extends StatelessWidget {
  const _StatCard({required this.icon, required this.color, required this.label, required this.value, required this.fg});

  final IconData icon;
  final Color color;
  final String label;
  final String value;
  final Color fg;

  @override
  Widget build(BuildContext context) {
    return GlassContainer(
      tier: GlassTier.flat,
      preference: ref0(context),
      borderRadius: BorderRadius.circular(16),
      padding: const EdgeInsets.all(13),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Row(
            children: [
              Icon(icon, size: 15, color: color),
              const SizedBox(width: 5),
              Text(label, style: TextStyle(fontSize: 11.5, color: fg.withValues(alpha: 0.6))),
            ],
          ),
          const SizedBox(height: 5),
          Text(
            value,
            style: TextStyle(fontSize: 25, fontWeight: FontWeight.w800, color: fg, height: 1.1),
          ),
        ],
      ),
    );
  }

  // flat 卡片无需偏好读取（0 blur），直接静态返回，避免无谓 rebuild
  static GlassPreference ref0(BuildContext context) => GlassPreference.auto;
}
