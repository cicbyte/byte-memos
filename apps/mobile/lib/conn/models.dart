/// REST DTO（与 crates/core/src/model 严格对齐）
///
/// 字段来源：
/// - Memo: model/memo.rs `Memo`
/// - StatsOverview / Category / TagStat: model/common.rs
/// - SearchHit: rest/search 返回（short_id/title/snippet/rank/matched_tags）
library;

class Memo {
  const Memo({
    required this.shortId,
    required this.title,
    required this.content,
    required this.visibility,
    required this.pinned,
    this.categoryId,
    this.category,
    required this.tags,
    required this.createdAt,
    required this.updatedAt,
    this.deletedAt,
  });

  final String shortId;
  final String title;
  final String content;
  final String visibility;
  final bool pinned;
  final int? categoryId;
  final String? category;
  final List<String> tags;
  final String createdAt;
  final String updatedAt;
  final String? deletedAt;

  DateTime? get created => DateTime.tryParse(createdAt);

  factory Memo.fromJson(Map<String, dynamic> j) => Memo(
        shortId: j['short_id'] as String? ?? '',
        title: j['title'] as String? ?? '',
        content: j['content'] as String? ?? '',
        visibility: j['visibility'] as String? ?? 'PRIVATE',
        pinned: j['pinned'] as bool? ?? false,
        categoryId: j['category_id'] as int?,
        category: j['category'] as String?,
        tags: (j['tags'] as List<dynamic>? ?? []).map((e) => e.toString()).toList(),
        createdAt: j['created_at'] as String? ?? '',
        updatedAt: j['updated_at'] as String? ?? '',
        deletedAt: j['deleted_at'] as String?,
      );

  Map<String, dynamic> toJson() => {
        'short_id': shortId,
        'title': title,
        'content': content,
        'visibility': visibility,
        'pinned': pinned,
        'category': category,
        'tags': tags,
        'created_at': createdAt,
        'updated_at': updatedAt,
      };
}

class Category {
  const Category({required this.id, required this.name, this.color, this.memoCount = 0});

  final int id;
  final String name;
  final String? color;
  final int memoCount;

  factory Category.fromJson(Map<String, dynamic> j) => Category(
        id: j['id'] as int,
        name: j['name'] as String? ?? '',
        color: j['color'] as String?,
        memoCount: j['memo_count'] as int? ?? 0,
      );
}

class StatsOverview {
  const StatsOverview({
    required this.memoCount,
    required this.tagCount,
    required this.categoryCount,
    required this.pinnedCount,
    required this.trashedCount,
  });

  final int memoCount;
  final int tagCount;
  final int categoryCount;
  final int pinnedCount;
  final int trashedCount;

  factory StatsOverview.fromJson(Map<String, dynamic> j) => StatsOverview(
        memoCount: j['memo_count'] as int? ?? 0,
        tagCount: j['tag_count'] as int? ?? 0,
        categoryCount: j['category_count'] as int? ?? 0,
        pinnedCount: j['pinned_count'] as int? ?? 0,
        trashedCount: j['trashed_count'] as int? ?? 0,
      );
}

/// 单日备忘数（日历视图数据源，本地日期键）
class DailyCount {
  const DailyCount({required this.date, required this.count});

  final String date;
  final int count;

  factory DailyCount.fromJson(Map<String, dynamic> j) => DailyCount(
        date: j['date'] as String? ?? '',
        count: j['count'] as int? ?? 0,
      );
}

class TagStat {
  const TagStat({required this.tag, required this.count});

  final String tag;
  final int count;

  factory TagStat.fromJson(Map<String, dynamic> j) => TagStat(
        tag: j['tag'] as String? ?? '',
        count: j['count'] as int? ?? 0,
      );
}

/// 附件元数据（D2）
class AttachmentDto {
  const AttachmentDto({
    required this.id,
    required this.filename,
    required this.fileSize,
    required this.mimeType,
    this.createdAt = '',
  });

  final int id;
  final String filename;
  final int fileSize;
  final String mimeType;
  final String createdAt;

  bool get isImage => mimeType.startsWith('image/');

  factory AttachmentDto.fromJson(Map<String, dynamic> j) => AttachmentDto(
        id: (j['id'] as num).toInt(),
        filename: j['filename'] as String? ?? '',
        fileSize: (j['file_size'] as num?)?.toInt() ?? 0,
        mimeType: j['mime_type'] as String? ?? '',
        createdAt: j['created_at'] as String? ?? '',
      );
}

/// AI 供应商（D3；不含 Key）
class AiProvider {
  const AiProvider({
    required this.id,
    required this.name,
    required this.providerKey,
    required this.model,
    required this.models,
    required this.isEnabled,
  });

  final int id;
  final String name;
  final String providerKey;
  final String model;
  final List<String> models;
  final bool isEnabled;

  factory AiProvider.fromJson(Map<String, dynamic> j) => AiProvider(
        id: (j['id'] as num).toInt(),
        name: j['name'] as String? ?? '',
        providerKey: j['provider_key'] as String? ?? '',
        model: j['model'] as String? ?? '',
        models: (j['models'] as List<dynamic>?)?.map((e) => e as String).toList() ?? const [],
        isEnabled: j['is_enabled'] as bool? ?? false,
      );
}

/// 回收站条目（软删除备忘，与桌面端 TrashedItem 对齐）
class TrashedItem {
  const TrashedItem({
    required this.shortId,
    required this.title,
    required this.description,
    this.category,
    required this.deletedAt,
  });

  final String shortId;
  final String title;
  final String description;
  final String? category;

  /// 删除时间（ISO8601）
  final String deletedAt;

  factory TrashedItem.fromJson(Map<String, dynamic> j) => TrashedItem(
        shortId: j['short_id'] as String? ?? '',
        title: j['title'] as String? ?? '',
        description: j['description'] as String? ?? '',
        category: j['category'] as String?,
        deletedAt: j['deleted_at'] as String? ?? '',
      );
}

class SearchHit {
  const SearchHit({
    required this.shortId,
    required this.title,
    required this.snippet,
    this.matchedTags = const [],
  });

  final String shortId;
  final String title;
  final String snippet;
  final List<String> matchedTags;

  /// 服务端高亮标签原样剥掉（移动端不渲染 mark）
  static String _clean(String s) => s.replaceAll(RegExp('</?mark>'), '');

  factory SearchHit.fromJson(Map<String, dynamic> j) => SearchHit(
        shortId: j['short_id'] as String? ?? '',
        title: j['title'] as String? ?? '',
        snippet: _clean(j['snippet'] as String? ?? ''),
        matchedTags: (j['matched_tags'] as List<dynamic>? ?? []).map((e) => e.toString()).toList(),
      );
}
