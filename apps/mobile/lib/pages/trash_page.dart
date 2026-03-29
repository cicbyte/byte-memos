/// 回收站页：软删除备忘列表 · 恢复 / 彻底删除 / 清空（F04 延伸）
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:intl/intl.dart';

import '../conn/client.dart';
import '../conn/models.dart';
import '../design/glass.dart';
import '../design/theme.dart';
import '../state/conn_state.dart';
import '../state/memos_state.dart';

/// 回收站数据：手动刷新（恢复/删除后 invalidate + 流列表联动刷新）
final trashProvider = StateNotifierProvider<TrashNotifier, AsyncValue<List<TrashedItem>>>((ref) => TrashNotifier(ref));

class TrashNotifier extends StateNotifier<AsyncValue<List<TrashedItem>>> {
  TrashNotifier(this._ref) : super(const AsyncValue.loading()) {
    load();
  }

  final Ref _ref;

  BmClient? get _client => _ref.read(connProvider).client;

  Future<void> load() async {
    final c = _client;
    if (c == null) return;
    state = const AsyncValue.loading();
    try {
      final list = await c.listTrash();
      state = AsyncValue.data(list);
    } catch (e) {
      state = AsyncValue.error(e, StackTrace.current);
    }
  }

  Future<bool> restore(String id) async {
    final c = _client;
    if (c == null) return false;
    try {
      await c.restoreMemo(id);
      await load();
      await _ref.read(memosProvider.notifier).refresh();
      _ref.invalidate(statsProvider);
      return true;
    } catch (_) {
      return false;
    }
  }

  Future<bool> purge(String id) async {
    final c = _client;
    if (c == null) return false;
    try {
      await c.purgeMemo(id);
      await load();
      _ref.invalidate(statsProvider);
      return true;
    } catch (_) {
      return false;
    }
  }

  Future<bool> empty() async {
    final c = _client;
    if (c == null) return false;
    try {
      await c.emptyTrash();
      await load();
      _ref.invalidate(statsProvider);
      return true;
    } catch (_) {
      return false;
    }
  }
}

class TrashPage extends ConsumerWidget {
  const TrashPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final trash = ref.watch(trashProvider);
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);

    return Scaffold(
      backgroundColor: Colors.transparent,
      body: Container(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: bgGradient(Theme.of(context).brightness),
          ),
        ),
        child: SafeArea(
          bottom: false,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // 顶栏（与详情页同款玻璃吸顶）
              GlassContainer(
                tier: GlassTier.regular,
                preference: ref.watch(glassPrefProvider),
                padding: EdgeInsets.zero,
                child: Row(
                  children: [
                    IconButton(
                      icon: const Icon(Icons.arrow_back_rounded),
                      onPressed: () => Navigator.pop(context),
                    ),
                    Text('回收站', style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600, color: fg)),
                    const Spacer(),
                    trash.maybeWhen(
                      data: (l) => l.isEmpty
                          ? const SizedBox.shrink()
                          : TextButton(
                              onPressed: () => _confirmEmpty(context, ref),
                              child: const Text('清空', style: TextStyle(fontSize: 13, color: Colors.redAccent)),
                            ),
                      orElse: () => const SizedBox.shrink(),
                    ),
                    const SizedBox(width: 6),
                  ],
                ),
              ),

              Expanded(
                child: trash.when(
                  loading: () => const Center(child: CircularProgressIndicator(strokeWidth: 2)),
                  error: (e, _) => Center(
                child: TextButton(
                  onPressed: () => ref.read(trashProvider.notifier).load(),
                  child: Text('加载失败，点此重试', style: TextStyle(fontSize: 13, color: Theme.of(context).hintColor)),
                ),
              ),
                  data: (items) => items.isEmpty
                      ? ListView(children: [
                          const SizedBox(height: 140),
                          Center(
                            child: Icon(Icons.delete_outline_rounded,
                                size: 46, color: Theme.of(context).hintColor.withValues(alpha: 0.5)),
                          ),
                          const SizedBox(height: 12),
                          Center(
                            child: Text('回收站是空的',
                                style: TextStyle(fontSize: 13, color: Theme.of(context).hintColor)),
                          ),
                        ])
                      : RefreshIndicator(
                          onRefresh: () => ref.read(trashProvider.notifier).load(),
                          color: kPrimary,
                          child: ListView.builder(
                            padding: const EdgeInsets.fromLTRB(16, 12, 16, 40),
                            itemCount: items.length,
                            itemBuilder: (_, i) => _TrashCard(item: items[i], fg: fg),
                          ),
                        ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _confirmEmpty(BuildContext context, WidgetRef ref) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('清空回收站？'),
        content: const Text('全部条目将被彻底删除，无法恢复。'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          TextButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('清空', style: TextStyle(color: Colors.redAccent)),
          ),
        ],
      ),
    );
    if (ok != true) return;
    final done = await ref.read(trashProvider.notifier).empty();
    if (!context.mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(done ? '已清空' : '操作失败')));
  }
}

