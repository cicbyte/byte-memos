/// 新建/编辑玻璃弹层（thick 材质模态）
///
/// - 纯文本输入 + Markdown 预览切换
/// - 常用格式快捷条（标题/列表/加粗/代码/引用）+ 相册选图（附件）
/// - 标题 / 分类 / 标签
/// - 编辑已有备忘时传入 [memo]
/// - [prefill]：分享收纳（F06）预填内容
library;

import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:image_picker/image_picker.dart';

import '../conn/models.dart';
import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';
import '../state/memos_state.dart';
import '../state/offline_queue.dart';

/// 打开新建/编辑弹层（全局入口）
void showEditSheet(BuildContext context, WidgetRef ref, {Memo? memo, String? prefill, String? prefillTitle}) {
  showModalBottomSheet(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (_) => _EditSheet(memo: memo, prefill: prefill, prefillTitle: prefillTitle),
  );
}

class _EditSheet extends ConsumerStatefulWidget {
  const _EditSheet({this.memo, this.prefill, this.prefillTitle});

  final Memo? memo;
  final String? prefill;
  final String? prefillTitle;

  @override
  ConsumerState<_EditSheet> createState() => _EditSheetState();
}

class _EditSheetState extends ConsumerState<_EditSheet> {
  late final _content = TextEditingController(text: widget.memo?.content ?? widget.prefill ?? '');
  late final _title = TextEditingController(text: widget.memo?.title ?? widget.prefillTitle ?? '');
  late final _tags = TextEditingController(text: widget.memo?.tags.join(', '));

  String? _category;
  bool _preview = false;
  bool _saving = false;
  final _focus = FocusNode();

  /// 待附加图片（编辑弹层收尾：相册选图 → 保存时随备忘上传）
  final List<XFile> _picks = [];

  @override
  void initState() {
    super.initState();
    _category = widget.memo?.category;
    // 焦点进内容框：速记场景键盘直接弹出
    WidgetsBinding.instance.addPostFrameCallback((_) => _focus.requestFocus());
  }

