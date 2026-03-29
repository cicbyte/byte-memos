/// 我的页：连接状态卡 + 多连接管理（增/改/删/切换/测试）+ 外观（玻璃强度/主题）
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../conn/conn_store.dart';
import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';

class MinePage extends ConsumerWidget {
  const MinePage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    final conn = ref.watch(connProvider);
    final glassPref = ref.watch(glassPrefProvider);
    final themeMode = ref.watch(themeModeProvider);

    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 190),
      children: [
        Text('我的', style: TextStyle(fontSize: 22, fontWeight: FontWeight.w800, color: fg)),
        const SizedBox(height: 14),

        // ===== 连接 =====
        _SectionLabel('连接', fg),
        const SizedBox(height: 8),
        GlassContainer(
          tier: GlassTier.flat,
          preference: glassPref,
          borderRadius: BorderRadius.circular(16),
          padding: const EdgeInsets.all(14),
          child: Column(
            children: [
              // 状态行
              Row(
                children: [
                  _StatusDot(status: conn.status),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      switch (conn.status) {
                        ConnStatus.none => '未配置连接',
                        ConnStatus.connecting => '连接中…',
                        ConnStatus.online => '已连接 · ${conn.activeProfile?.name ?? ''}',
                        ConnStatus.keyError => 'API Key 不正确',
                        ConnStatus.offline => '无法连接（服务未开？）',
                      },
                      style: TextStyle(fontSize: 13.5, fontWeight: FontWeight.w600, color: fg),
                    ),
                  ),
                  if (conn.status != ConnStatus.none)
                    IconButton(
                      visualDensity: VisualDensity.compact,
                      icon: const Icon(Icons.refresh_rounded, size: 19),
                      onPressed: () {
                        final p = conn.activeProfile;
                        if (p != null) ref.read(connProvider.notifier).probe(p);
                      },
                    ),
                ],
              ),
              const SizedBox(height: 6),
              // 配置列表
              for (final p in conn.profiles)
                ListTile(
                  dense: true,
                  contentPadding: EdgeInsets.zero,
                  leading: Icon(
                    p.id == conn.activeId ? Icons.radio_button_checked_rounded : Icons.radio_button_off_rounded,
                    size: 17,
                    color: p.id == conn.activeId ? kIndigo : (dark ? Colors.white30 : Colors.black26),
                  ),
                  title: Text(
                    '${p.name} · ${_shortUrl(p.baseUrl)}',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(fontSize: 12.5, color: fg),
                  ),
                  trailing: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      IconButton(
                        visualDensity: VisualDensity.compact,
                        icon: Icon(Icons.edit_outlined, size: 16, color: dark ? Colors.white38 : Colors.black38),
                        onPressed: () => _showProfileForm(context, ref, existing: p),
                      ),
                      IconButton(
                        visualDensity: VisualDensity.compact,
                        icon: Icon(Icons.delete_outline_rounded, size: 17, color: Colors.redAccent.withValues(alpha: 0.7)),
                        onPressed: () => ref.read(connProvider.notifier).remove(p.id),
                      ),
                    ],
                  ),
                  onTap: () => ref.read(connProvider.notifier).activate(p.id),
                ),
              // 新增
              Align(
                alignment: Alignment.centerLeft,
                child: TextButton.icon(
                  onPressed: () => _showProfileForm(context, ref),
                  icon: const Icon(Icons.add_rounded, size: 17),
                  label: const Text('添加连接', style: TextStyle(fontSize: 12.5)),
                ),
              ),
              const SizedBox(height: 4),
              Text(
                '在桌面端「REST API 服务」页启动服务后，这里填它的地址（默认端口 8318）。同一 Wi-Fi 用电脑局域网 IP；模拟器联调用 adb reverse 后填 127.0.0.1。',
                style: TextStyle(fontSize: 11, height: 1.6, color: fg.withValues(alpha: 0.5)),
              ),
            ],
          ),
        ),

        const SizedBox(height: 20),

        // ===== 外观 =====
        _SectionLabel('外观', fg),
        const SizedBox(height: 8),
        GlassContainer(
          tier: GlassTier.flat,
          preference: glassPref,
          borderRadius: BorderRadius.circular(16),
          padding: const EdgeInsets.all(14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('玻璃强度', style: TextStyle(fontSize: 12.5, fontWeight: FontWeight.w600, color: fg)),
              const SizedBox(height: 4),
              Text('省电：关闭所有实时模糊，保留透明度层级', style: TextStyle(fontSize: 11, color: fg.withValues(alpha: 0.5))),
              const SizedBox(height: 10),
              SegmentedButton<GlassPreference>(
                segments: [
                  for (final p in GlassPreference.values) ButtonSegment(value: p, label: Text(p.label, style: const TextStyle(fontSize: 12))),
                ],
                selected: {glassPref},
                onSelectionChanged: (s) => ref.read(glassPrefProvider.notifier).set(s.first),
                showSelectedIcon: false,
                style: ButtonStyle(
                  visualDensity: VisualDensity.compact,
                  backgroundColor: WidgetStateProperty.resolveWith((states) =>
                      states.contains(WidgetState.selected) ? kIndigo.withValues(alpha: 0.85) : null),
                  foregroundColor: WidgetStateProperty.resolveWith((states) =>
                      states.contains(WidgetState.selected) ? Colors.white : fg.withValues(alpha: 0.6)),
                ),
              ),
              const SizedBox(height: 14),
              Text('主题', style: TextStyle(fontSize: 12.5, fontWeight: FontWeight.w600, color: fg)),
              const SizedBox(height: 10),
              SegmentedButton<ThemeMode>(
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
                      states.contains(WidgetState.selected) ? kIndigo.withValues(alpha: 0.85) : null),
                  foregroundColor: WidgetStateProperty.resolveWith((states) =>
                      states.contains(WidgetState.selected) ? Colors.white : fg.withValues(alpha: 0.6)),
                ),
              ),
            ],
          ),
        ),

        const SizedBox(height: 20),
        Center(
          child: Text(
            'byte-memos mobile v0.1.0 · 液态玻璃设计',
            style: TextStyle(fontSize: 10.5, color: fg.withValues(alpha: 0.4)),
          ),
        ),
      ],
    );
  }

  String _shortUrl(String url) => url.replaceFirst(RegExp(r'^https?://'), '');

  void _showProfileForm(BuildContext context, WidgetRef ref, {ConnProfile? existing}) {
    final name = TextEditingController(text: existing?.name ?? '');
    final url = TextEditingController(text: existing?.baseUrl ?? 'http://');
    final key = TextEditingController(text: existing?.apiKey ?? '');
    final dark = Theme.of(context).brightness == Brightness.dark;

    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (ctx) => Padding(
        padding: EdgeInsets.only(bottom: MediaQuery.viewInsetsOf(ctx).bottom),
        child: GlassContainer(
          tier: GlassTier.thick,
          preference: ref.read(glassPrefProvider),
          borderRadius: const BorderRadius.vertical(top: Radius.circular(24)),
          padding: const EdgeInsets.all(20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(existing == null ? '添加连接' : '编辑连接',
                  style: TextStyle(fontSize: 15.5, fontWeight: FontWeight.w700, color: dark ? Colors.white : Colors.black87)),
              const SizedBox(height: 14),
              _field(ctx, name, '名称（如：家里台式机）', dark),
              const SizedBox(height: 10),
              _field(ctx, url, 'http://192.168.1.10:8318', dark),
              const SizedBox(height: 10),
              _field(ctx, key, 'API Key（开「本机免 Key」可留空）', dark, obscure: true),
              const SizedBox(height: 16),
              Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
                  const SizedBox(width: 8),
                  FilledButton(
                    style: FilledButton.styleFrom(
                      backgroundColor: kIndigo,
                      foregroundColor: Colors.white,
                    ),
                    onPressed: () {
                      final u = url.text.trim().replaceAll(RegExp(r'/+$'), '');
                      if (!RegExp(r'^https?://.+:\\d+$|^https?://.+$').hasMatch(u)) {
                        ScaffoldMessenger.of(ctx).showSnackBar(const SnackBar(content: Text('地址不合法，需以 http(s):// 开头')));
                        return;
                      }
                      final p = ConnProfile(
                        id: existing?.id ?? DateTime.now().microsecondsSinceEpoch.toString(),
                        name: name.text.trim().isEmpty ? '连接' : name.text.trim(),
                        baseUrl: u,
                        apiKey: key.text.trim(),
                      );
                      Navigator.pop(ctx);
                      ref.read(connProvider.notifier).saveAndActivate(p);
                    },
                    child: const Text('保存并测试'),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _field(BuildContext ctx, TextEditingController c, String hint, bool dark, {bool obscure = false}) {
    return TextField(
      controller: c,
      obscureText: obscure,
      style: TextStyle(fontSize: 13, color: dark ? Colors.white : Colors.black87),
      decoration: InputDecoration(
        hintText: hint,
        hintStyle: TextStyle(fontSize: 12.5, color: dark ? Colors.white38 : Colors.black38),
        isDense: true,
        contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 11),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(10),
          borderSide: BorderSide(color: dark ? Colors.white24 : Colors.black12),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(10),
          borderSide: BorderSide(color: dark ? Colors.white24 : Colors.black12),
        ),
      ),
    );
  }
}

class _SectionLabel extends StatelessWidget {
  const _SectionLabel(this.text, this.fg);

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
      width: 9,
      height: 9,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: color,
        boxShadow: glow ? [BoxShadow(color: color.withValues(alpha: 0.5), blurRadius: 6, spreadRadius: 1)] : null,
      ),
    );
  }
}
