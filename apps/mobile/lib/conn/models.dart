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

class TagStat {
  const TagStat({required this.tag, required this.count});

  final String tag;
  final int count;

  factory TagStat.fromJson(Map<String, dynamic> j) => TagStat(
        tag: j['tag'] as String? ?? '',
        count: j['count'] as int? ?? 0,
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
