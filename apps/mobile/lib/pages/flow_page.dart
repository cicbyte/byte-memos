/// 流页：备忘录时间线（置顶组在前 + 按天分组）· 无限滚动 · 下拉刷新
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../conn/models.dart';
import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';
import '../state/memos_state.dart';
import '../util/checklist.dart';
import '../widgets/memo_card.dart';
import 'detail_page.dart';

class FlowPage extends ConsumerWidget {
  const FlowPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final memos = ref.watch(memosProvider);
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);

    return Column(
      children: [
        // 页头：标题 + 今日日期
        Padding(
          padding: const EdgeInsets.fromLTRB(20, 14, 20, 4),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: [
              Text('备忘流', style: TextStyle(fontSize: 22, fontWeight: FontWeight.w800, color: fg)),
              const SizedBox(width: 10),
              Text(
                DateFormat('M月d日 · ', 'zh').format(DateTime.now()) +
                    (memos.items.isEmpty ? '' : '${memos.items.length} 条'),
                style: TextStyle(fontSize: 11.5, color: fg.withValues(alpha: 0.45)),
              ),
            ],
          ),
        ),

        // 分类筛选条（有分类才显示）
        const _CategoryBar(),

        Expanded(
          child: memos.loading && memos.items.isEmpty
              ? const Center(child: CircularProgressIndicator(strokeWidth: 2))
              : memos.items.isEmpty
                  ? const _Empty()
                  : _FlowList(memos: memos),
        ),
      ],
    );
  }
}

/// 分类筛选条：待办 + 全部 + 各分类（横向滚动胶囊；待办与分类互斥）
class _CategoryBar extends ConsumerWidget {
  const _CategoryBar();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cats = ref.watch(categoriesProvider).valueOrNull ?? [];
    final active = ref.watch(categoryFilterProvider);
    final todoOn = ref.watch(todoFilterProvider);
    final n = cats.length + 2; // 待办 + 全部 + 分类
    return SizedBox(
      height: 36,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 16),
        itemCount: n,
        separatorBuilder: (_, _) => const SizedBox(width: 8),
        itemBuilder: (_, i) {
          if (i == 0) {
            return _Chip(
              label: '待办',
              active: todoOn,
              icon: Icons.check_box_outline_blank_rounded,
              onTap: () {
                final next = !todoOn;
                ref.read(todoFilterProvider.notifier).state = next;
                if (next) ref.read(categoryFilterProvider.notifier).state = null;
              },
            );
          }
          final label = i == 1 ? null : cats[i - 2].name;
          final on = !todoOn && ((i == 1 && active == null) || (label != null && active == label));
          return _Chip(
            label: label ?? '全部',
            active: on,
            onTap: () {
              ref.read(categoryFilterProvider.notifier).state = label;
              if (label != null) ref.read(todoFilterProvider.notifier).state = false;
            },
          );
        },
      ),
    );
  }
}

class _Chip extends ConsumerWidget {
  const _Chip({required this.label, required this.active, required this.onTap, this.icon});

