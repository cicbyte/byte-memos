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
import 'ai_chat_page.dart';
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
        // 页头：标题 + 今日日期 + 视图切换（时间轴/卡片）
        Padding(
          padding: const EdgeInsets.fromLTRB(20, 14, 12, 4),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: [
              Text('备忘录', style: TextStyle(fontSize: 22, fontWeight: FontWeight.w800, color: fg)),
              const SizedBox(width: 10),
              Text(
                DateFormat('M月d日 · ', 'zh').format(DateTime.now()) +
                    (memos.items.isEmpty ? '' : '${memos.items.length} 条'),
                style: TextStyle(fontSize: 11.5, color: fg.withValues(alpha: 0.45)),
              ),
              const Spacer(),
              // AI 助手（D3）：内容场景直达对话页
              GestureDetector(
                onTap: () => Navigator.of(context).push(MaterialPageRoute(builder: (_) => const AiChatPage())),
                child: Container(
                  width: 32,
                  height: 32,
                  decoration: BoxDecoration(
                    color: kPrimary.withValues(alpha: 0.14),
                    borderRadius: BorderRadius.circular(9),
                  ),
                  child: const Icon(Icons.auto_awesome_rounded, size: 17, color: kPrimary),
                ),
              ),
              const SizedBox(width: 4),
              _ViewToggle(dark: dark, fg: fg),
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
                  : switch (ref.watch(flowViewModeProvider)) {
                      FlowViewMode.timeline => _TimelineList(memos: memos),
                      FlowViewMode.card => _FlowList(memos: memos),
                      FlowViewMode.calendar => _CalendarList(memos: memos),
                    },
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

/// 视图切换（时间轴 / 卡片）
class _ViewToggle extends ConsumerWidget {
  const _ViewToggle({required this.dark, required this.fg});

  final bool dark;
  final Color fg;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = ref.watch(flowViewModeProvider);
    Widget btn(IconData icon, FlowViewMode m, String tip) => Tooltip(
          message: tip,
          child: GestureDetector(
            onTap: () => ref.read(flowViewModeProvider.notifier).set(m),
            child: Container(
              width: 32,
              height: 32,
              decoration: BoxDecoration(
                color: mode == m ? kPrimary.withValues(alpha: 0.14) : Colors.transparent,
                borderRadius: BorderRadius.circular(9),
              ),
              child: Icon(
                icon,
                size: 17,
                color: mode == m ? kPrimary : fg.withValues(alpha: 0.4),
              ),
            ),
          ),
        );
    return Row(
      children: [
        btn(Icons.view_timeline_rounded, FlowViewMode.timeline, '时间轴'),
        const SizedBox(width: 4),
        btn(Icons.grid_view_rounded, FlowViewMode.card, '卡片'),
        const SizedBox(width: 4),
        btn(Icons.calendar_month_rounded, FlowViewMode.calendar, '日历'),
      ],
    );
  }
}

/// 日历视图（与桌面端 MemoCalendar 同形态）：月网格 + 日计数点 + 选中日列表
class _CalendarList extends ConsumerStatefulWidget {
  const _CalendarList({required this.memos});

  final MemosState memos;

  @override
  ConsumerState<_CalendarList> createState() => _CalendarListState();
}

class _CalendarListState extends ConsumerState<_CalendarList> {
  DateTime _month = DateTime.now();
  DateTime? _selected;

  static const _weekLabels = ['一', '二', '三', '四', '五', '六', '日'];

  /// 月网格：周一开头，前置补位；42 格（null = 非本月）
  List<DateTime?> _cells(DateTime month) {
    final lead = (DateTime(month.year, month.month, 1).weekday - 1) % 7; // 周一=0
    final days = DateTime(month.year, month.month + 1, 0).day;
    return [
      for (var i = 0; i < lead; i++) null,
      for (var d = 1; d <= days; d++) DateTime(month.year, month.month, d),
      for (var i = lead + days; i < 42; i++) null,
    ];
  }

  static String _key(DateTime d) =>
      '${d.year.toString().padLeft(4, '0')}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    final counts = ref.watch(dailyCountsProvider).valueOrNull ?? const <String, int>{};
    final today = DateTime.now();
    final cells = _cells(_month);

    // 选中日的备忘（本地日期匹配；待办/分类筛选同作用）
    final filter = ref.watch(categoryFilterProvider);
    final todo = ref.watch(todoFilterProvider);
    List<Memo> dayMemos = [];
    if (_selected != null) {
      final k = _key(_selected!);
      dayMemos = widget.memos.items.where((m) {
        final c = m.created?.toLocal();
        if (c == null || _key(c) != k) return false;
        if (todo && countUnchecked(m.content) == 0) return false;
        if (!todo && filter != null && (m.category ?? '') != filter) return false;
        return true;
      }).toList()
        ..sort((a, b) => (b.created ?? DateTime(0)).compareTo(a.created ?? DateTime(0)));
    }

    return Column(
      children: [
        // 月导航
        Padding(
          padding: const EdgeInsets.fromLTRB(20, 6, 12, 2),
          child: Row(
            children: [
              Text('${_month.year}年${_month.month}月',
                  style: TextStyle(fontSize: 16, fontWeight: FontWeight.w800, color: fg)),
              const Spacer(),
              if (_selected != null)
                TextButton(
                  onPressed: () => setState(() {
                    _month = DateTime.now();
                    _selected = DateTime.now();
                  }),
                  child: const Text('回今天', style: TextStyle(fontSize: 12)),
                ),
              IconButton(
                visualDensity: VisualDensity.compact,
                icon: Icon(Icons.chevron_left_rounded, size: 22, color: fg.withValues(alpha: 0.5)),
                onPressed: () => setState(() => _month = DateTime(_month.year, _month.month - 1, 1)),
              ),
              IconButton(
                visualDensity: VisualDensity.compact,
                icon: Icon(Icons.chevron_right_rounded, size: 22, color: fg.withValues(alpha: 0.5)),
                onPressed: () => setState(() => _month = DateTime(_month.year, _month.month + 1, 1)),
              ),
            ],
          ),
        ),
        // 周头
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14),
          child: Row(
            children: [
              for (final w in _weekLabels)
                Expanded(
                  child: Center(child: Text(w, style: TextStyle(fontSize: 10.5, color: fg.withValues(alpha: 0.4)))),
                ),
            ],
          ),
        ),
        const SizedBox(height: 4),
        // 日网格
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14),
          child: Column(
            children: [
              for (var r = 0; r < 6; r++)
                Row(
                  children: [
                    for (var c = 0; c < 7; c++) _cell(cells[r * 7 + c], counts, today, fg),
                  ],
                ),
            ],
          ),
        ),
        const SizedBox(height: 6),
        // 选中日列表
        Expanded(
          child: _selected == null
              ? Center(child: Text('点日期看当天的备忘', style: TextStyle(fontSize: 12, color: fg.withValues(alpha: 0.4))))
              : RefreshIndicator(
                  onRefresh: () => ref.read(memosProvider.notifier).refresh(),
                  color: kPrimary,
                  child: dayMemos.isEmpty
                      ? ListView(
                          physics: const AlwaysScrollableScrollPhysics(),
                          children: [
                            Padding(
                              padding: const EdgeInsets.only(top: 40),
                              child: Center(
                                child: Text('${_selected!.month}月${_selected!.day}日 · 无备忘',
                                    style: TextStyle(fontSize: 12, color: fg.withValues(alpha: 0.4))),
                              ),
                            ),
                          ],
                        )
                      : ListView.builder(
                          physics: const AlwaysScrollableScrollPhysics(),
                          padding: const EdgeInsets.fromLTRB(16, 4, 16, 190),
                          itemCount: dayMemos.length + 1,
                          itemBuilder: (context, i) {
                            if (i == 0) {
                              return Padding(
                                padding: const EdgeInsets.only(left: 4, bottom: 4),
                                child: Text(
                                  '${_selected!.month}月${_selected!.day}日 · ${dayMemos.length} 条',
                                  style: TextStyle(
                                      fontSize: 12.5, fontWeight: FontWeight.w700, fontFamily: 'serif', color: fg),
                                ),
                              );
                            }
                            final m = dayMemos[i - 1];
                            final dt = m.created?.toLocal();
                            final time = dt == null
                                ? ''
                                : '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
                            final excerpt = (m.title.isNotEmpty ? m.title : m.content)
                                .replaceAll(RegExp(r'[#>*_~`\-]+'), ' ')
                                .replaceAll(RegExp(r'\s+'), ' ')
                                .trim();
                            return GestureDetector(
                              behavior: HitTestBehavior.opaque,
                              onTap: () => _openDetail(context, ref, m),
                              onLongPress: () => _quickActions(context, ref, m),
                              child: Padding(
                                padding: const EdgeInsets.symmetric(vertical: 8),
                                child: Row(
                                  children: [
                                    SizedBox(
                                      width: 38,
                                      child: Text(time, style: TextStyle(fontSize: 11, color: fg.withValues(alpha: 0.45))),
                                    ),
                                    const SizedBox(width: 6),
                                    Expanded(
                                      child: Text(
                                        excerpt.isEmpty ? '（无标题）' : excerpt,
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                        style: TextStyle(fontSize: 13.5, fontWeight: FontWeight.w600, color: fg),
                                      ),
                                    ),
                                    if (m.pinned) ...[
                                      const SizedBox(width: 6),
                                      Icon(Icons.push_pin_rounded, size: 11, color: kPrimary.withValues(alpha: 0.9)),
                                    ],
                                    if (m.tags.isNotEmpty) ...[
                                      const SizedBox(width: 6),
                                      Text('#${m.tags.first}',
                                          style: TextStyle(fontSize: 10, color: kPrimary.withValues(alpha: 0.7))),
                                    ],
                                  ],
                                ),
                              ),
                            );
                          },
                        ),
                ),
        ),
      ],
    );
  }

  Widget _cell(DateTime? d, Map<String, int> counts, DateTime today, Color fg) {
    if (d == null) return const Expanded(child: SizedBox(height: 46));
    final n = counts[_key(d)] ?? 0;
    final isToday = d.year == today.year && d.month == today.month && d.day == today.day;
    final isSel = _selected != null &&
        d.year == _selected!.year &&
        d.month == _selected!.month &&
        d.day == _selected!.day;
    final isFuture = d.isAfter(DateTime(today.year, today.month, today.day));
    return Expanded(
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () => setState(() => _selected = d),
        child: Container(
          height: 46,
          margin: const EdgeInsets.all(1.5),
          decoration: BoxDecoration(
            color: isSel ? kPrimary.withValues(alpha: 0.16) : Colors.transparent,
            borderRadius: BorderRadius.circular(10),
            border: isToday && !isSel ? Border.all(color: kPrimary.withValues(alpha: 0.45), width: 1.2) : null,
          ),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Text(
                '${d.day}',
                style: TextStyle(
                  fontSize: 13,
                  fontWeight: isToday || isSel ? FontWeight.w700 : FontWeight.w400,
                  color: isFuture ? fg.withValues(alpha: 0.3) : (isSel ? kPrimary : fg.withValues(alpha: 0.8)),
                ),
              ),
              const SizedBox(height: 3),
              // 计数点：有记录的主色小点（多条加宽）
              Container(
                width: n > 0 ? (n > 1 ? 12.0 : 5.0) : 0,
                height: n > 0 ? 3.5 : 0,
                decoration: BoxDecoration(
                  color: kPrimary.withValues(alpha: isSel ? 1 : 0.55),
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// 时间轴列表（与桌面端 MemoTimeline 同形态）：按天分组纵向轨道 + 分类色圆点 + HH:mm
class _TimelineList extends ConsumerWidget {
  const _TimelineList({required this.memos});

  final MemosState memos;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
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
    // 时间轴不单列置顶区：按天分组，置顶条目带图钉标（与桌面一致）
    final groups = <String, List<Memo>>{};
    for (final m in visible) {
      final dt = m.created;
      final key = dt == null ? '更早' : _dayLabel(DateTime.now().difference(dt));
      (groups[key] ??= []).add(m);
    }
    if (visible.isEmpty) return const _Empty();

    final cats = ref.watch(categoriesProvider).valueOrNull ?? [];
    final catColors = {for (final c in cats) c.name: c.color};
    final dark = Theme.of(context).brightness == Brightness.dark;

    return RefreshIndicator(
      onRefresh: () => ref.read(memosProvider.notifier).refresh(),
      displacement: 20,
      color: kPrimary,
      child: ListView.builder(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 190),
        itemCount: groups.length * 2, // 每组：日期头 + 轨道容器
        itemBuilder: (context, i) {
          final entry = groups.entries.elementAt(i ~/ 2);
          if (i.isOdd) {
            return Container(
              margin: const EdgeInsets.only(left: 14),
              padding: const EdgeInsets.only(left: 18),
              decoration: BoxDecoration(
                border: Border(
                  left: BorderSide(
                    width: 2,
                    color: entry.key == '今天' ? kPrimary.withValues(alpha: 0.45) : (dark ? Colors.white12 : Colors.black12),
                  ),
                ),
              ),
              child: Column(
                children: [
                  for (final m in entry.value)
                    _TlEntry(
                      memo: m,
                      color: _parseColor(catColors[m.category]) ?? kPrimary,
                      onTap: () => _openDetail(context, ref, m),
                      onLongPress: () => _quickActions(context, ref, m),
                    ),
                ],
              ),
            );
          }
          return Padding(
            padding: const EdgeInsets.fromLTRB(2, 12, 0, 6),
            child: Row(
              children: [
                Text(
                  entry.key,
                  style: TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w700,
                    fontFamily: 'serif',
                    color: entry.key == '今天' ? kPrimary : (dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328)),
                  ),
                ),
                const SizedBox(width: 8),
                Text('${entry.value.length} 条', style: TextStyle(fontSize: 10, color: Theme.of(context).hintColor)),
              ],
            ),
          );
        },
      ),
    );
  }

  static Color? _parseColor(String? hex) {
    if (hex == null) return null;
    final h = hex.replaceFirst('#', '');
    if (h.length != 6) return null;
    final v = int.tryParse('FF$h', radix: 16);
    return v == null ? null : Color(v);
  }
}

/// 轨道上的单条挂件：圆点（分类色/置顶描环）+ 时间 + 标题摘要
class _TlEntry extends StatelessWidget {
  const _TlEntry({required this.memo, required this.color, required this.onTap, required this.onLongPress});

  final Memo memo;
  final Color color;
  final VoidCallback onTap;
  final VoidCallback onLongPress;

  String get _time {
    final dt = memo.created;
    if (dt == null) return '';
    return '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }

  String get _excerpt {
    final src = memo.title.isNotEmpty ? memo.title : memo.content;
    return src.replaceAll(RegExp(r'[#>*_~`\-]+'), ' ').replaceAll(RegExp(r'\s+'), ' ').trim();
  }

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    final bg = dark ? const Color(0xFF191C1B) : const Color(0xFFF8F6F1);
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      onLongPress: onLongPress,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 7),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // 圆点（叠在轨道线上，bg 披光环隔开轨道）
            Transform.translate(
              offset: const Offset(-23.5, 5),
              child: Container(
                width: 9,
                height: 9,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: color,
                  boxShadow: [BoxShadow(color: bg, blurRadius: 0, spreadRadius: 3)],
                  border: memo.pinned ? Border.all(color: kPrimary.withValues(alpha: 0.5), width: 1.5) : null,
                ),
              ),
            ),
            SizedBox(
              width: 38,
              child: Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Text(_time, style: TextStyle(fontSize: 11, color: fg.withValues(alpha: 0.45))),
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      if (memo.pinned) ...[
                        Icon(Icons.push_pin_rounded, size: 11, color: kPrimary.withValues(alpha: 0.9)),
                        const SizedBox(width: 4),
                      ],
                      Expanded(
                        child: Text(
                          _excerpt.isEmpty ? '（无标题）' : _excerpt,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: TextStyle(fontSize: 13.5, fontWeight: FontWeight.w600, color: fg),
                        ),
                      ),
                    ],
                  ),
                  if (memo.tags.isNotEmpty) ...[
                    const SizedBox(height: 3),
                    Text(
                      memo.tags.take(4).map((t) => '#$t').join('  '),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(fontSize: 10.5, color: kAccent.withValues(alpha: 0.9)),
                    ),
                  ],
                ],
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

  int _count(List<Memo> pinned, Map<String, List<Memo>> groups, MemosState memos) {
    var n = 1; // 尾部元素
    if (pinned.isNotEmpty) n += 1 + pinned.length;
    n += groups.entries.fold<int>(0, (acc, e) => acc + 1 + e.value.length);
    return n;
  }
}

void _openDetail(BuildContext context, WidgetRef ref, Memo m) {
  Navigator.of(context).push(MaterialPageRoute(builder: (_) => DetailPage(memoId: m.shortId)));
}

String _dayLabel(Duration diff) {
  if (diff.inDays == 0) return '今天';
  if (diff.inDays == 1) return '昨天';
  if (diff.inDays < 7) return '${diff.inDays} 天前';
  return DateFormat('M 月 d 日', 'zh').format(DateTime.now().subtract(diff));
}

/// F11：长按快捷菜单（置顶 / 公开-私有），时间轴与卡片视图共用
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
