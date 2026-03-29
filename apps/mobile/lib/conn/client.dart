/// REST client：对桌面端 `/api/v1` 的完整封装（dio）
///
/// 鉴权：X-API-Key 头（连接配置里 Key 为空则不带——桌面端开「本机免 Key」时可用）。
/// 所有错误统一抛 [BmApiException]，UI 层按 status 区分提示。
library;

import 'package:dio/dio.dart';

import 'models.dart';

/// 业务错误：携带 HTTP 状态码（0 = 网络不通）
class BmApiException implements Exception {
  const BmApiException(this.status, this.message);

  final int status;
  final String message;

  /// 是否「连不上」（区分于 Key 错 / 服务端 5xx）
  bool get unreachable => status == 0;

  @override
  String toString() => 'BmApiException($status): $message';
}

class BmClient {
  BmClient({required String baseUrl, String? apiKey})
      : _dio = Dio(
          BaseOptions(
            baseUrl: baseUrl.endsWith('/api/v1') ? baseUrl : '$baseUrl/api/v1',
            connectTimeout: const Duration(seconds: 4),
            receiveTimeout: const Duration(seconds: 10),
            headers: {
              if (apiKey != null && apiKey.isNotEmpty) 'X-API-Key': apiKey,
            },
          ),
        );

  final Dio _dio;

  // ===== 内部：统一错误映射 =====

  Future<T> _guard<T>(Future<Response<T>> Function() fn) async {
    try {
      final resp = await fn();
      return resp.data as T;
    } on DioException catch (e) {
      if (e.type == DioExceptionType.connectionTimeout ||
          e.type == DioExceptionType.sendTimeout ||
          e.type == DioExceptionType.connectionError) {
        throw BmApiException(0, '无法连接服务器');
      }
      final status = e.response?.statusCode ?? 0;
      if (status == 401) throw const BmApiException(401, 'API Key 不正确');
      final body = e.response?.data;
      final msg = body is Map ? (body['error'] as String? ?? '请求失败') : '请求失败(HTTP $status)';
      throw BmApiException(status, msg);
    }
  }

  // ===== 连通性 =====

  /// 连接测试：GET /memos?limit=1
  /// 返回 (ok, statusCode)——401 说明服务在跑但 Key 错
  Future<(bool, int)> ping() async {
    try {
      await _dio.get<dynamic>('/memos', queryParameters: {'limit': 1});
      return (true, 200);
    } on DioException catch (e) {
      final status = e.response?.statusCode ?? 0;
      return (false, status);
    }
  }

  // ===== 备忘录 =====

  Future<List<Memo>> listMemos({int limit = 20, int offset = 0, String? tag, String? category, bool pinnedOnly = false}) async {
    final data = await _guard(() => _dio.get<dynamic>(
          '/memos',
          queryParameters: {
            'limit': limit,
            'offset': offset,
            'tag': ?tag,
            'category': ?category,
            if (pinnedOnly) 'pinned_only': true,
          },
        ));
    return (data as List<dynamic>).map((e) => Memo.fromJson(e as Map<String, dynamic>)).toList();
  }

  Future<Memo> getMemo(String id) async {
    final data = await _guard(() => _dio.get<dynamic>('/memos/$id'));
    return Memo.fromJson(data as Map<String, dynamic>);
  }

  Future<Memo> addMemo({required String content, String title = '', String visibility = 'PRIVATE', String? category, List<String> tags = const []}) async {
    final data = await _guard(() => _dio.post<dynamic>(
          '/memos',
          data: {
            'title': title,
            'content': content,
            'visibility': visibility,
            'category': ?category,
            'tags': tags,
          },
        ));
    return Memo.fromJson(data as Map<String, dynamic>);
  }

  Future<Memo> updateMemo(String id, {String? title, String? content, String? category, List<String>? tags, bool? pinned, String? visibility}) async {
    final data = await _guard(() => _dio.put<dynamic>(
          '/memos/$id',
          data: {
            'title': ?title,
            'content': ?content,
            'category': ?category,
            'tags': ?tags,
            'pinned': ?pinned,
            'visibility': ?visibility,
          },
        ));
    return Memo.fromJson(data as Map<String, dynamic>);
  }

  Future<void> deleteMemo(String id) async {
    await _guard(() => _dio.delete<dynamic>('/memos/$id'));
  }

  Future<Memo> togglePinned(String id) async {
    final data = await _guard(() => _dio.post<dynamic>('/memos/$id/toggle-pinned'));
    return Memo.fromJson(data as Map<String, dynamic>);
  }

  Future<List<SearchHit>> search(String q, {int limit = 30}) async {
    final data = await _guard(() => _dio.get<dynamic>(
          '/search/memos',
          queryParameters: {'q': q, 'limit': limit},
        ));
    return (data as List<dynamic>).map((e) => SearchHit.fromJson(e as Map<String, dynamic>)).toList();
  }

  // ===== 分类 / 统计 / 标签 =====

  Future<List<Category>> listCategories() async {
    final data = await _guard(() => _dio.get<dynamic>('/categories'));
    return (data as List<dynamic>).map((e) => Category.fromJson(e as Map<String, dynamic>)).toList();
  }

  Future<StatsOverview> stats() async {
    final data = await _guard(() => _dio.get<dynamic>('/stats'));
    return StatsOverview.fromJson(data as Map<String, dynamic>);
  }

  Future<List<TagStat>> tagStats() async {
    final data = await _guard(() => _dio.get<dynamic>('/tags'));
    return (data as List<dynamic>).map((e) => TagStat.fromJson(e as Map<String, dynamic>)).toList();
  }

  // ===== 回收站（删除撤回用） =====

  Future<void> restoreMemo(String id) async {
    await _guard(() => _dio.post<dynamic>('/trash/memo/$id/restore'));
  }

  Future<List<TrashedItem>> listTrash() async {
    final data = await _guard(() => _dio.get<dynamic>('/trash'));
    return (data as List<dynamic>).map((e) => TrashedItem.fromJson(e as Map<String, dynamic>)).toList();
  }

  Future<void> purgeMemo(String id) async {
    await _guard(() => _dio.delete<dynamic>('/trash/memo/$id'));
  }

  Future<void> emptyTrash() async {
    await _guard(() => _dio.post<dynamic>('/trash'));
  }
}
