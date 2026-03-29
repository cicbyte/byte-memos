/// 连接配置存储（dsh-mobile conn_store 模式）
///
/// - 配置列表 + 激活项：shared_preferences（JSON）
/// - API Key：flutter_secure_storage（Android Keystore，绝不进明文 prefs）
library;

import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:shared_preferences/shared_preferences.dart';

class ConnProfile {
  const ConnProfile({required this.id, required this.name, required this.baseUrl, this.apiKey = ''});

  final String id;
  final String name;

  /// 形如 http://192.168.1.10:8318（可带可不带 /api/v1，client 内归一）
  final String baseUrl;

  /// 仅存 secure storage；内存中传递
  final String apiKey;

  ConnProfile copyWith({String? name, String? baseUrl, String? apiKey}) => ConnProfile(
        id: id,
        name: name ?? this.name,
        baseUrl: baseUrl ?? this.baseUrl,
        apiKey: apiKey ?? this.apiKey,
      );

  Map<String, dynamic> toJson() => {'id': id, 'name': name, 'baseUrl': baseUrl};

  factory ConnProfile.fromJson(Map<String, dynamic> j) => ConnProfile(
        id: j['id'] as String,
        name: j['name'] as String? ?? '未命名',
        baseUrl: j['baseUrl'] as String? ?? '',
      );
}

class ConnStore {
  ConnStore(this._prefs, this._secure);

  final SharedPreferences _prefs;
  final FlutterSecureStorage _secure;

  static const _kProfiles = 'bm.conn.profiles';
  static const _kActive = 'bm.conn.active';
  static const _kKeyPrefix = 'bm.conn.key.';

  Future<List<ConnProfile>> loadProfiles() async {
    final raw = _prefs.getString(_kProfiles);
    if (raw == null) return [];
    try {
      final list = (jsonDecode(raw) as List<dynamic>).map((e) => ConnProfile.fromJson(e as Map<String, dynamic>)).toList();
      // 从 secure storage 回填 Key
      final out = <ConnProfile>[];
      for (final p in list) {
        final key = await _secure.read(key: _kKeyPrefix + p.id) ?? '';
        out.add(p.copyWith(apiKey: key));
      }
      return out;
    } catch (_) {
      return [];
    }
  }

  Future<String?> loadActiveId() async => _prefs.getString(_kActive);

  Future<void> saveProfiles(List<ConnProfile> profiles, {String? activeId}) async {
    final active = activeId ?? await loadActiveId();
    await _prefs.setString(_kProfiles, jsonEncode(profiles.map((p) => p.toJson()).toList()));
    if (active != null) await _prefs.setString(_kActive, active);
    // Key 逐个写 secure storage
    for (final p in profiles) {
      final k = _kKeyPrefix + p.id;
      if (p.apiKey.isEmpty) {
        await _secure.delete(key: k);
      } else {
        await _secure.write(key: k, value: p.apiKey);
      }
    }
  }

  Future<void> setActive(String id) async => _prefs.setString(_kActive, id);

  Future<void> deleteProfile(String id) async {
    final profiles = await loadProfiles();
    await saveProfiles(profiles.where((p) => p.id != id).toList());
    await _secure.delete(key: _kKeyPrefix + id);
  }
}