  final String label;
  final bool active;
  final VoidCallback onTap;
  final IconData? icon;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final activeColor = dark ? Colors.white : kPrimary;
    return GestureDetector(
      onTap: onTap,
      child: GlassContainer(
        tier: active ? GlassTier.regular : GlassTier.flat,
        preference: ref.watch(glassPrefProvider),
        borderRadius: BorderRadius.circular(999),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (icon != null) ...[
              Icon(icon, size: 13, color: active ? activeColor : (dark ? Colors.white60 : Colors.black54)),
              const SizedBox(width: 4),
            ],
            Text(
              label,
              style: TextStyle(
                fontSize: 12.5,
                fontWeight: active ? FontWeight.w600 : FontWeight.w400,
                color: active ? activeColor : (dark ? Colors.white60 : Colors.black54),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _FlowList extends ConsumerWidget {
  const _FlowList({required this.memos});

  final MemosState memos;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // 分类筛选：null=全部（筛选时隐藏置顶区，避免跨分类混淆）；待办筛选优先于分类
    final filter = ref.watch(categoryFilterProvider);
    final todo = ref.watch(todoFilterProvider);
    final List<Memo> visible;
    if (todo) {
      visible = memos.items.where((m) => countUnchecked(m.content) > 0).toList();
    } else if (filter == null) {
      visible = memos.items;
    } else {
      visible = memos.items.where((m) => (m.category ?? '') == filter).toList();
    }
    final pinned = !todo && filter == null ? visible.where((m) => m.pinned).toList() : <Memo>[];
    final rest = !todo && filter == null ? visible.where((m) => !m.pinned).toList() : visible;

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
      color: kPrimary,
      child: ListView.builder(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 190),
        itemCount: _count(pinned, groups, memos),
        itemBuilder: (context, i) {
          var idx = i;
          // 置顶组
          if (pinned.isNotEmpty) {
            if (idx == 0) return const _GroupHeader('置顶', icon: Icons.push_pin_rounded);
            idx--;
            if (idx < pinned.length) {
              return _FadeSlideIn(
                keyId: 'p${pinned[idx].shortId}',
                index: idx,
                child: MemoCard(
                  memo: pinned[idx],
                  onTap: () => _openDetail(context, ref, pinned[idx]),
                  onLongPress: () => _quickActions(context, ref, pinned[idx]),
                ),
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
              return _FadeSlideIn(
                keyId: m.shortId,
                index: idx,
                child: MemoCard(
                  memo: m,
                  onTap: () => _openDetail(context, ref, m),
                  onLongPress: () => _quickActions(context, ref, m),
                ),
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
                memos.hasMore ? '上拉加载更多' : '· 共 ${visible.length} 条 ·',
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

  /// F11：长按快捷菜单（置顶 / 公开-私有）
  void _quickActions(BuildContext context, WidgetRef ref, Memo m) {
    final client = ref.read(connProvider).client;
    final dark = Theme.of(context).brightness == Brightness.dark;
    final isPublic = m.visibility == 'PUBLIC';
    showModalBottomSheet(
      context: context,
      backgroundColor: Colors.transparent,
      builder: (ctx) => GlassContainer(
        tier: GlassTier.thick,
        preference: ref.read(glassPrefProvider),
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
        padding: const EdgeInsets.fromLTRB(8, 6, 8, 14),
        child: SafeArea(
          top: false,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Padding(
                padding: const EdgeInsets.only(left: 12, top: 8, bottom: 4),
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: Text(
                    m.title.isEmpty ? '快捷操作' : m.title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(fontSize: 12, color: (dark ? Colors.white : Colors.black87).withValues(alpha: 0.55)),
                  ),
                ),
              ),
              ListTile(
                leading: Icon(m.pinned ? Icons.push_pin_rounded : Icons.push_pin_outlined, size: 20),
                title: Text(m.pinned ? '取消置顶' : '置顶', style: const TextStyle(fontSize: 14)),
                onTap: () async {
                  Navigator.pop(ctx);
                  try {
                    final updated = await client!.togglePinned(m.shortId);
                    ref.read(memosProvider.notifier).upsert(updated);
                  } catch (_) {}
                },
              ),
              ListTile(
                leading: Icon(isPublic ? Icons.lock_outline_rounded : Icons.public_rounded, size: 20),
                title: Text(isPublic ? '设为私有' : '设为公开', style: const TextStyle(fontSize: 14)),
                subtitle: Text(isPublic ? '仅自己可见' : '公开分享', style: const TextStyle(fontSize: 11)),
                onTap: () async {
                  Navigator.pop(ctx);
                  try {
                    final updated = await client!.updateMemo(
                      m.shortId,
                      visibility: isPublic ? 'PRIVATE' : 'PUBLIC',
                    );
                    ref.read(memosProvider.notifier).upsert(updated);
                  } catch (_) {}
                },
              ),
            ],
          ),
        ),
      ),
    );
  }

  int _count(List<Memo> pinned, Map<String, List<Memo>> groups, MemosState memos) {
    var n = 1; // 尾部元素
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

/// 卡片入场：淡入 + 轻微上滑（首屏 stagger，系统 reduce-motion 时跳过）
class _FadeSlideIn extends StatefulWidget {
  const _FadeSlideIn({required this.keyId, required this.index, required this.child});

  final String keyId;
  final int index;
  final Widget child;

  @override
  State<_FadeSlideIn> createState() => _FadeSlideInState();
}

class _FadeSlideInState extends State<_FadeSlideIn> with SingleTickerProviderStateMixin {
  late final _ctrl = AnimationController(vsync: this, duration: const Duration(milliseconds: 340));
  bool _started = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_started) return;
    _started = true;
    if (MediaQuery.disableAnimationsOf(context)) {
      _ctrl.value = 1; // 系统「移除动画」时直接呈现
    } else {
      Future.delayed(Duration(milliseconds: (widget.index % 10) * 40), () {
        if (mounted) _ctrl.forward();
      });
    }
  }

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return FadeTransition(
      opacity: CurvedAnimation(parent: _ctrl, curve: Curves.easeOut),
      child: SlideTransition(
        position: Tween<Offset>(begin: const Offset(0, 0.035), end: Offset.zero)
            .animate(CurvedAnimation(parent: _ctrl, curve: Curves.easeOutCubic)),
        child: widget.child,
      ),
    );
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
  const _Empty();

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    return ListView(
      physics: const AlwaysScrollableScrollPhysics(),
      children: [
        const SizedBox(height: 110),
        Center(
          child: Container(
            width: 92,
            height: 92,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              gradient: LinearGradient(
                begin: Alignment.topLeft,
                end: Alignment.bottomRight,
                colors: [
                  kPrimary.withValues(alpha: dark ? 0.25 : 0.16),
                  kAccent.withValues(alpha: dark ? 0.15 : 0.10),
                ],
              ),
            ),
            child: Icon(Icons.note_add_outlined, size: 40, color: kPrimary.withValues(alpha: 0.9)),
          ),
        ),
        const SizedBox(height: 16),
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
