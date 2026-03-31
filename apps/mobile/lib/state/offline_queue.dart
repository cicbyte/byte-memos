/// 离线速记队列（F15）：断网新建备忘落本地，联网自动补传
///
/// - 持久化：SharedPreferences 单键 JSON（速记为纯文本，量级可控）
/// - 补传：connProvider 转 online 时 flush（shell 根监听），逐条 addMemo
///   成功即移除；任一失败即停（视为仍离线），下次再试
library;

import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../conn/client.dart';
import 'conn_state.dart';
import 'memos_state.dart';

/// 待同步备忘（离线速记快照）
class PendingMemo {
  const PendingMemo({required this.content, this.title = '', this.tags = const [], this.createdAt = ''});

  final String content;
  final String title;
  final List<String> tags;
  final String createdAt;

  Map<String, dynamic> toJson() => {'content': content, 'title': title, 'tags': tags, 'created_at': createdAt};

  factory PendingMemo.fromJson(Map<String, dynamic> j) => PendingMemo(
        content: j['content'] as String? ?? '',
        title: j['title'] as String? ?? '',
        tags: (j['tags'] as List?)?.map((e) => e.toString()).toList() ?? const [],
        createdAt: j['created_at'] as String? ?? '',
      );
}

final offlineQueueProvider = StateNotifierProvider<OfflineQueueNotifier, List<PendingMemo>>((ref) {
  return OfflineQueueNotifier(ref);
});

class OfflineQueueNotifier extends StateNotifier<List<PendingMemo>> {
  OfflineQueueNotifier(this._ref) : super(const []) {
    _load();
  }

  static const _key = 'bm.offline.queue';

  final Ref _ref;

  Future<void> _load() async {
    try {
      final prefs = await _ref.read(sharedPrefsProvider.future);
      final raw = prefs.getString(_key);
      if (raw == null || raw.isEmpty) return;
      final list = (jsonDecode(raw) as List).map((e) => PendingMemo.fromJson(Map<String, dynamic>.from(e))).toList();
      state = list;
    } catch (_) {}
  }

  Future<void> _persist() async {
    try {
      final prefs = await _ref.read(sharedPrefsProvider.future);
      await prefs.setString(_key, jsonEncode(state.map((m) => m.toJson()).toList()));
    } catch (_) {}
  }

  /// 入队（离线保存）
  Future<void> enqueue(PendingMemo m) async {
    state = [...state, m];
    await _persist();
  }

  /// 补传：返回成功条数；队列耗尽返回同步总数，中途失败返回 -1（仍离线）
  Future<int> flush(BmClient client) async {
    if (state.isEmpty) return 0;
    var synced = 0;
    for (final m in state) {
      try {
        final created = await client.addMemo(content: m.content, title: m.title, tags: m.tags);
        _ref.read(memosProvider.notifier).upsert(created);
        synced++;
        state = [...state]..removeWhere((e) => identical(e, m));
      } catch (_) {
        await _persist();
        return -1;
      }
    }
    await _persist();
    return synced;
  }
}
