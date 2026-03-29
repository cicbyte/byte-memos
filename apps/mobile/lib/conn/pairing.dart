/// 移动端配对载荷（与桌面端 buildPairingString 对齐）
///
/// 支持三种形态：
/// 1. `bmemos://pair?v=1&name=..&host=..&port=..&key=..`（扫码/配对串）
/// 2. JSON `{"v":1,"name":..,"host":..,"port":..,"key":..}`
/// 3. 裸 URL `http://192.168.x.x:8318`（手输兜底）
library;

import 'dart:convert';

class PairingPayload {
  const PairingPayload({required this.name, required this.baseUrl, this.apiKey = ''});

  final String name;
  final String baseUrl;
  final String apiKey;
}

PairingPayload? parsePairing(String raw) {
  final s = raw.trim();
  if (s.isEmpty) return null;

  // 1. bmemos://pair?...
  if (s.startsWith('bmemos://')) {
    final uri = Uri.parse(s);
    if (uri.host != 'pair') return null;
    final q = uri.queryParameters;
    final host = q['host'];
    final port = int.tryParse(q['port'] ?? '');
    if (host == null || host.isEmpty || port == null) return null;
    return PairingPayload(
      name: (q['name'] == null || q['name']!.isEmpty) ? '桌面端' : q['name']!,
      baseUrl: 'http://$host:$port',
      apiKey: q['key'] ?? '',
    );
  }

  // 2. JSON
  if (s.startsWith('{')) {
    try {
      final j = jsonDecode(s) as Map<String, dynamic>;
      final host = j['host'] as String?;
      final port = j['port'] as int?;
      if (host == null || host.isEmpty || port == null) return null;
      return PairingPayload(
        name: (j['name'] as String?)?.isNotEmpty == true ? j['name'] as String : '桌面端',
        baseUrl: 'http://$host:$port',
        apiKey: (j['key'] as String?) ?? '',
      );
    } catch (_) {
      return null;
    }
  }

  // 3. 裸 URL
  final url = Uri.tryParse(s);
  if (url != null && (url.scheme == 'http' || url.scheme == 'https') && url.host.isNotEmpty) {
    final port = url.hasPort ? url.port : (url.scheme == 'https' ? 443 : 80);
    return PairingPayload(
      name: '桌面端',
      baseUrl: '${url.scheme}://${url.host}:$port',
      apiKey: '',
    );
  }

  return null;
}
