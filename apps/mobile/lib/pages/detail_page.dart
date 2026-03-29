/// 详情页：全文 Markdown 渲染 + 编辑/置顶/删除（可撤回）/分享出去
library;

import 'package:flutter/material.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:share_plus/share_plus.dart';

import '../conn/models.dart';
import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';
import '../state/memos_state.dart';
import 'edit_sheet.dart';

class DetailPage extends ConsumerStatefulWidget {
  const DetailPage({super.key, required this.memoId});

  final String memoId;

  @override
  ConsumerState<DetailPage> createState() => _DetailPageState();
}

class _DetailPageState extends ConsumerState<DetailPage> {
  Memo? _memo;
  String? _error;

  @override
  void initState() {
    super.initState();
    // 列表里通常已有：先本地命中再拉远端兜底
    final local = ref.read(memosProvider).items.where((m) => m.shortId == widget.memoId).firstOrNull;
    if (local != null) {
      _memo = local;
    } else {
      _load();
    }
  }

  Future<void> _load() async {
    final client = ref.read(connProvider).client;
    if (client == null) return;
    try {
      final m = await client.getMemo(widget.memoId);
      setState(() => _memo = m);
    } catch (e) {
      setState(() => _error = '$e');
    }
  }

  Future<void> _togglePin() async {
    final client = ref.read(connProvider).client;
    final m = _memo;
    if (client == null || m == null) return;
    try {
      final updated = await client.togglePinned(m.shortId);
      setState(() => _memo = updated);
      ref.read(memosProvider.notifier).upsert(updated);
    } catch (_) {
      _toast('操作失败');
    }
  }

  Future<void> _delete() async {
    final client = ref.read(connProvider).client;
    final m = _memo;
    if (client == null || m == null) return;

    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('删除这条备忘？'),
        content: Text(m.title.isEmpty ? _excerpt(m.content) : m.title, maxLines: 2, overflow: TextOverflow.ellipsis),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          TextButton(onPressed: () => Navigator.pop(ctx, true), child: const Text('删除', style: TextStyle(color: Colors.redAccent))),
        ],
      ),
    );
    if (ok != true) return;

    try {
      await client.deleteMemo(m.shortId);
      ref.read(memosProvider.notifier).removeLocal(m.shortId);
      if (!mounted) return;
      // 撤回：调恢复接口 + 回列表（F04）
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: const Text('已删除'),
          action: SnackBarAction(
            label: '撤回',
            onPressed: () async {
              try {
                await ref.read(connProvider).client?.restoreMemo(m.shortId);
                final restored = await ref.read(connProvider).client?.getMemo(m.shortId);
                if (restored != null) ref.read(memosProvider.notifier).upsert(restored);
              } catch (_) {
                _toast('撤回失败，可在桌面端回收站恢复');
              }
            },
          ),
        ),
      );
      Navigator.pop(context);
    } catch (_) {
      _toast('删除失败');
    }
  }

  void _shareOut() {
    final m = _memo;
    if (m == null) return;
    // 系统分享：纯文本（标题 + 正文）
    final text = m.title.isEmpty ? m.content : '${m.title}\n\n${m.content}';
    Share.share(text);
  }

  void _toast(String msg) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg)));
  }

  String _excerpt(String s) => s.length > 60 ? s.substring(0, 60) : s;

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    final m = _memo;

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
          child: m == null
              ? Center(
                  child: _error != null
                      ? Text('加载失败：$_error', style: const TextStyle(fontSize: 13))
                      : const CircularProgressIndicator(strokeWidth: 2),
                )
              : Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // 顶栏
                    Padding(
                      padding: const EdgeInsets.fromLTRB(8, 4, 8, 0),
                      child: Row(
                        children: [
                          IconButton(
                            icon: const Icon(Icons.arrow_back_rounded),
                            onPressed: () => Navigator.pop(context),
                          ),
                          const Spacer(),
                          IconButton(
                            icon: Icon(m.pinned ? Icons.push_pin_rounded : Icons.push_pin_outlined, size: 20),
                            onPressed: _togglePin,
                          ),
                          IconButton(
                            icon: const Icon(Icons.ios_share_rounded, size: 20),
                            onPressed: _shareOut,
                          ),
                          IconButton(
                            icon: const Icon(Icons.edit_outlined, size: 20),
                            onPressed: () => showEditSheet(context, ref, memo: m),
                          ),
                          IconButton(
                            icon: const Icon(Icons.delete_outline_rounded, size: 20, color: Colors.redAccent),
                            onPressed: _delete,
                          ),
                        ],
                      ),
                    ),
                    // 元信息
                    Padding(
                      padding: const EdgeInsets.fromLTRB(20, 8, 20, 0),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          if (m.title.isNotEmpty)
                            Text(
                              m.title,
                              style: TextStyle(fontSize: 20, fontWeight: FontWeight.w800, height: 1.35, color: fg),
                            ),
                          const SizedBox(height: 6),
                          Row(
                            children: [
                              Text(
                                _fmtTime(m.createdAt),
                                style: TextStyle(fontSize: 11, color: Theme.of(context).hintColor),
                              ),
                              for (final t in m.tags.take(6)) ...[
                                const SizedBox(width: 8),
                                Text('#$t', style: TextStyle(fontSize: 11, color: kAccent.withValues(alpha: 0.9))),
                              ],
                            ],
                          ),
                        ],
                      ),
                    ),
                    // 正文（实体垫底卡片：可读性红线）
                    Expanded(
                      child: GlassContainer(
                        tier: GlassTier.flat,
                        preference: ref.watch(glassPrefProvider),
                        borderRadius: BorderRadius.circular(20),
                        margin: const EdgeInsets.fromLTRB(16, 12, 16, 24),
                        padding: const EdgeInsets.all(16),
                        child: SingleChildScrollView(
                          child: MarkdownBody(
                            data: m.content,
                            selectable: true,
                            styleSheet: _mdStyle(context, fg),
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
        ),
      ),
    );
  }

  MarkdownStyleSheet _mdStyle(BuildContext context, Color fg) {
    final base = MarkdownStyleSheet.fromTheme(Theme.of(context));
    return base.copyWith(
      p: TextStyle(fontSize: 14.5, height: 1.75, color: fg),
      h1: TextStyle(fontSize: 20, fontWeight: FontWeight.w800, color: fg),
      h2: TextStyle(fontSize: 17.5, fontWeight: FontWeight.w700, color: fg),
      h3: TextStyle(fontSize: 15.5, fontWeight: FontWeight.w700, color: fg),
      listBullet: TextStyle(fontSize: 14.5, color: fg),
      code: TextStyle(fontSize: 12.5, color: fg.withValues(alpha: 0.9), backgroundColor: Theme.of(context).brightness == Brightness.dark ? Colors.white10 : Colors.black.withValues(alpha: 0.05)),
      blockquote: TextStyle(fontSize: 14, color: fg.withValues(alpha: 0.75)),
      blockquoteDecoration: BoxDecoration(
        border: Border(left: BorderSide(width: 3, color: kPrimary.withValues(alpha: 0.5))),
      ),
    );
  }

  String _fmtTime(String iso) {
    final dt = DateTime.tryParse(iso);
    return dt == null ? '' : '${dt.year}-${dt.month.toString().padLeft(2, '0')}-${dt.day.toString().padLeft(2, '0')} ${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
  }
}
