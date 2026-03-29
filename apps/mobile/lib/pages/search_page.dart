/// 搜索页：输入即搜（防抖 300ms）+ 标签/分类筛选 chips
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../conn/models.dart';
import '../design/theme.dart';
import '../state/memos_state.dart';
import 'detail_page.dart';

class SearchPage extends ConsumerWidget {
  SearchPage({super.key});

  final _controller = TextEditingController();

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    final q = ref.watch(searchQueryProvider);
    final results = ref.watch(searchResultsProvider);
    final tags = ref.watch(tagsProvider).valueOrNull ?? [];
    final cats = ref.watch(categoriesProvider).valueOrNull ?? [];
    final activeTag = ref.watch(searchTagProvider);

    return Column(
      children: [
        // 搜索框
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
          child: TextField(
            controller: _controller,
            onChanged: (v) => ref.read(searchQueryProvider.notifier).state = v,
            style: TextStyle(color: fg, fontSize: 14),
            decoration: InputDecoration(
              hintText: '搜索备忘录…',
              hintStyle: TextStyle(color: dark ? Colors.white38 : Colors.black38),
              prefixIcon: Icon(Icons.search_rounded, size: 19, color: dark ? Colors.white38 : Colors.black38),
              suffixIcon: q.isEmpty
                  ? null
                  : IconButton(
                      icon: Icon(Icons.close_rounded, size: 17, color: dark ? Colors.white38 : Colors.black38),
                      onPressed: () {
                        _controller.clear();
                        ref.read(searchQueryProvider.notifier).state = '';
                      },
                    ),
              filled: true,
              fillColor: dark ? Colors.white.withValues(alpha: 0.06) : Colors.white.withValues(alpha: 0.62),
              contentPadding: const EdgeInsets.symmetric(vertical: 11),
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(14),
                borderSide: BorderSide.none,
              ),
            ),
          ),
        ),

        // 标签 chips
        if (tags.isNotEmpty)
          SizedBox(
            height: 38,
            child: ListView.separated(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
              itemCount: tags.take(12).length,
              separatorBuilder: (_, _) => const SizedBox(width: 6),
              itemBuilder: (context, i) {
                final t = tags[i];
                final active = activeTag == t.tag;
                return _Chip(
                  label: '#${t.tag}',
                  active: active,
                  onTap: () => ref.read(searchTagProvider.notifier).state = active ? null : t.tag,
                );
              },
            ),
          ),

        // 结果
        Expanded(
          child: q.trim().length < 2
              ? _recentCats(context, ref, cats, fg)
              : results.when(
                  loading: () => const Center(child: CircularProgressIndicator(strokeWidth: 2)),
                  error: (e, _) => Center(child: Text('$e', style: const TextStyle(fontSize: 12.5))),
                  data: (hits) {
                    // 有标签筛选时客户端二次过滤（REST 搜索不认 tag 参数）
                    final filtered = activeTag == null ? hits : hits.where((h) => h.matchedTags.contains(activeTag)).toList();
                    if (filtered.isEmpty) {
                      return Center(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            Icon(Icons.search_off_rounded, size: 40, color: Theme.of(context).hintColor),
                            const SizedBox(height: 10),
                            Text('没有匹配的备忘录', style: TextStyle(fontSize: 13, color: Theme.of(context).hintColor)),
                          ],
                        ),
                      );
                    }
                    return ListView.separated(
                      padding: const EdgeInsets.fromLTRB(16, 8, 16, 190),
                      itemCount: filtered.length,
                      separatorBuilder: (_, _) => const SizedBox(height: 8),
                      itemBuilder: (context, i) {
                        final h = filtered[i];
                        return _HitCard(
                          hit: h,
                          onTap: () => Navigator.of(context).push(
                            MaterialPageRoute(builder: (_) => DetailPage(memoId: h.shortId)),
                          ),
                        );
                      },
                    );
                  },
                ),
        ),
      ],
    );
  }

  Widget _recentCats(BuildContext context, WidgetRef ref, List cats, Color fg) {
    if (cats.isEmpty) {
      return Center(
        child: Text(
          '输入至少 2 个字符开始搜索',
          style: TextStyle(fontSize: 12.5, color: Theme.of(context).hintColor),
        ),
      );
    }
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 190),
      children: [
        Padding(
          padding: const EdgeInsets.only(left: 4, bottom: 8),
          child: Text('分类', style: TextStyle(fontSize: 12.5, fontWeight: FontWeight.w600, color: Theme.of(context).hintColor)),
        ),
        for (final c in cats)
          ListTile(
            dense: true,
            contentPadding: const EdgeInsets.symmetric(horizontal: 10),
            leading: Icon(Icons.folder_outlined, size: 19, color: kPrimary.withValues(alpha: 0.8)),
            title: Text(c.name, style: TextStyle(fontSize: 13.5, color: fg)),
            trailing: Text('${c.memoCount} 条', style: TextStyle(fontSize: 11, color: Theme.of(context).hintColor)),
            onTap: () {
              // 按分类筛选 → 跳流页思路：直接搜分类名（简单可靠）
              _controller.text = c.name;
              ref.read(searchQueryProvider.notifier).state = c.name;
            },
          ),
      ],
    );
  }
}

class _Chip extends StatelessWidget {
  const _Chip({required this.label, required this.active, required this.onTap});

  final String label;
  final bool active;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 11, vertical: 4.5),
        decoration: BoxDecoration(
          color: active ? kPrimary : Theme.of(context).brightness == Brightness.dark ? Colors.white.withValues(alpha: 0.08) : Colors.white.withValues(alpha: 0.62),
          borderRadius: BorderRadius.circular(999),
        ),
        child: Text(
          label,
          style: TextStyle(
            fontSize: 11.5,
            color: active ? Colors.white : (Theme.of(context).brightness == Brightness.dark ? Colors.white70 : Colors.black54),
          ),
        ),
      ),
    );
  }
}

class _HitCard extends StatelessWidget {
  const _HitCard({required this.hit, required this.onTap});

  final SearchHit hit;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final title = hit.title;
    final snippet = hit.snippet;
    final shortId = hit.shortId;
    final matched = hit.matchedTags;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);

    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.all(13),
        decoration: BoxDecoration(
          color: dark ? Colors.white.withValues(alpha: 0.05) : Colors.white.withValues(alpha: 0.62),
          borderRadius: BorderRadius.circular(14),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    title.isEmpty ? '（无标题）' : title,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: fg),
                  ),
                ),
                Text(shortId, style: TextStyle(fontSize: 9.5, color: Theme.of(context).hintColor)),
              ],
            ),
            if (snippet.isNotEmpty) ...[
              const SizedBox(height: 5),
              Text(
                snippet,
                maxLines: 3,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(fontSize: 12, height: 1.5, color: fg.withValues(alpha: 0.65)),
              ),
            ],
            if (matched.isNotEmpty) ...[
              const SizedBox(height: 7),
              Wrap(
                spacing: 8,
                children: [for (final t in matched.take(5)) Text('#$t', style: TextStyle(fontSize: 10.5, color: kAccent.withValues(alpha: 0.9)))],
              ),
            ],
          ],
        ),
      ),
    );
  }
}
