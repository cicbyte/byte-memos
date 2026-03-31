/// 我的页：连接 Hero + 数据速览 + 设置 / 关于 · 操作进二级页
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';
import '../state/memos_state.dart';
import 'appearance_page.dart';
import 'connections_page.dart';

class MinePage extends ConsumerWidget {
  const MinePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    final glassPref = ref.watch(glassPrefProvider);
    final themeMode = ref.watch(themeModeProvider);
    final act = ref.watch(streakProvider).valueOrNull ?? const ActivityStats(today: 0, streak: 0, todos: 0);

    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 190),
      children: [
        Text('我的', style: TextStyle(fontSize: 22, fontWeight: FontWeight.w800, color: fg)),
        const SizedBox(height: 16),

        // ===== 连接 Hero 卡（点进连接管理） =====
        _ConnHero(fg: fg, glassPref: glassPref),
        const SizedBox(height: 12),

        // ===== 数据速览（今日 / 连续 / 待办） =====
        GlassContainer(
          tier: GlassTier.flat,
          preference: glassPref,
          borderRadius: BorderRadius.circular(18),
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 14),
          child: Row(
            children: [
              _StatMini(value: '${act.today}', label: '今日', color: kPrimary, fg: fg),
              _VDiv(fg: fg),
              _StatMini(value: '${act.streak}', label: '连续天', color: const Color(0xFFF59E0B), fg: fg),
              _VDiv(fg: fg),
              _StatMini(value: '${act.todos}', label: '待办', color: kAccent, fg: fg),
            ],
          ),
        ),
        const SizedBox(height: 20),

        // ===== 设置 =====
        _GroupLabel('设置', fg),
        const SizedBox(height: 8),
        GlassContainer(
          tier: GlassTier.flat,
          preference: glassPref,
          borderRadius: BorderRadius.circular(18),
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
          child: _Row(
            icon: Icons.palette_outlined,
            tint: kAccent,
            title: '外观',
            subtitle: '玻璃 ${glassPref.label} · 主题 ${switch (themeMode) {
              ThemeMode.system => '跟随系统',
              ThemeMode.light => '浅色',
              ThemeMode.dark => '深色',
            }}',
            onTap: () => Navigator.of(context).push(MaterialPageRoute(builder: (_) => const AppearancePage())),
          ),
        ),
        const SizedBox(height: 20),

        // ===== 关于 =====
        _GroupLabel('关于', fg),
        const SizedBox(height: 8),
        GlassContainer(
          tier: GlassTier.flat,
          preference: glassPref,
          borderRadius: BorderRadius.circular(18),
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
          child: _Row(
            icon: Icons.bookmark_rounded,
            tint: kPrimary,
            title: '关于 byte-memos',
            subtitle: 'v0.1.0 · 液态玻璃设计',
            onTap: () => _showAbout(context, ref, fg),
          ),
        ),
        const SizedBox(height: 24),
        Center(
          child: Text(
            'byte-memos · 液态玻璃设计',
            style: TextStyle(fontSize: 10.5, color: fg.withValues(alpha: 0.4)),
          ),
        ),
      ],
    );
  }
  /// 关于弹层：应用介绍 + 技术栈
  void _showAbout(BuildContext context, WidgetRef ref, Color fg) {
    showModalBottomSheet(
      context: context,
      backgroundColor: Colors.transparent,
      builder: (ctx) => GlassContainer(
        tier: GlassTier.thick,
        preference: ref.read(glassPrefProvider),
        borderRadius: const BorderRadius.vertical(top: Radius.circular(24)),
        padding: const EdgeInsets.fromLTRB(24, 20, 24, 30),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Container(
                  width: 42,
                  height: 42,
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(12),
                    gradient: LinearGradient(colors: [kPrimary.withValues(alpha: 0.35), kAccent.withValues(alpha: 0.35)]),
                  ),
                  child: const Icon(Icons.sticky_note_2_rounded, size: 21, color: kPrimary),
                ),
                const SizedBox(width: 12),
                Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('byte-memos', style: TextStyle(fontSize: 17, fontWeight: FontWeight.w800, color: fg)),
                    const SizedBox(height: 2),
                    Text('v0.1.0 · 液态玻璃设计', style: TextStyle(fontSize: 11, color: fg.withValues(alpha: 0.5))),
                  ],
                ),
              ],
            ),
            const SizedBox(height: 16),
            Text(
              '自托管的双端备忘录：桌面端（Tauri）管库与 AI，移动端（Flutter）扫码配对、随手记、时间轴浏览、勾选待办与图片收纳。',
              style: TextStyle(fontSize: 12.5, height: 1.8, color: fg.withValues(alpha: 0.75)),
            ),
            const SizedBox(height: 12),
            Text(
              '设计语言 · 液态玻璃（分层磨砂 / 透明度层级）\n数据 · 局域网 REST 直连桌面端，不经过任何云端',
              style: TextStyle(fontSize: 11, height: 1.9, color: fg.withValues(alpha: 0.45)),
            ),
          ],
        ),
      ),
    );
  }
}

/// 数据速览小列
class _StatMini extends StatelessWidget {
  const _StatMini({required this.value, required this.label, required this.color, required this.fg});

  final String value;
  final String label;
  final Color color;
  final Color fg;

