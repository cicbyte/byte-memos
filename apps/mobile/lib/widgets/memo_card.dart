/// 备忘录卡片（flat 伪玻璃：无 blur，性能红线内）
library;

import 'package:flutter/material.dart';

import '../conn/models.dart';
import '../design/glass.dart';
import '../design/theme.dart';

class MemoCard extends StatelessWidget {
  const MemoCard({super.key, required this.memo, required this.onTap, this.onLongPress});

  final Memo memo;
  final VoidCallback onTap;

  /// 长按 → 快捷菜单（置顶/可见性，F11）
  final VoidCallback? onLongPress;

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final text = Theme.of(context).textTheme;

    return GestureDetector(
      onTap: onTap,
      onLongPress: onLongPress,
      child: GlassContainer(
        tier: GlassTier.flat,
        preference: GlassPreference.auto, // flat 不受省电档影响（本来就是 0 blur）
        borderRadius: BorderRadius.circular(16),
        margin: const EdgeInsets.only(bottom: 10),
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // 标题行：置顶标记 + 标题 + 分类
            Row(
              children: [
                if (memo.pinned) ...[
                  const Icon(Icons.push_pin_rounded, size: 13, color: Color(0xFFF59E0B)),
                  const SizedBox(width: 5),
                ],
                Expanded(
                  child: Text(
                    memo.title.isEmpty ? _firstLine(memo.content) : memo.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: (dark ? text.titleMedium : text.titleMedium)?.copyWith(
                      fontWeight: FontWeight.w600,
                      fontSize: 14.5,
                    ),
                  ),
                ),
                if (memo.category != null && memo.category!.isNotEmpty) ...[
                  const SizedBox(width: 6),
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
                    decoration: BoxDecoration(
                      color: kPrimary.withValues(alpha: 0.12),
                      borderRadius: BorderRadius.circular(6),
                    ),
                    child: Text(
                      memo.category!,
                      style: const TextStyle(fontSize: 10, color: kPrimary),
                    ),
                  ),
                ],
              ],
            ),
            const SizedBox(height: 6),
            // 正文摘要（Markdown 源文截 6 行）
            Text(
              memo.content,
              maxLines: 6,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: 12.5,
                height: 1.55,
                color: dark ? Colors.white70 : Colors.black.withValues(alpha: 0.65),
              ),
            ),
            // 标签 + 时间
            if (memo.tags.isNotEmpty || memo.createdAt.isNotEmpty) ...[
              const SizedBox(height: 9),
              Row(
                children: [
                  Expanded(
                    child: Wrap(
                      spacing: 6,
                      runSpacing: 4,
                      children: [
                        for (final t in memo.tags.take(4))
                          Text(
                            '#$t',
                            style: TextStyle(fontSize: 10.5, color: kAccent.withValues(alpha: 0.9)),
                          ),
                      ],
                    ),
                  ),
                  Text(
                    _shortTime(memo.createdAt),
                    style: TextStyle(fontSize: 10, color: Theme.of(context).hintColor),
                  ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }

  String _firstLine(String content) {
    final line = content.trim().split('\n').first;
    return line.isEmpty ? '（无标题）' : line.length > 40 ? line.substring(0, 40) : line;
  }

  String _shortTime(String iso) {
    final dt = DateTime.tryParse(iso);
    if (dt == null) return '';
    final diff = DateTime.now().difference(dt);
    if (diff.inMinutes < 1) return '刚刚';
    if (diff.inHours < 1) return '${diff.inMinutes} 分钟前';
    if (diff.inDays == 0) return '${diff.inHours} 小时前';
    if (diff.inDays < 7) return '${diff.inDays} 天前';
    return '${dt.month}/${dt.day}';
  }
}
