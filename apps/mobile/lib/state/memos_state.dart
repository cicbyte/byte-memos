/// 备忘录流 / 搜索 / 元数据 providers
///
/// - MemosNotifier：首刷 + 无限滚动 + 本地 CRUD 同步 + 离线只读快照（F07）
/// - searchProvider：300ms 防抖
library;

import 'dart:async' show unawaited;
import 'dart:convert';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../conn/client.dart';
import '../conn/models.dart';
import 'conn_state.dart' show ConnState, ConnStatus, connProvider, sharedPrefsProvider;

class MemosState {
  const MemosState({
    this.items = const [],
    this.loading = false,
    this.loadingMore = false,
    this.hasMore = true,
    this.fromSnapshot = false,
    this.lastSyncAt,
  });

  final List<Memo> items;
  final bool loading;
  final bool loadingMore;
  final bool hasMore;

  /// 当前数据是否来自离线快照（只读，顶部横幅提示）
  final bool fromSnapshot;
  final DateTime? lastSyncAt;

  MemosState copyWith({
    List<Memo>? items,
    bool? loading,
    bool? loadingMore,
    bool? hasMore,
    bool? fromSnapshot,
    DateTime? lastSyncAt,
  }) =>
      MemosState(
        items: items ?? this.items,
        loading: loading ?? this.loading,
        loadingMore: loadingMore ?? this.loadingMore,
        hasMore: hasMore ?? this.hasMore,
        fromSnapshot: fromSnapshot ?? this.fromSnapshot,
        lastSyncAt: lastSyncAt ?? this.lastSyncAt,
      );
}

const _kSnapshot = 'bm.snapshot.memos';

class MemosNotifier extends StateNotifier<MemosState> {
  MemosNotifier(this._ref) : super(const MemosState()) {
    // 连接从离线/未配置 → 在线时自动首刷（配置保存成功、切换连接、恢复在线）
    _ref.listen<ConnState>(connProvider, (prev, next) {
      if (next.status == ConnStatus.online && prev?.status != ConnStatus.online) {
        refresh();
      }
    });
  }

  final Ref _ref;

  static const _pageSize = 20;

  BmClient? get _client => _ref.read(connProvider).client;

  /// 首刷：在线拉取并写快照；离线读快照
  Future<void> refresh() async {
    final client = _client;
    if (client == null) {
      await _loadSnapshot();
      return;
    }
    state = state.copyWith(loading: true);
    try {
      final items = await client.listMemos(limit: _pageSize, offset: 0);
      state = MemosState(items: items, hasMore: items.length >= _pageSize, lastSyncAt: DateTime.now());
      unawaited(_saveSnapshot(items));
    } on BmApiException catch (e) {
      if (e.unreachable) {
        _ref.read(connProvider.notifier).markOffline();
        await _loadSnapshot();
      } else {
        state = state.copyWith(loading: false);
      }
    }
  }

  /// 无限滚动
  Future<void> loadMore() async {
    final client = _client;
    if (client == null || state.loadingMore || !state.hasMore || state.fromSnapshot) return;
    state = state.copyWith(loadingMore: true);
    try {
      final more = await client.listMemos(limit: _pageSize, offset: state.items.length);
      state = state.copyWith(
        items: [...state.items, ...more],
        loadingMore: false,
        hasMore: more.length >= _pageSize,
      );
    } on BmApiException catch (e) {
      state = state.copyWith(loadingMore: false);
      if (e.unreachable) _ref.read(connProvider.notifier).markOffline();
    }
  }

  // ===== 本地同步（写操作成功后 UI 即时更新，不整页重刷） =====

  void upsert(Memo m) {
    final items = [...state.items];
    final i = items.indexWhere((x) => x.shortId == m.shortId);
    if (i >= 0) {
      items[i] = m;
    } else {
      items.insert(0, m);
    }
    // 置顶的排最前（服务端排序在下次 refresh 校准）
    items.sort((a, b) {
      if (a.pinned != b.pinned) return a.pinned ? -1 : 1;
      return (b.created ?? DateTime(0)).compareTo(a.created ?? DateTime(0));
    });
    state = state.copyWith(items: items);
  }

  void removeLocal(String id) {
    state = state.copyWith(items: state.items.where((x) => x.shortId != id).toList());
  }

  // ===== 离线快照（F07：最近一次成功响应的只读缓存） =====

  Future<void> _saveSnapshot(List<Memo> items) async {
    try {
      final prefs = await _ref.read(sharedPrefsProvider.future);
      await prefs.setString(
        _kSnapshot,
        jsonEncode({'at': DateTime.now().toIso8601String(), 'items': items.map((m) => m.toJson()).toList()}),
      );
    } catch (_) {}
  }

  Future<void> _loadSnapshot() async {
    try {
      final prefs = await _ref.read(sharedPrefsProvider.future);
      final raw = prefs.getString(_kSnapshot);
      if (raw == null) {
        state = state.copyWith(loading: false, fromSnapshot: true);
        return;
      }
      final data = jsonDecode(raw) as Map<String, dynamic>;
      final items = (data['items'] as List<dynamic>).map((e) => Memo.fromJson(e as Map<String, dynamic>)).toList();
      state = MemosState(items: items, hasMore: false, fromSnapshot: true, lastSyncAt: DateTime.tryParse(data['at'] as String? ?? ''));
    } catch (_) {
      state = state.copyWith(loading: false, fromSnapshot: true);
    }
  }
}

final memosProvider = StateNotifierProvider<MemosNotifier, MemosState>((ref) => MemosNotifier(ref));

// ===== 搜索（防抖 300ms） =====

final searchQueryProvider = StateProvider<String>((_) => '');
final searchTagProvider = StateProvider<String?>((_) => null);

/// 流页分类筛选（null = 全部）
final categoryFilterProvider = StateProvider<String?>((_) => null);

final searchResultsProvider = FutureProvider.autoDispose<List<SearchHit>>((ref) async {
  final q = ref.watch(searchQueryProvider).trim();
  final client = ref.watch(connProvider).client;
  if (q.length < 2 || client == null) return [];
  // 防抖：query 变化会重建 provider（旧执行结果被丢弃），入口延迟 300ms 即等效防抖
  await Future<void>.delayed(const Duration(milliseconds: 300));
  if (q != ref.read(searchQueryProvider).trim()) return []; // 过期执行，丢弃
  try {
    return await client.search(q, limit: 30);
  } on BmApiException catch (e) {
    if (e.unreachable) ref.read(connProvider.notifier).markOffline();
    return [];
  }
});

// ===== 标签 / 分类 / 统计（在线时随流页刷新） =====

final tagsProvider = FutureProvider<List<TagStat>>((ref) async {
  final client = ref.watch(connProvider).client;
  if (client == null) return [];
  try {
    return await client.tagStats();
  } on BmApiException {
    return [];
  }
});

final categoriesProvider = FutureProvider<List<Category>>((ref) async {
  final client = ref.watch(connProvider).client;
  if (client == null) return [];
  try {
    return await client.listCategories();
  } on BmApiException {
    return [];
  }
});

final statsProvider = FutureProvider<StatsOverview>((ref) async {
  // 依赖流页刷新（lastSyncAt 变化即重取统计）
  ref.watch(memosProvider.select((s) => s.lastSyncAt));
  final client = ref.watch(connProvider).client;
  if (client == null) throw const BmApiException(0, '离线');
  return client.stats();
});
