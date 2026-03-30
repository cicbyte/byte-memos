/// 连接管理页（我的 → 连接 Hero 卡进入）：多配置增/改/删/切换 + 扫码/剪贴板配对
library;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../conn/conn_store.dart';
import '../conn/pairing.dart';
import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';
import 'pair_scan_page.dart';

class ConnectionsPage extends ConsumerWidget {
  const ConnectionsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    final conn = ref.watch(connProvider);

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
              // 顶栏（与回收站同款玻璃吸顶）
              GlassContainer(
                tier: GlassTier.regular,
                preference: ref.watch(glassPrefProvider),
                padding: EdgeInsets.zero,
                child: Row(
                  children: [
                    IconButton(
                      icon: const Icon(Icons.arrow_back_rounded),
                      onPressed: () => Navigator.pop(context),
                    ),
                    Text('连接管理', style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600, color: fg)),
                    const Spacer(),
                    TextButton.icon(
                      onPressed: () => _showProfileForm(context, ref),
                      icon: const Icon(Icons.add_rounded, size: 18),
                      label: const Text('添加', style: TextStyle(fontSize: 13)),
                    ),
                  ],
                ),
              ),
              Expanded(
                child: ListView(
                  padding: const EdgeInsets.fromLTRB(16, 14, 16, 40),
                  children: [
                    // 配置列表卡
                    GlassContainer(
                      tier: GlassTier.flat,
                      preference: ref.watch(glassPrefProvider),
                      borderRadius: BorderRadius.circular(18),
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                      child: conn.profiles.isEmpty
                          ? Padding(
                              padding: const EdgeInsets.all(18),
                              child: Center(
                                child: Text('还没有连接配置', style: TextStyle(fontSize: 12.5, color: fg.withValues(alpha: 0.5))),
                              ),
                            )
                          : Column(
                              children: [
                                for (var i = 0; i < conn.profiles.length; i++) ...[
                                  if (i > 0) Container(margin: const EdgeInsets.only(left: 52), height: 0.5, color: fg.withValues(alpha: 0.08)),
                                  _profileTile(context, ref, conn.profiles[i], conn.activeId, fg, dark),
                                ],
                              ],
                            ),
                    ),
                    const SizedBox(height: 14),
                    // 快捷配对
                    Row(
                      children: [
                        Expanded(
                          child: OutlinedButton.icon(
                            style: OutlinedButton.styleFrom(
                              foregroundColor: dark ? Colors.white : kPrimary,
                              side: BorderSide(color: dark ? Colors.white24 : kPrimary.withValues(alpha: 0.5)),
                              padding: const EdgeInsets.symmetric(vertical: 11),
                            ),
                            onPressed: () async {
                              final payload = await Navigator.of(context).push<PairingPayload>(
                                MaterialPageRoute(builder: (_) => const PairScanPage()),
                              );
                              if (payload != null) {
                                ref.read(connProvider.notifier).saveAndActivate(ConnProfile(
                                  id: DateTime.now().microsecondsSinceEpoch.toString(),
                                  name: payload.name,
                                  baseUrl: payload.baseUrl,
                                  apiKey: payload.apiKey,
                                ));
                              }
                            },
                            icon: const Icon(Icons.qr_code_scanner_rounded, size: 17),
                            label: const Text('扫码配对', style: TextStyle(fontSize: 12.5)),
                          ),
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: OutlinedButton.icon(
                            style: OutlinedButton.styleFrom(
                              foregroundColor: dark ? Colors.white : kPrimary,
                              side: BorderSide(color: dark ? Colors.white24 : kPrimary.withValues(alpha: 0.5)),
                              padding: const EdgeInsets.symmetric(vertical: 11),
                            ),
                            onPressed: () async {
                              final data = await Clipboard.getData('text/plain');
                              if (!context.mounted) return;
                              final payload = data?.text == null ? null : parsePairing(data!.text!);
                              if (payload == null) {
                                ScaffoldMessenger.of(context).showSnackBar(
                                  const SnackBar(content: Text('剪贴板没有配对串（在桌面端复制后重试）')),
                                );
                                return;
                              }
                              ref.read(connProvider.notifier).saveAndActivate(ConnProfile(
                                id: DateTime.now().microsecondsSinceEpoch.toString(),
                                name: payload.name,
                                baseUrl: payload.baseUrl,
                                apiKey: payload.apiKey,
                              ));
                            },
                            icon: const Icon(Icons.content_paste_rounded, size: 17),
                            label: const Text('从剪贴板配对', style: TextStyle(fontSize: 12.5)),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 18),
                    Text(
                      '在桌面端「REST API 服务」页启动服务后，这里填它的地址（默认端口 8318）。同一 Wi-Fi 用电脑局域网 IP；模拟器联调用 adb reverse 后填 127.0.0.1。',
                      style: TextStyle(fontSize: 11, height: 1.6, color: fg.withValues(alpha: 0.5)),
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

  Widget _profileTile(BuildContext context, WidgetRef ref, ConnProfile p, String? activeId, Color fg, bool dark) {
    final active = p.id == activeId;
    return ListTile(
      dense: true,
      contentPadding: const EdgeInsets.symmetric(horizontal: 10),
      leading: Icon(
        active ? Icons.radio_button_checked_rounded : Icons.radio_button_off_rounded,
        size: 17,
        color: active ? kPrimary : (dark ? Colors.white30 : Colors.black26),
      ),
      title: Text(
        p.name,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: TextStyle(fontSize: 13.5, fontWeight: active ? FontWeight.w600 : FontWeight.w400, color: fg),
      ),
      subtitle: Text(
        p.baseUrl.replaceFirst(RegExp(r'^https?://'), ''),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: TextStyle(fontSize: 11, color: fg.withValues(alpha: 0.5)),
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
            visualDensity: VisualDensity.comfortable,
            icon: Icon(Icons.delete_outline_rounded, size: 17, color: Colors.redAccent.withValues(alpha: 0.7)),
            onPressed: () => ref.read(connProvider.notifier).remove(p.id),
          ),
        ],
      ),
      onTap: () => ref.read(connProvider.notifier).activate(p.id),
    );
  }

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
                    style: FilledButton.styleFrom(backgroundColor: kPrimary, foregroundColor: Colors.white),
                    onPressed: () {
                      final u = url.text.trim().replaceAll(RegExp(r'/+$'), '');
                      if (!RegExp(r'^https?://.+$').hasMatch(u)) {
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
