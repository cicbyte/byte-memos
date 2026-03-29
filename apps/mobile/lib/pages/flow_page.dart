/// 流页：备忘录时间线（置顶组在前 + 按天分组）· 无限滚动 · 下拉刷新
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../conn/models.dart';
import '../state/memos_state.dart';
import '../widgets/memo_card.dart';
import 'detail_page.dart';

class FlowPage extends ConsumerWidget {
  const FlowPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final memos = ref.watch(memosProvider);

    if (memos.loading && memos.items.isEmpty) {
      return const Center(child: CircularProgressIndicator(strokeWidth: 2));
    }

    if (memos.items.isEmpty) {
      return _Empty(onRefresh: () => ref.read(memosProvider.notifier).refresh());
    }

    final pinned = memos.items.where((m) => m.pinned).toList();
    final rest = memos.items.where((m) => !m.pinned).toList();

    // 按天分组（rest 按创建时间倒序，由 provider 保证）
    final groups = <String, List<Memo>>{};
    for (final m in rest) {
      final dt = m.created;
      final key = dt == null ? '更早' : _dayLabel(DateTime.now().difference(dt));
      (groups[key] ??= []).add(m);
    }

    return RefreshIndicator(
      onRefresh: () => ref.read(memosProvider.notifier).refresh(),
      displacement: 20,
      child: ListView.builder(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 190),
        itemCount: _count(pinned, groups, memos),
        itemBuilder: (context, i) {
          var idx = i;
          // 置顶组
          if (pinned.isNotEmpty) {
            if (idx == 0) return const _GroupHeader('置顶', icon: Icons.push_pin_rounded);
            idx--;
            if (idx < pinned.length) {
              return MemoCard(
                memo: pinned[idx],
                onTap: () => _openDetail(context, ref, pinned[idx]),
              );
            }
            idx -= pinned.length;
          }
          // 按天分组
          for (final entry in groups.entries) {
            if (idx == 0) return _GroupHeader(entry.key);
            idx--;
            if (idx < entry.value.length) {
              final m = entry.value[idx];
              return MemoCard(
                memo: m,
                onTap: () => _openDetail(context, ref, m),
              );
            }
            idx -= entry.value.length;
          }
          // 尾部：加载更多 / 结尾
          if (memos.loadingMore) {
            return const Padding(
              padding: EdgeInsets.symmetric(vertical: 20),
              child: Center(child: SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))),
            );
          }
          return Padding(
            padding: const EdgeInsets.symmetric(vertical: 16),
            child: Center(
              child: Text(
                memos.hasMore ? '上拉加载更多' : '· 共 ${memos.items.length} 条 ·',
                style: TextStyle(fontSize: 11, color: Theme.of(context).hintColor),
              ),
            ),
          );
        },
      ),
    );
  }

  void _openDetail(BuildContext context, WidgetRef ref, Memo m) {
    Navigator.of(context).push(MaterialPageRoute(builder: (_) => DetailPage(memoId: m.shortId)));
  }

  int _count(List<Memo> pinned, Map<String, List<Memo>> groups, MemosState memos) {
    var n = 1 + (memos.hasMore || memos.loadingMore ? 1 : 1); // 尾部元素
    if (pinned.isNotEmpty) n += 1 + pinned.length;
    n += groups.entries.fold<int>(0, (acc, e) => acc + 1 + e.value.length);
    return n;
  }

  String _dayLabel(Duration diff) {
    if (diff.inDays == 0) return '今天';
    if (diff.inDays == 1) return '昨天';
    if (diff.inDays < 7) return '${diff.inDays} 天前';
    return DateFormat('M 月 d 日', 'zh').format(DateTime.now().subtract(diff));
  }
}

class _GroupHeader extends StatelessWidget {
  const _GroupHeader(this.label, {this.icon});

  final String label;
  final IconData? icon;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(4, 18, 0, 8),
      child: Row(
        children: [
          if (icon != null) ...[
            Icon(icon, size: 14, color: Theme.of(context).hintColor),
            const SizedBox(width: 5),
          ],
          Text(
            label,
            style: TextStyle(
              fontSize: 12.5,
              fontWeight: FontWeight.w600,
              color: Theme.of(context).hintColor,
            ),
          ),
        ],
      ),
    );
  }
}

class _Empty extends StatelessWidget {
  const _Empty({required this.onRefresh});

  final Future<void> Function() onRefresh;

  @override
  Widget build(BuildContext context) {
    return ListView(
      physics: const AlwaysScrollableScrollPhysics(),
      children: [
        const SizedBox(height: 120),
        Icon(Icons.note_add_outlined, size: 52, color: Theme.of(context).hintColor),
        const SizedBox(height: 12),
        Center(
          child: Text(
            '还没有备忘录\n点右下角 + 记第一条',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 13, height: 1.7, color: Theme.of(context).hintColor),
          ),
        ),
      ],
    );
  }
}
