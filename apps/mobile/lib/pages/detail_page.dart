/// 详情页：全文 Markdown 渲染 + 交互勾选 + AI 抽待办 + 附件图片查看 + 编辑/置顶/删除（可撤回）/分享出去
library;

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:share_plus/share_plus.dart';

import '../conn/models.dart';
import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';
import '../state/memos_state.dart';
import '../util/checklist.dart';
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
    final barTitle = m == null ? '' : (m.title.isEmpty ? _excerpt(m.content) : m.title);

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
          child: m == null
              ? Center(
                  child: _error != null
                      ? Text('加载失败：$_error', style: const TextStyle(fontSize: 13))
                      : const CircularProgressIndicator(strokeWidth: 2),
                )
              : CustomScrollView(
                  physics: const ClampingScrollPhysics(),
                  slivers: [
                    // 玻璃吸顶条：内容滚动时从模糊层下方穿过
                    SliverAppBar(
                      pinned: true,
                      elevation: 0,
                      scrolledUnderElevation: 0,
                      backgroundColor: Colors.transparent,
                      foregroundColor: fg,
                      title: Text(
                        barTitle,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600, color: fg),
                      ),
                      leading: IconButton(
                        icon: const Icon(Icons.arrow_back_rounded),
                        onPressed: () => Navigator.pop(context),
                      ),
                      actions: [
                        IconButton(
                          tooltip: m.pinned ? '取消置顶' : '置顶',
                          icon: Icon(m.pinned ? Icons.push_pin_rounded : Icons.push_pin_outlined, size: 20),
                          onPressed: _togglePin,
                        ),
                        PopupMenuButton<String>(
                          icon: const Icon(Icons.more_horiz_rounded, size: 22),
                          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(14)),
                          position: PopupMenuPosition.under,
                          onSelected: (v) {
                            switch (v) {
                              case 'edit':
                                showEditSheet(context, ref, memo: m);
                              case 'todo':
                                _aiExtract(m);
                              case 'share':
                                _shareOut();
                              case 'delete':
                                _delete();
                            }
                          },
                          itemBuilder: (_) => [
                            const PopupMenuItem(value: 'edit', child: Text('编辑', style: TextStyle(fontSize: 14))),
                            const PopupMenuItem(value: 'todo', child: Text('AI 抽待办', style: TextStyle(fontSize: 14))),
                            const PopupMenuItem(value: 'share', child: Text('分享出去', style: TextStyle(fontSize: 14))),
                            const PopupMenuItem(
                              value: 'delete',
                              child: Text('删除', style: TextStyle(fontSize: 14, color: Colors.redAccent)),
                            ),
                          ],
                        ),
                      ],
                      flexibleSpace: GlassContainer(
                        tier: GlassTier.regular,
                        preference: ref.watch(glassPrefProvider),
                        borderRadius: BorderRadius.zero,
                        padding: EdgeInsets.zero,
                        child: const SizedBox.expand(),
                      ),
                    ),

                    // 元信息（滚出视口）
                    SliverPadding(
                      padding: const EdgeInsets.fromLTRB(20, 10, 20, 0),
                      sliver: SliverToBoxAdapter(
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
                    ),

                    // 附件条（F09：图片缩略图，点击全屏）
                    SliverToBoxAdapter(
                      child: FutureBuilder<List<AttachmentDto>>(
                        future: _loadAttachments(),
                        builder: (context, snap) {
                          final images = (snap.data ?? []).where((a) => a.isImage).toList();
                          if (images.isEmpty) return const SizedBox.shrink();
                          return Padding(
                            padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
                            child: SizedBox(
                              height: 86,
                              child: ListView.separated(
                                scrollDirection: Axis.horizontal,
                                itemCount: images.length,
                                separatorBuilder: (_, _) => const SizedBox(width: 8),
                                itemBuilder: (_, i) => _AttThumb(att: images[i]),
                              ),
                            ),
                          );
                        },
                      ),
                    ),

                    // 正文（实体垫底卡片：可读性红线；勾选框可点按写回）
                    SliverPadding(
                      padding: const EdgeInsets.fromLTRB(16, 12, 16, 36),
                      sliver: SliverToBoxAdapter(
                        child: GlassContainer(
                          tier: GlassTier.flat,
                          preference: ref.watch(glassPrefProvider),
                          borderRadius: BorderRadius.circular(20),
                          padding: const EdgeInsets.all(16),
                          child: Builder(builder: (context) {
                            // checkboxBuilder 只给 bool：按构建顺序自计数定位第 n 个框
                            final order = _CheckboxOrder();
                            return MarkdownBody(
                              data: m.content,
                              selectable: true,
                              styleSheet: _mdStyle(context, fg),
                              checkboxBuilder: (checked) => _tapCheckbox(
                                context,
                                m,
                                fg,
                                checked,
                                order.next(),
                              ),
                            );
                          }),
                        ),
                      ),
                    ),
                  ],
                ),
        ),
      ),
    );
  }

  Future<List<AttachmentDto>> _loadAttachments() async {
    final client = ref.read(connProvider).client;
    if (client == null) return [];
    try {
      return await client.listAttachments(widget.memoId);
    } catch (_) {
      return [];
    }
  }

  /// AI 抽待办：把备忘里的行动项抽成勾选清单，确认后追加「## 待办」段 + #待办 标签
  Future<void> _aiExtract(Memo m) async {
    final client = ref.read(connProvider).client;
    if (client == null) {
      _toast('离线状态不可用');
      return;
    }
    final buf = ValueNotifier<String>('');
    final done = ValueNotifier<bool>(false);
    final err = ValueNotifier<String?>(null);
    unawaited(
      client
          .aiChatStream(
        [
          {'role': 'user', 'content': m.content},
        ],
        system: '从下面的备忘内容里抽取可执行的行动项（待办）。'
            '只输出 Markdown 任务列表（每行一个「- [ ] 事项」），不要标题、不要解释、不要代码块围栏。'
            '没有行动项就只输出「- [ ] （未发现行动项）」。内容：\n\n${m.content}',
        onEvent: (event, data) {
          if (event == 'delta') buf.value += data['text'] as String? ?? '';
          if (event == 'error') err.value = data['message'] as String? ?? 'AI 出错';
        },
      )
          .whenComplete(() => done.value = true),
    );

    if (!mounted) return;
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    await showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      builder: (ctx) => _AiTodoSheet(buf: buf, done: done, err: err, fg: fg, onConfirm: (list) async {
        Navigator.pop(ctx);
        final tags = {...m.tags, '待办'}.toList();
        final updated = await client.updateMemo(
          m.shortId,
          content: appendChecklist(m.content, list),
          tags: tags,
        );
        ref.read(memosProvider.notifier).upsert(updated);
      }),
    );
  }

  /// 勾选框渲染 + 点按写回（n = 文档顺序第几个框）
  Widget _tapCheckbox(BuildContext context, Memo m, Color fg, bool checked, int n) {
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () async {
        final client = ref.read(connProvider).client;
        if (client == null) return;
        final next = toggleCheckbox(m.content, n);
        if (next == m.content) return;
        try {
          final updated = await client.updateMemo(m.shortId, content: next);
          ref.read(memosProvider.notifier).upsert(updated);
        } catch (_) {
          if (mounted) {
            ScaffoldMessenger.of(this.context).showSnackBar(
              const SnackBar(content: Text('勾选未保存（连接不可用？）'), duration: Duration(seconds: 2)),
            );
          }
        }
      },
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 2),
        child: Icon(
          checked ? Icons.check_box_rounded : Icons.check_box_outline_blank_rounded,
          size: 17,
          color: checked ? kPrimary : fg.withValues(alpha: 0.55),
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

/// 勾选框构建顺序计数器（MarkdownBody 每次构建文档序遍历，与源文行序一致）
class _CheckboxOrder {
  int _n = 0;
  int next() => _n++;
}

/// AI 抽待办弹层：流式预览 → 确认追加
class _AiTodoSheet extends StatelessWidget {
  const _AiTodoSheet({
    required this.buf,
    required this.done,
    required this.err,
    required this.fg,
    required this.onConfirm,
  });

  final ValueNotifier<String> buf;
  final ValueNotifier<bool> done;
  final ValueNotifier<String?> err;
  final Color fg;
  final Future<void> Function(String list) onConfirm;

  /// 剥掉模型偶尔加的代码块围栏
  static String _stripFences(String s) {
    var t = s.trim();
    if (t.startsWith('```')) {
      t = t.replaceFirst(RegExp(r'^```[a-zA-Z]*\n'), '');
      if (t.endsWith('```')) t = t.substring(0, t.length - 3);
    }
    return t.trim();
  }

  @override
  Widget build(BuildContext context) {
    return GlassContainer(
      tier: GlassTier.thick,
      preference: GlassPreference.auto,
      borderRadius: const BorderRadius.vertical(top: Radius.circular(24)),
      padding: const EdgeInsets.all(20),
      child: SafeArea(
        top: false,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('AI 抽待办', style: TextStyle(fontSize: 15.5, fontWeight: FontWeight.w700, color: fg)),
            const SizedBox(height: 4),
            Text('确认后追加到「## 待办」段并加 #待办 标签', style: TextStyle(fontSize: 11, color: fg.withValues(alpha: 0.5))),
            const SizedBox(height: 14),
            Flexible(
              child: AnimatedBuilder(
                animation: Listenable.merge([buf, done, err]),
                builder: (context, _) {
                  final list = _stripFences(buf.value);
                  final finished = done.value;
                  final error = err.value;
                  if (error != null) {
                    return Padding(
                      padding: const EdgeInsets.symmetric(vertical: 18),
                      child: Text('生成失败：$error', style: const TextStyle(fontSize: 12.5, color: Color(0xFFFF6B6B))),
                    );
                  }
                  if (!finished && list.isEmpty) {
                    return const Padding(
                      padding: EdgeInsets.symmetric(vertical: 18),
                      child: Center(child: SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))),
                    );
                  }
                  return SingleChildScrollView(
                    child: Text(
                      list.isEmpty ? '（无输出）' : '$list${finished ? '' : ' ▌'}',
                      style: TextStyle(fontSize: 13, height: 1.7, color: fg),
                    ),
                  );
                },
              ),
            ),
            const SizedBox(height: 14),
            ValueListenableBuilder<bool>(
              valueListenable: done,
              builder: (context, finished, _) => Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  TextButton(onPressed: () => Navigator.pop(context), child: const Text('取消')),
                  const SizedBox(width: 8),
                  FilledButton(
                    style: FilledButton.styleFrom(backgroundColor: kPrimary, foregroundColor: Colors.white),
                    onPressed: finished && err.value == null && buf.value.trim().isNotEmpty
                        ? () => onConfirm(_stripFences(buf.value))
                        : null,
                    child: Text(finished ? '追加到备忘' : '生成中…'),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// 附件缩略图（点击全屏 InteractiveViewer）
class _AttThumb extends ConsumerWidget {
  const _AttThumb({required this.att});

  final AttachmentDto att;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return FutureBuilder<Uint8List>(
      future: ref.read(connProvider).client?.downloadAttachment(att.id),
      builder: (context, snap) {
        final bytes = snap.data;
        final thumb = bytes == null
            ? Container(
                width: 86,
                height: 86,
                decoration: BoxDecoration(
                  color: Theme.of(context).hintColor.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(12),
                ),
                child: const Center(child: SizedBox(width: 16, height: 16, child: CircularProgressIndicator(strokeWidth: 2))),
              )
            : GestureDetector(
                onTap: () => _openViewer(context, bytes),
                onLongPress: () => _share(context, bytes),
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(12),
                  child: Image.memory(bytes, width: 86, height: 86, fit: BoxFit.cover),
                ),
              );
        return thumb;
      },
    );
  }

  /// 附件图片转发出去（系统分享，走 XFile 临时文件语义）
  Future<void> _share(BuildContext context, Uint8List bytes) async {
    await Share.shareXFiles(
      [XFile.fromData(bytes, name: att.filename, mimeType: att.mimeType)],
      subject: att.filename,
    );
  }

  void _openViewer(BuildContext context, Uint8List bytes) {
    Navigator.of(context).push(PageRouteBuilder(
      opaque: false,
      barrierColor: Colors.black87,
      pageBuilder: (_, _, _) => Scaffold(
        backgroundColor: Colors.transparent,
        body: GestureDetector(
          onTap: () => Navigator.pop(context),
          child: Stack(
            children: [
              Center(
                child: InteractiveViewer(
                  maxScale: 5,
                  child: Image.memory(bytes),
                ),
              ),
              // 右上：转发出去
              SafeArea(
                child: Align(
                  alignment: Alignment.topRight,
                  child: Padding(
                    padding: const EdgeInsets.all(14),
                    child: GestureDetector(
                      onTap: () => _share(context, bytes),
                      child: Container(
                        width: 38,
                        height: 38,
                        decoration: BoxDecoration(color: Colors.black45, shape: BoxShape.circle),
                        child: const Icon(Icons.ios_share_rounded, size: 18, color: Colors.white),
                      ),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    ));
  }
}
