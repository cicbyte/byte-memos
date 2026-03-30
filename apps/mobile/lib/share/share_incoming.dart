/// 分享收纳（F06 文本 / F14 图片）：接收 Android ACTION_SEND
///
/// 平台通道：MainActivity 在 onCreate/onNewIntent 里把 EXTRA_TEXT / 图片流发过来；
/// Dart 侧单订阅回调 → 文本弹预填弹层，图片弹「收纳为附件」弹层。
library;

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';
import '../state/memos_state.dart';
import '../pages/edit_sheet.dart' show showEditSheet;

typedef ShareTextHandler = void Function(String text);
typedef ShareImageHandler = void Function(SharedImage image);

/// 平台层送来的图片（base64 解码后使用）
class SharedImage {
  const SharedImage({required this.name, required this.mime, required this.bytes});

  final String name;
  final String mime;
  final Uint8List bytes;
}

ShareTextHandler? _textHandler;
ShareImageHandler? _imageHandler;

/// 绑定分享回调（main 启动时调用一次）
void bindShareIncoming({ShareTextHandler? onText, ShareImageHandler? onImage}) {
  _textHandler = onText;
  _imageHandler = onImage;
  const channel = MethodChannel('byte_memos/share');
  channel.setMethodCallHandler((call) async {
    switch (call.method) {
      case 'shareText' when call.arguments is String:
        _textHandler?.call(call.arguments as String);
      case 'shareImage' when call.arguments is Map:
        final args = Map<String, dynamic>.from(call.arguments as Map);
        final data = args['data'] as String?;
        if (data != null) {
          _imageHandler?.call(SharedImage(
            name: args['name'] as String? ?? 'shared.png',
            mime: args['mime'] as String? ?? 'image/png',
            bytes: const Base64Decoder().convert(data),
          ));
        }
    }
  });
  // 冷启动：拉取 Activity 暂存的分享内容
  channel.invokeMethod<dynamic>('takePendingShare').then((v) {
    if (v is String && v.trim().isNotEmpty) {
      _textHandler?.call(v);
    } else if (v is Map) {
      final args = Map<String, dynamic>.from(v);
      final data = args['data'] as String?;
      if (data != null) {
        _imageHandler?.call(SharedImage(
          name: args['name'] as String? ?? 'shared.png',
          mime: args['mime'] as String? ?? 'image/png',
          bytes: const Base64Decoder().convert(data),
        ));
      }
    }
  });
}

/// 弹出预填的编辑弹层（附来源说明行）
void showShareSheet(BuildContext context, WidgetRef ref, String text) {
  showEditSheet(context, ref, prefill: '$text\n\n> 来自手机分享');
}

/// 图片收纳弹层（F14）：预览 + 一键存为新备忘（图片挂为附件）
void showImageShareSheet(BuildContext context, WidgetRef ref, SharedImage img) {
  final dark = Theme.of(context).brightness == Brightness.dark;
  final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
  final controller = TextEditingController();
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
            Text('收纳图片', style: TextStyle(fontSize: 15.5, fontWeight: FontWeight.w700, color: fg)),
            const SizedBox(height: 12),
            Center(
              child: ClipRRect(
                borderRadius: BorderRadius.circular(12),
                child: Image.memory(img.bytes, width: 170, fit: BoxFit.contain),
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: controller,
              maxLines: 1,
              style: TextStyle(fontSize: 13, color: fg),
              decoration: InputDecoration(
                hintText: '备注（可空，将作为备忘内容）',
                hintStyle: TextStyle(fontSize: 12.5, color: dark ? Colors.white38 : Colors.black38),
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 11),
                border: OutlineInputBorder(borderRadius: BorderRadius.circular(10)),
              ),
            ),
            const SizedBox(height: 14),
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('取消')),
                const SizedBox(width: 8),
                FilledButton(
                  style: FilledButton.styleFrom(backgroundColor: kPrimary, foregroundColor: Colors.white),
                  onPressed: () async {
                    final client = ref.read(connProvider).client;
                    if (client == null) return;
                    Navigator.pop(ctx);
                    final note = controller.text.trim();
                    try {
                      // 先建备忘（内容=备注或占位），再挂附件
                      final memo = await client.addMemo(
                        content: note.isEmpty ? '![${img.name}](${img.name})' : note,
                        title: note.isEmpty ? '图片 · ${img.name}' : note,
                        tags: const ['图片收纳'],
                      );
                      await client.uploadAttachment(
                        memo.shortId,
                        filename: img.name,
                        mimeType: img.mime,
                        bytes: img.bytes,
                      );
                      ref.read(memosProvider.notifier).upsert(memo);
                    } catch (_) {
                      if (context.mounted) {
                        ScaffoldMessenger.of(context)
                            .showSnackBar(const SnackBar(content: Text('收纳失败（连接不可用？）')));
                      }
                    }
                  },
                  child: const Text('存为备忘'),
                ),
              ],
            ),
          ],
        ),
      ),
    ),
  );
}