  @override
  Widget build(BuildContext context) {
    return Expanded(
      child: Column(
        children: [
          Text(value, style: TextStyle(fontSize: 21, fontWeight: FontWeight.w800, color: color, height: 1.15)),
          const SizedBox(height: 3),
          Text(label, style: TextStyle(fontSize: 10.5, color: fg.withValues(alpha: 0.5))),
        ],
      ),
    );
  }
}

class _VDiv extends StatelessWidget {
  const _VDiv({required this.fg});

  final Color fg;

  @override
  Widget build(BuildContext context) => Container(width: 0.5, height: 30, color: fg.withValues(alpha: 0.1));
}

/// 分组标签
class _GroupLabel extends StatelessWidget {
  const _GroupLabel(this.text, this.fg);

  final String text;
  final Color fg;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(left: 4),
      child: Text(text, style: TextStyle(fontSize: 12.5, fontWeight: FontWeight.w600, color: fg.withValues(alpha: 0.55))),
    );
  }
}

/// 连接状态 Hero 卡：状态点 + 当前主机 + 地址，整卡进连接管理
class _ConnHero extends ConsumerWidget {
  const _ConnHero({required this.fg, required this.glassPref});

  final Color fg;
  final GlassPreference glassPref;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final conn = ref.watch(connProvider);
    final dark = Theme.of(context).brightness == Brightness.dark;
    final statusText = switch (conn.status) {
      ConnStatus.none => '未配置连接',
      ConnStatus.connecting => '连接中…',
      ConnStatus.online => '已连接',
      ConnStatus.keyError => 'API Key 不正确',
      ConnStatus.offline => '无法连接',
    };
    final active = conn.activeProfile;
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () => Navigator.of(context).push(MaterialPageRoute(builder: (_) => const ConnectionsPage())),
      child: GlassContainer(
        tier: GlassTier.regular,
        preference: glassPref,
        borderRadius: BorderRadius.circular(18),
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            // 主机头像圆
            Container(
              width: 44,
              height: 44,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                gradient: LinearGradient(colors: [kPrimary.withValues(alpha: 0.35), kAccent.withValues(alpha: 0.35)]),
              ),
              child: Icon(
                conn.status == ConnStatus.online ? Icons.computer_rounded : Icons.computer_outlined,
                size: 21,
                color: conn.status == ConnStatus.online ? kPrimary : fg.withValues(alpha: 0.6),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      _StatusDot(status: conn.status),
                      const SizedBox(width: 6),
                      Text(
                        active == null ? statusText : '$statusText · ${active.name}',
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(fontSize: 14.5, fontWeight: FontWeight.w700, color: fg),
                      ),
                    ],
                  ),
                  const SizedBox(height: 3),
                  Text(
                    active == null ? '点此扫码或手填，连上桌面端' : _shortUrl(active.baseUrl),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(fontSize: 11.5, color: fg.withValues(alpha: 0.5)),
                  ),
                ],
              ),
            ),
            if (conn.status != ConnStatus.none)
              IconButton(
                visualDensity: VisualDensity.compact,
                icon: Icon(Icons.refresh_rounded, size: 19, color: dark ? Colors.white54 : Colors.black45),
                onPressed: () {
                  final p = conn.activeProfile;
                  if (p != null) ref.read(connProvider.notifier).probe(p);
                },
              ),
            Icon(Icons.chevron_right_rounded, size: 22, color: fg.withValues(alpha: 0.35)),
          ],
        ),
      ),
    );
  }

  static String _shortUrl(String url) => url.replaceFirst(RegExp(r'^https?://'), '');
}

/// 统一设置行：圆角渐变图标 + 标题 + 摘要 + chevron（fixedTrailing 覆盖右侧）
class _Row extends StatelessWidget {
  const _Row({
    required this.icon,
    required this.tint,
    required this.title,
    required this.subtitle,
    this.onTap,
  });

  final IconData icon;
  final Color tint;
  final String title;
  final String subtitle;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    return ListTile(
      contentPadding: const EdgeInsets.symmetric(horizontal: 10),
      visualDensity: VisualDensity.comfortable,
      leading: Container(
        width: 34,
        height: 34,
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(10),
          gradient: LinearGradient(colors: [tint.withValues(alpha: 0.3), tint.withValues(alpha: 0.12)]),
        ),
        child: Icon(icon, size: 17, color: tint),
      ),
      title: Text(title, style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: fg)),
      subtitle: subtitle.isEmpty ? null : Text(subtitle, maxLines: 1, overflow: TextOverflow.ellipsis, style: TextStyle(fontSize: 11, color: fg.withValues(alpha: 0.5))),
      trailing: Icon(Icons.chevron_right_rounded, size: 20, color: fg.withValues(alpha: 0.35)),
      onTap: onTap,
    );
  }
}

class _StatusDot extends StatelessWidget {
  const _StatusDot({required this.status});

  final ConnStatus status;

  @override
  Widget build(BuildContext context) {
    final (color, glow) = switch (status) {
      ConnStatus.online => (const Color(0xFF22C55E), true),
      ConnStatus.keyError => (const Color(0xFFF59E0B), true),
      ConnStatus.offline => (const Color(0xFFEF4444), true),
      ConnStatus.connecting => (const Color(0xFF9CA3AF), false),
      ConnStatus.none => (const Color(0xFF9CA3AF), false),
    };
    return Container(
      width: 8,
      height: 8,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: color,
        boxShadow: glow ? [BoxShadow(color: color.withValues(alpha: 0.5), blurRadius: 6, spreadRadius: 1)] : null,
      ),
    );
  }
}