  @override
  void dispose() {
    _content.dispose();
    _title.dispose();
    _tags.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _insertAround(String before, [String after = '']) {
    final sel = _content.selection;
    final text = _content.text;
    if (!sel.isValid) return;
    final newText = text.replaceRange(sel.start, sel.end, '$before${text.substring(sel.start, sel.end)}$after');
    _content.value = TextEditingValue(
      text: newText,
      selection: TextSelection.collapsed(offset: sel.start + before.length + (sel.end - sel.start)),
    );
    _focus.requestFocus();
  }

  Future<void> _pickImages() async {
    final picker = ImagePicker();
    try {
      final files = await picker.pickMultiImage(imageQuality: 85, limit: 6);
      if (files.isNotEmpty) setState(() => _picks.addAll(files.take(6 - _picks.length)));
    } catch (_) {
      _toast('选图失败');
    }
  }

  /// 图片后缀推断 mime（image_picker 在部分 ROM 不给 mimeType）
  String _mimeOf(XFile f) {
    final mt = f.mimeType;
    if (mt != null && mt.startsWith('image/')) return mt;
    final ext = f.path.split('.').last.toLowerCase();
    return switch (ext) {
      'png' => 'image/png',
      'gif' => 'image/gif',
      'webp' => 'image/webp',
      'bmp' => 'image/bmp',
      _ => 'image/jpeg',
    };
  }

  Future<void> _save() async {
    if (_saving) return;
    final rawContent = _content.text.trim();
    if (rawContent.isEmpty && _picks.isEmpty) return;
    // 只附图不写字：占位正文
    final content = rawContent.isEmpty ? '（图片×${_picks.length}）' : rawContent;
    setState(() => _saving = true);

    final client = ref.read(connProvider).client;
    final tags = _tags.text.split(RegExp(r'[,，]')).map((e) => e.trim().replaceAll('#', '')).where((e) => e.isNotEmpty).toList();
    // 离线兜底（F15）：新建入离线队列，联网自动补传；编辑已有备忘离线不支持
    if (client == null) {
      if (widget.memo == null) {
        await ref.read(offlineQueueProvider.notifier).enqueue(PendingMemo(
              content: content,
              title: _title.text.trim(),
              tags: tags,
              createdAt: DateTime.now().toIso8601String(),
            ));
        if (mounted) Navigator.pop(context);
        _toast(_picks.isNotEmpty ? '离线已存（图片需联网后再加），联网自动同步' : '离线已存，联网后自动同步');
      } else {
        _toast('离线状态，编辑需连接桌面端');
      }
      setState(() => _saving = false);
      return;
    }

    try {
      final Memo m;
      if (widget.memo == null) {
        m = await client.addMemo(
          content: content,
          title: _title.text.trim(),
          category: _category,
          tags: tags,
        );
      } else {
        m = await client.updateMemo(
          widget.memo!.shortId,
          content: content,
          title: _title.text.trim(),
          category: _category,
          tags: tags,
        );
      }
      // 附加图片上传（失败不阻断保存，toast 提示）
      var uploadFail = 0;
      for (final f in _picks) {
        try {
          final bytes = await File(f.path).readAsBytes();
          final name = f.name.isEmpty ? 'img-${DateTime.now().millisecondsSinceEpoch}.jpg' : f.name;
          await client.uploadAttachment(m.shortId, filename: name, mimeType: _mimeOf(f), bytes: bytes);
        } catch (_) {
          uploadFail++;
        }
      }
      ref.read(memosProvider.notifier).upsert(m);
      if (mounted) Navigator.pop(context);
      if (uploadFail > 0) _toast('$uploadFail 张图片上传失败');
    } catch (e) {
      // 在线保存失败（网络中断）：新建同样落离线队列
      if (widget.memo == null) {
        await ref.read(offlineQueueProvider.notifier).enqueue(PendingMemo(
              content: content,
              title: _title.text.trim(),
              tags: tags,
              createdAt: DateTime.now().toIso8601String(),
            ));
        if (mounted) Navigator.pop(context);
        _toast('连接中断，已存离线队列，联网后自动同步');
      } else {
        _toast('保存失败：$e');
        setState(() => _saving = false);
      }
    }
  }

  void _toast(String msg) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(msg), duration: const Duration(seconds: 2)));
  }

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final glassPref = ref.watch(glassPrefProvider);
    final cats = ref.watch(categoriesProvider).valueOrNull ?? const <Category>[];
    final bottomInset = MediaQuery.viewInsetsOf(context).bottom;
    final fg = dark ? Colors.white : Colors.black87;
    final fieldBorder = OutlineInputBorder(
      borderRadius: BorderRadius.circular(12),
      borderSide: BorderSide(color: dark ? Colors.white24 : Colors.black12),
    );

    return Padding(
      padding: EdgeInsets.only(bottom: bottomInset),
      child: SizedBox(
        height: MediaQuery.sizeOf(context).height * 0.82,
        child: GlassContainer(
          tier: GlassTier.thick,
          preference: glassPref,
          borderRadius: const BorderRadius.vertical(top: Radius.circular(28)),
          padding: const EdgeInsets.fromLTRB(18, 10, 18, 18),
          child: Column(
            children: [
              // 抓手
              Container(
                width: 36,
                height: 4,
                decoration: BoxDecoration(
                  color: dark ? Colors.white24 : Colors.black12,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
              const SizedBox(height: 12),
              Row(
                children: [
                  Expanded(
                    child: Text(
                      widget.memo == null ? '新建备忘' : '编辑备忘',
                      style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: fg),
                    ),
                  ),
                  // 预览切换
                  GestureDetector(
                    onTap: () => setState(() => _preview = !_preview),
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
                      decoration: BoxDecoration(
                        color: _preview ? (dark ? Colors.white : kPrimary).withValues(alpha: 0.15) : Colors.transparent,
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Icon(Icons.visibility_outlined, size: 14, color: fg),
                          const SizedBox(width: 4),
                          Text('预览', style: TextStyle(fontSize: 12, color: fg)),
                        ],
                      ),
                    ),
                  ),
                  const SizedBox(width: 10),
                  GestureDetector(
                    onTap: _save,
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
                      decoration: BoxDecoration(
                        gradient: const LinearGradient(colors: [kPrimary, kAccent]),
                        borderRadius: BorderRadius.circular(9),
                      ),
                      child: _saving
                          ? const SizedBox(width: 13, height: 13, child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
                          : const Text('保存', style: TextStyle(fontSize: 13, color: Colors.white, fontWeight: FontWeight.w600)),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 14),

              // 标题 + 分类
              Row(
                children: [
                  Expanded(
                    flex: 3,
                    child: TextField(
                      controller: _title,
                      style: TextStyle(color: fg, fontSize: 13.5),
                      decoration: InputDecoration(
                        hintText: '标题（可选）',
                        hintStyle: TextStyle(color: dark ? Colors.white38 : Colors.black38, fontSize: 13),
                        isDense: true,
                        contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
                        border: fieldBorder,
                        enabledBorder: fieldBorder,
                        focusedBorder: fieldBorder,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    flex: 2,
                    child: Container(
                      height: 40,
                      padding: const EdgeInsets.symmetric(horizontal: 10),
                      decoration: BoxDecoration(
                        border: Border.all(color: dark ? Colors.white24 : Colors.black12),
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: DropdownButtonHideUnderline(
                        child: DropdownButton<String?>(
                          value: _category,
                          isExpanded: true,
                          dropdownColor: dark ? const Color(0xFF26282E) : Colors.white,
                          style: TextStyle(fontSize: 12.5, color: fg),
                          hint: Text('分类', style: TextStyle(fontSize: 12.5, color: dark ? Colors.white38 : Colors.black38)),
                          items: [
                            const DropdownMenuItem(value: null, child: Text('无分类', style: TextStyle(fontSize: 12.5))),
                            for (final c in cats)
                              DropdownMenuItem(value: c.name, child: Text(c.name, style: const TextStyle(fontSize: 12.5))),
                          ],
                          onChanged: (v) => setState(() => _category = v),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),

              // 格式快捷条（尾：相册选图）
              SizedBox(
                height: 36,
                child: Row(
                  children: [
                    _fmtBtn(context, 'H', () => _insertAround('\n## ', '')),
                    _fmtBtn(context, '•', () => _insertAround('\n- ', '')),
                    _fmtBtn(context, '☑', () => _insertAround('\n- [ ] ', '')),
                    _fmtBtn(context, 'B', () => _insertAround('**', '**'), bold: true),
                    _fmtBtn(context, '`', () => _insertAround('`', '`'), mono: true),
                    _fmtBtn(context, '❝', () => _insertAround('\n> ', '')),
                    const Spacer(),
                    GestureDetector(
                      onTap: _picks.length >= 6 ? null : _pickImages,
                      child: Container(
                        width: 40,
                        decoration: BoxDecoration(
                          color: kPrimary.withValues(alpha: _picks.length >= 6 ? 0.06 : 0.14),
                          borderRadius: BorderRadius.circular(9),
                        ),
                        child: Stack(
                          alignment: Alignment.center,
                          children: [
                            Icon(Icons.image_rounded, size: 16, color: kPrimary.withValues(alpha: _picks.length >= 6 ? 0.4 : 1)),
                            if (_picks.isNotEmpty)
                              Positioned(
                                right: 2,
                                top: 2,
                                child: Container(
                                  padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                                  decoration: BoxDecoration(color: kPrimary, borderRadius: BorderRadius.circular(7)),
                                  child: Text('${_picks.length}', style: const TextStyle(fontSize: 8.5, color: Colors.white)),
                                ),
                              ),
                          ],
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 8),

              // 待传图片条
              if (_picks.isNotEmpty)
                SizedBox(
                  height: 64,
                  child: ListView.separated(
                    scrollDirection: Axis.horizontal,
                    itemCount: _picks.length,
                    separatorBuilder: (_, _) => const SizedBox(width: 8),
                    itemBuilder: (_, i) => Stack(
                      children: [
                        ClipRRect(
                          borderRadius: BorderRadius.circular(10),
                          child: Image.file(File(_picks[i].path), width: 64, height: 64, fit: BoxFit.cover),
                        ),
                        Positioned(
                          right: -2,
                          top: -2,
                          child: GestureDetector(
                            onTap: () => setState(() => _picks.removeAt(i)),
                            child: Container(
                              width: 18,
                              height: 18,
                              decoration: const BoxDecoration(color: Colors.black54, shape: BoxShape.circle),
                              child: const Icon(Icons.close_rounded, size: 12, color: Colors.white),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              if (_picks.isNotEmpty) const SizedBox(height: 8),

              // 正文 / 预览
              Expanded(
                child: _preview
                    ? Container(
                        width: double.infinity,
                        padding: const EdgeInsets.all(12),
                        decoration: BoxDecoration(
                          color: dark ? Colors.white.withValues(alpha: 0.04) : Colors.white.withValues(alpha: 0.55),
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: MarkdownBody(
                          data: _content.text,
                          selectable: true,
                          styleSheet: MarkdownStyleSheet.fromTheme(Theme.of(context)).copyWith(
                            p: TextStyle(fontSize: 13.5, height: 1.6, color: fg),
                            h2: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: fg),
                          ),
                        ),
                      )
                    : TextField(
                        controller: _content,
                        focusNode: _focus,
                        maxLines: null,
                        expands: true,
                        textAlignVertical: TextAlignVertical.top,
                        style: TextStyle(color: fg, fontSize: 13.5, height: 1.6),
                        decoration: InputDecoration(
                          hintText: '记点什么…（支持 Markdown）',
                          hintStyle: TextStyle(color: dark ? Colors.white38 : Colors.black38),
                          filled: true,
                          fillColor: dark ? Colors.white.withValues(alpha: 0.05) : Colors.white.withValues(alpha: 0.6),
                          contentPadding: const EdgeInsets.all(12),
                          border: fieldBorder,
                          enabledBorder: fieldBorder,
                          focusedBorder: fieldBorder,
                        ),
                      ),
              ),
              const SizedBox(height: 8),

              // 标签
              TextField(
                controller: _tags,
                style: TextStyle(color: fg, fontSize: 12.5),
                decoration: InputDecoration(
                  hintText: '标签（逗号分隔）',
                  hintStyle: TextStyle(color: dark ? Colors.white38 : Colors.black38, fontSize: 12.5),
                  isDense: true,
                  prefixIcon: Icon(Icons.tag_rounded, size: 15, color: dark ? Colors.white38 : Colors.black38),
                  contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 9),
                  border: fieldBorder,
                  enabledBorder: fieldBorder,
                  focusedBorder: fieldBorder,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _fmtBtn(BuildContext context, String label, VoidCallback onTap, {bool bold = false, bool mono = false}) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    return GestureDetector(
      onTap: onTap,
      child: Container(
        width: 40,
        margin: const EdgeInsets.only(right: 6),
        decoration: BoxDecoration(
          color: dark ? Colors.white.withValues(alpha: 0.07) : Colors.white.withValues(alpha: 0.6),
          borderRadius: BorderRadius.circular(9),
        ),
        alignment: Alignment.center,
        child: Text(
          label,
          style: TextStyle(
            fontSize: 14,
            color: dark ? Colors.white : Colors.black87,
            fontWeight: bold ? FontWeight.w800 : FontWeight.w500,
            fontFamily: mono ? 'monospace' : null,
          ),
        ),
      ),
    );
  }
}
