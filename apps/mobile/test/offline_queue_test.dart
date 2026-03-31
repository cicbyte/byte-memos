import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:byte_memos_mobile/conn/client.dart';
import 'package:byte_memos_mobile/state/conn_state.dart' show sharedPrefsProvider;
import 'package:byte_memos_mobile/state/offline_queue.dart';

void main() {
  setUp(() {
    SharedPreferences.setMockInitialValues({});
  });

  test('enqueue 持久化并跨容器恢复', () async {
    // 容器 1：入队 2 条
    final c1 = ProviderContainer();
    addTearDown(c1.dispose);
    await c1.read(offlineQueueProvider.notifier).enqueue(
          const PendingMemo(content: '离线一', title: 't1', tags: ['a']),
        );
    await c1.read(offlineQueueProvider.notifier).enqueue(
          const PendingMemo(content: '离线二', createdAt: '2026-10-16T00:00:00'),
        );
    expect(c1.read(offlineQueueProvider).length, 2);

    // 容器 2：重新加载（模拟 app 重启）——等 prefs 就绪 + _load 微任务追平
    final c2 = ProviderContainer();
    addTearDown(c2.dispose);
    await c2.read(sharedPrefsProvider.future);
    var restored = c2.read(offlineQueueProvider);
    for (var i = 0; i < 50 && restored.isEmpty; i++) {
      await Future<void>.delayed(const Duration(milliseconds: 5));
      restored = c2.read(offlineQueueProvider);
    }
    expect(restored.length, 2);
    expect(restored[0].content, '离线一');
    expect(restored[0].tags, ['a']);
    expect(restored[1].content, '离线二');
    expect(restored[1].createdAt, '2026-10-16T00:00:00');
  });

  test('toJson/fromJson round-trip', () {
    const m = PendingMemo(content: 'c', title: 't', tags: ['x', 'y'], createdAt: 'now');
    final back = PendingMemo.fromJson(m.toJson());
    expect(back.content, 'c');
    expect(back.title, 't');
    expect(back.tags, ['x', 'y']);
    expect(back.createdAt, 'now');
  });

  test('空队列 flush 返回 0', () async {
    final c = ProviderContainer();
    addTearDown(c.dispose);
    // 无 client 场景下空队列直接短路
    final n = await c.read(offlineQueueProvider.notifier).flush(_FakeClient());
    expect(n, 0);
  });
}

/// 假 client：仅用于空队列短路测试（不会真正发出请求）
class _FakeClient extends BmClient {
  _FakeClient() : super(baseUrl: 'http://localhost:1');
}
