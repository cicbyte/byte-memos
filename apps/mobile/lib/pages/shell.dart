/// 应用外壳：彩色渐变舞台 + 四 Tab + 底部玻璃导航 + 玻璃 FAB + 离线横幅
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';
import '../state/memos_state.dart';
import 'edit_sheet.dart';
import 'flow_page.dart';
import 'mine_page.dart';
import 'search_page.dart';
import 'stats_page.dart';

class AppShell extends ConsumerStatefulWidget {
  const AppShell({super.key});

  @override
  ConsumerState<AppShell> createState() => _AppShellState();
}

class _AppShellState extends ConsumerState<AppShell> {
  int _tab = 0;

  @override
  void initState() {
    super.initState();
    // 首帧后触发连接探测 + 首刷
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(memosProvider.notifier).refresh();
    });
  }

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final glassPref = ref.watch(glassPrefProvider);
    final conn = ref.watch(connProvider);
    final offline = conn.status == ConnStatus.offline || conn.status == ConnStatus.keyError;

    return Scaffold(
      // 彩色渐变舞台：玻璃的底色
      body: Container(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: bgGradient(dark ? Brightness.dark : Brightness.light),
          ),
        ),
        child: SafeArea(
          bottom: false,
          child: Stack(
            children: [
              // 内容层
              Column(
                children: [
                  if (offline) const _OfflineBanner(),
                  Expanded(
                    child: IndexedStack(
                      index: _tab,
                      children: [const FlowPage(), SearchPage(), const StatsPage(), const MinePage()],
                    ),
                  ),
                ],
              ),

              // 玻璃 FAB：新建/速记（S1 场景入口）
              Positioned(
                right: 20,
                bottom: 108,
                child: _GlassFab(
                  preference: glassPref,
                  onTap: () => showEditSheet(context, ref),
                ),
              ),

              // 底部玻璃导航
              Positioned(
                left: 16,
                right: 16,
                bottom: 16,
                child: GlassContainer(
                  tier: GlassTier.regular,
                  preference: glassPref,
                  borderRadius: BorderRadius.circular(28),
                  padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.spaceAround,
                    children: [
                      for (final (i, item) in _tabs.indexed)
                        _NavItem(
                          icon: item.$icon,
                          label: item.label,
                          active: _tab == i,
                          onTap: () => setState(() => _tab = i),
                        ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _TabSpec {
  const _TabSpec(this.label, this.$icon);
  final String label;
  final IconData $icon;
}

const _tabs = [
  _TabSpec('流', Icons.blur_on_rounded),
  _TabSpec('搜索', Icons.search_rounded),
  _TabSpec('统计', Icons.insights_rounded),
  _TabSpec('我的', Icons.person_rounded),
];

/// 玻璃 FAB：按下弹性回弹（REQUIREMENTS §5.2 触觉动效）
class _GlassFab extends StatefulWidget {
  const _GlassFab({required this.preference, required this.onTap});

  final GlassPreference preference;
  final VoidCallback onTap;

  @override
  State<_GlassFab> createState() => _GlassFabState();
}

class _GlassFabState extends State<_GlassFab> with SingleTickerProviderStateMixin {
  late final _ctrl = AnimationController(
    vsync: this,
    duration: const Duration(milliseconds: 250),
    lowerBound: 0.96,
    upperBound: 1,
    value: 1,
  );

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    return GestureDetector(
      onTapDown: (_) => _ctrl.forward(from: _ctrl.lowerBound),
      onTapUp: (_) {
        _ctrl.animateTo(1, curve: Curves.elasticOut);
        widget.onTap();
      },
      child: ScaleTransition(
        scale: _ctrl,
        child: GlassContainer(
          tier: GlassTier.regular,
          preference: widget.preference,
          borderRadius: BorderRadius.circular(24),
          padding: const EdgeInsets.all(18),
          child: Semantics(
            label: '新建备忘',
            button: true,
            child: Icon(
              Icons.add_rounded,
              size: 28,
              color: dark ? Colors.white : kIndigo,
            ),
          ),
        ),
      ),
    );
  }
  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }
}

class _NavItem extends StatelessWidget {
  const _NavItem({required this.icon, required this.label, required this.active, required this.onTap});

  final IconData icon;
  final String label;
  final bool active;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final activeColor = dark ? Colors.white : kIndigo;
    final inactiveColor = dark ? Colors.white38 : Colors.black38;
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
        padding: EdgeInsets.symmetric(horizontal: 16, vertical: active ? 6 : 4),
        transform: Matrix4.translationValues(0, active ? -2 : 0, 0), // 选中态「悬浮位移」
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 22, color: active ? activeColor : inactiveColor),
            const SizedBox(height: 2),
            Text(
              label,
              style: TextStyle(
                fontSize: 10.5,
                fontWeight: active ? FontWeight.w600 : FontWeight.w400,
                color: active ? activeColor : inactiveColor,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// 离线横幅（F07）：状态可视化 + 快照时间
class _OfflineBanner extends ConsumerWidget {
  const _OfflineBanner();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final memos = ref.watch(memosProvider);
    final conn = ref.watch(connProvider);
    final isKey = conn.status == ConnStatus.keyError;
    final mins = memos.lastSyncAt == null
        ? null
        : DateTime.now().difference(memos.lastSyncAt!).inMinutes;
    return Material(
      color: isKey ? Colors.orange.withValues(alpha: 0.9) : Colors.black54,
      child: Padding(
        padding: EdgeInsets.only(top: MediaQuery.paddingOf(context).top + 4, bottom: 6),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(isKey ? Icons.key_off_rounded : Icons.wifi_off_rounded, size: 13, color: Colors.white70),
            const SizedBox(width: 6),
            Text(
              isKey
                  ? 'API Key 不正确 · 去「我的」修改'
                  : memos.fromSnapshot
                      ? '离线 · 浏览 ${mins == null ? '早前' : '$mins 分钟前'}的快照'
                      : '连接中断 · 点击重试',
              style: const TextStyle(fontSize: 12, color: Colors.white70),
            ),
          ],
        ),
      ),
    );
  }
}
