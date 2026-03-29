/// 连接状态 + 玻璃偏好（Riverpod）
///
/// ConnState 是全 app 的「电网」：offline 时列表/统计全部降级为快照只读。
library;

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../conn/client.dart';
import '../conn/conn_store.dart';
import '../design/glass.dart';

enum ConnStatus {
  /// 未配置任何连接
  none,

  /// 正在连接/测试
  connecting,

  /// 在线
  online,

  /// 连不上（服务未开 / 地址错）
  offline,

  /// 服务在跑但 Key 不对
  keyError,
}

class ConnState {
  const ConnState({
    this.status = ConnStatus.none,
    this.profiles = const [],
    this.activeId,
    this.client,
    this.lastOkAt,
  });

  final ConnStatus status;
  final List<ConnProfile> profiles;
  final String? activeId;

  /// 激活连接的 client（status==online 时可用）
  final BmClient? client;

  /// 最近一次成功通信时间（离线横幅展示）
  final DateTime? lastOkAt;

  ConnProfile? get activeProfile => profiles.where((p) => p.id == activeId).firstOrNull;

  bool get isOnline => status == ConnStatus.online;

  ConnState copyWith({ConnStatus? status, List<ConnProfile>? profiles, String? activeId, BmClient? client, DateTime? lastOkAt}) =>
      ConnState(
        status: status ?? this.status,
        profiles: profiles ?? this.profiles,
        activeId: activeId ?? this.activeId,
        client: client ?? this.client,
        lastOkAt: lastOkAt ?? this.lastOkAt,
      );
}

class ConnNotifier extends StateNotifier<ConnState> {
  ConnNotifier() : super(const ConnState());

  ConnStore? _store;

  /// store 就绪后挂载并自动探测（由 provider 装配处驱动）
  void attach(ConnStore store) {
    if (_store != null) return;
    _store = store;
    unawaited(init());
  }

  /// 启动时载入配置并探测激活连接
  Future<void> init() async {
    final store = _store;
    if (store == null) return;
    final profiles = await store.loadProfiles();
    final activeId = await store.loadActiveId();
    state = ConnState(profiles: profiles, activeId: activeId, client: null, status: ConnStatus.none);
    final active = profiles.where((p) => p.id == activeId).firstOrNull;
    if (active == null || active.baseUrl.isEmpty) return;
    await probe(active);
  }

  /// 探测某个配置：ping → online / offline / keyError
  Future<ConnStatus> probe(ConnProfile p) async {
    state = state.copyWith(status: ConnStatus.connecting);
    final client = BmClient(baseUrl: p.baseUrl, apiKey: p.apiKey);
    final (ok, code) = await client.ping();
    final st = ok
        ? ConnStatus.online
        : code == 401
            ? ConnStatus.keyError
            : ConnStatus.offline;
    state = ConnState(
      status: st,
      profiles: state.profiles,
      activeId: p.id,
      client: ok ? client : null,
      lastOkAt: ok ? DateTime.now() : state.lastOkAt,
    );
    return st;
  }

  /// 保存/更新配置并激活
  Future<void> saveAndActivate(ConnProfile p) async {
    final store = _store;
    if (store == null) return;
    final profiles = [...state.profiles.where((x) => x.id != p.id), p];
    await store.saveProfiles(profiles, activeId: p.id);
    state = state.copyWith(profiles: profiles, activeId: p.id);
    await probe(p);
  }

  Future<void> activate(String id) async {
    final p = state.profiles.where((x) => x.id == id).firstOrNull;
    if (p == null) return;
    await _store?.setActive(id);
    await probe(p);
  }

  Future<void> remove(String id) async {
    await _store?.deleteProfile(id);
    final profiles = state.profiles.where((x) => x.id != id).toList();
    if (state.activeId == id) {
      final next = profiles.firstOrNull;
      state = ConnState(profiles: profiles, activeId: next?.id);
      if (next != null) await probe(next);
    } else {
      state = state.copyWith(profiles: profiles);
    }
  }

  /// 任何请求失败后由调用方标记失联
  void markOffline() {
    if (state.status == ConnStatus.online) {
      state = state.copyWith(status: ConnStatus.offline, client: null);
    }
  }
}

// ===== Provider 装配 =====

final sharedPrefsProvider = FutureProvider<SharedPreferences>((_) => SharedPreferences.getInstance());

final connStoreProvider = FutureProvider<ConnStore>((ref) async {
  final prefs = await ref.watch(sharedPrefsProvider.future);
  return ConnStore(prefs, const FlutterSecureStorage());
});

final connProvider = StateNotifierProvider<ConnNotifier, ConnState>((ref) {
  final notifier = ConnNotifier();
  // store 就绪即挂载（fireImmediately 保证冷启动路径）
  ref.listen(connStoreProvider, (_, next) {
    next.whenData(notifier.attach);
  }, fireImmediately: true);
  return notifier;
});

/// 玻璃强度偏好（持久化）
final glassPrefProvider = StateNotifierProvider<GlassPrefNotifier, GlassPreference>((ref) {
  return GlassPrefNotifier(ref);
});

class GlassPrefNotifier extends StateNotifier<GlassPreference> {
  GlassPrefNotifier(this._ref) : super(GlassPreference.auto) {
    _load();
  }

  final Ref _ref;

  Future<void> _load() async {
    try {
      final prefs = await _ref.read(sharedPrefsProvider.future);
      state = GlassPreference.fromName(prefs.getString('bm.glass.pref'));
    } catch (_) {}
  }

  Future<void> set(GlassPreference p) async {
    state = p;
    try {
      final prefs = await _ref.read(sharedPrefsProvider.future);
      await prefs.setString('bm.glass.pref', p.name);
    } catch (_) {}
  }
}

/// 主题模式：跟随系统 / 亮 / 暗
final themeModeProvider = StateNotifierProvider<ThemeModeNotifier, ThemeMode>((ref) => ThemeModeNotifier(ref));

class ThemeModeNotifier extends StateNotifier<ThemeMode> {
  ThemeModeNotifier(this._ref) : super(ThemeMode.system) {
    _load();
  }

  final Ref _ref;

  Future<void> _load() async {
    try {
      final prefs = await _ref.read(sharedPrefsProvider.future);
      final v = prefs.getString('bm.theme.mode');
      state = switch (v) {
        'light' => ThemeMode.light,
        'dark' => ThemeMode.dark,
        _ => ThemeMode.system,
      };
    } catch (_) {}
  }

  Future<void> set(ThemeMode m) async {
    state = m;
    try {
      final prefs = await _ref.read(sharedPrefsProvider.future);
      await prefs.setString('bm.theme.mode', m.name);
    } catch (_) {}
  }
}