class _TrashCard extends ConsumerWidget {
  const _TrashCard({required this.item, required this.fg});

  final TrashedItem item;
  final Color fg;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final dt = DateTime.tryParse(item.deletedAt);
    return GlassContainer(
      tier: GlassTier.flat,
      preference: ref.watch(glassPrefProvider),
      margin: const EdgeInsets.only(bottom: 10),
      padding: const EdgeInsets.all(14),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  item.title.isEmpty ? '（无标题）' : item.title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(fontSize: 14.5, fontWeight: FontWeight.w600, color: fg),
                ),
                if (item.description.isNotEmpty) ...[
                  const SizedBox(height: 3),
                  Text(
                    item.description,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(fontSize: 12, height: 1.5, color: fg.withValues(alpha: 0.55)),
                  ),
                ],
                const SizedBox(height: 5),
                Row(
                  children: [
                    Text(
                      '删除于 ${dt == null ? '' : DateFormat('M月d日 HH:mm', 'zh').format(dt)}',
                      style: TextStyle(fontSize: 10.5, color: Theme.of(context).hintColor),
                    ),
                    if (item.category != null) ...[
                      const SizedBox(width: 8),
                      Text('${item.category}',
                          style: TextStyle(fontSize: 10.5, color: kAccent.withValues(alpha: 0.8))),
                    ],
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          // 恢复
          GestureDetector(
            onTap: () async {
              final ok = await ref.read(trashProvider.notifier).restore(item.shortId);
              if (!context.mounted) return;
              ScaffoldMessenger.of(context)
                  .showSnackBar(SnackBar(content: Text(ok ? '已恢复到备忘流' : '恢复失败')));
            },
            child: GlassContainer(
              tier: GlassTier.regular,
              preference: ref.watch(glassPrefProvider),
              borderRadius: BorderRadius.circular(12),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 7),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.restore_rounded, size: 15, color: dark ? Colors.white : kPrimary),
                  const SizedBox(width: 4),
                  Text('恢复', style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: dark ? Colors.white : kPrimary)),
                ],
              ),
            ),
          ),
          const SizedBox(width: 6),
          GestureDetector(
            onTap: () => _confirmPurge(context, ref),
            child: Icon(Icons.delete_forever_rounded, size: 20, color: Colors.redAccent.withValues(alpha: 0.8)),
          ),
        ],
      ),
    );
  }

  void _confirmPurge(BuildContext context, WidgetRef ref) async {
    final ok = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('彻底删除？'),
        content: Text(item.title.isEmpty ? '该条目将无法恢复。' : '「${item.title}」将无法恢复。'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('取消')),
          TextButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('彻底删除', style: TextStyle(color: Colors.redAccent)),
          ),
        ],
      ),
    );
    if (ok != true) return;
    await ref.read(trashProvider.notifier).purge(item.shortId);
  }
}
