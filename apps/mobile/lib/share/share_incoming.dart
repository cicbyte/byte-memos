/// 分享收纳（F06）：接收 Android ACTION_SEND text/plain
///
/// 平台通道：MainActivity 在 onCreate/onNewIntent 里把 EXTRA_TEXT 发过来；
/// Dart 侧单订阅回调 → 弹预填的新建弹层。
library;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../pages/edit_sheet.dart';

typedef ShareTextHandler = void Function(String text);

ShareTextHandler? _handler;

/// 绑定分享回调（main 启动时调用一次）
void bindShareIncoming(ShareTextHandler onText) {
  _handler = onText;
  const channel = MethodChannel('byte_memos/share');
  channel.setMethodCallHandler((call) async {
    if (call.method == 'shareText' && call.arguments is String) {
      _handler?.call(call.arguments as String);
    }
  });
  // 冷启动：拉取 Activity 暂存的分享内容
  channel.invokeMethod<dynamic>('takePendingShare').then((v) {
    if (v is String && v.trim().isNotEmpty) _handler?.call(v);
  });
}

/// 弹出预填的编辑弹层（附来源说明行）
void showShareSheet(BuildContext context, WidgetRef ref, String text) {
  showEditSheet(context, ref, prefill: '$text\n\n> 来自手机分享');
}
