/// AI 对话页（D3）：玻璃气泡 + SSE 流式输出 + 供应商/模型切换
library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../design/glass.dart';
import '../design/theme.dart';
import '../state/ai_state.dart';
import '../state/conn_state.dart';

class AiChatPage extends ConsumerStatefulWidget {
  const AiChatPage({super.key});

  @override
  ConsumerState<AiChatPage> createState() => _AiChatPageState();
}

class _AiChatPageState extends ConsumerState<AiChatPage> {
  final _input = TextEditingController();
  final _scroll = ScrollController();

  @override
  void dispose() {
    _input.dispose();
    _scroll.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    final fg = dark ? const Color(0xFFE6E8EE) : const Color(0xFF1F2328);
    final chat = ref.watch(aiChatProvider);
    final prov = ref.watch(aiProvidersProvider);
    final notifier = ref.read(aiChatProvider.notifier);

    return Scaffold(
      extendBodyBehindAppBar: true,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        title: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text('AI 助手', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: fg)),
            const SizedBox(width: 8),
            _ModelChip(dark: dark, fg: fg),
          ],
        ),
        actions: [
          if (chat.messages.isNotEmpty)
            IconButton(
              tooltip: '清空对话',
              icon: Icon(Icons.delete_sweep_outlined, size: 20, color: fg.withValues(alpha: 0.7)),
              onPressed: () => notifier.reset(),
            ),
          const SizedBox(width: 4),
        ],
      ),
      body: Container(
        decoration: BoxDecoration(gradient: LinearGradient(colors: bgGradient(Theme.of(context).brightness))),
        child: SafeArea(
          child: Column(
            children: [
              // 消息流
              Expanded(
                child: prov.when(
                  loading: () => const Center(child: CircularProgressIndicator(strokeWidth: 2)),
                  error: (e, _) => _Hint(text: '供应商加载失败：$e', fg: fg),
                  data: (_) => chat.messages.isEmpty
                      ? _Empty(fg: fg)
                      : ListView.builder(
                          controller: _scroll,
                          padding: const EdgeInsets.fromLTRB(14, 8, 14, 8),
                          itemCount: chat.messages.length + (chat.streaming ? 1 : 0),
                          itemBuilder: (_, i) {
                            if (i == chat.messages.length) {
                              return _Bubble(
                                role: 'assistant',
                                text: chat.streamBuf,
                                thinking: chat.thinkingBuf,
                                streaming: true,
                                dark: dark,
                                fg: fg,
                              );
                            }
                            final m = chat.messages[i];
                            return _Bubble(
                              role: m.role,
                              text: m.content,
                              dark: dark,
                              fg: fg,
                            );
                          },
                        ),
                ),
              ),
              // 错误条
              if (chat.error != null)
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 4, 16, 0),
                  child: Text(
                    chat.error!,
                    style: const TextStyle(fontSize: 11, color: Color(0xFFFF6B6B)),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              // 输入条
              _InputBar(
                controller: _input,
                dark: dark,
                fg: fg,
                busy: chat.streaming,
                onSend: () {
                  final text = _input.text.trim();
                  if (text.isEmpty || chat.streaming) return;
                  _input.clear();
                  notifier.send(text);
                  Future.delayed(const Duration(milliseconds: 80), _scrollToEnd);
                },
                onStop: () => notifier.stop(),
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _scrollToEnd() {
    if (!_scroll.hasClients) return;
    _scroll.animateTo(_scroll.position.maxScrollExtent + 60,
        duration: const Duration(milliseconds: 200), curve: Curves.easeOut);
  }
}

// ===== 供应商/模型选择 =====

class _ModelChip extends ConsumerWidget {
  const _ModelChip({required this.dark, required this.fg});

  final bool dark;
  final Color fg;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final sel = ref.watch(aiSelectionProvider);
    return GestureDetector(
      onTap: () => _pickModel(context, ref),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 3),
        decoration: BoxDecoration(
          color: kPrimary.withValues(alpha: 0.12),
          borderRadius: BorderRadius.circular(8),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.auto_awesome, size: 11, color: kPrimary.withValues(alpha: 0.9)),
            const SizedBox(width: 4),
            Text(
              sel == null ? '选模型' : sel.$2,
              style: TextStyle(fontSize: 10.5, color: kPrimary, fontWeight: FontWeight.w600),
            ),
            Icon(Icons.expand_more_rounded, size: 13, color: kPrimary.withValues(alpha: 0.8)),
          ],
        ),
      ),
    );
  }

  Future<void> _pickModel(BuildContext context, WidgetRef ref) async {
    final providers = ref.read(aiProvidersProvider).valueOrNull ?? [];
    final sel = ref.read(aiSelectionProvider);
    // 展平 (provider, model) 选项；已选的排最前
    final options = <(String, String, int, String)>[];
    for (final p in providers) {
      if (p.model.isEmpty && p.models.isEmpty) continue;
      if (p.model.isNotEmpty) options.add((p.name, p.model, p.id, p.model));
      for (final m in p.models) {
        if (m != p.model) options.add((p.name, m, p.id, m));
      }
    }
    if (!context.mounted) return;
    await showModalBottomSheet(
      context: context,
      backgroundColor: Colors.transparent,
      builder: (ctx) => GlassContainer(
        tier: GlassTier.thick,
        preference: ref.read(glassPrefProvider),
        borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
        padding: const EdgeInsets.fromLTRB(8, 6, 8, 14),
        child: SafeArea(
          top: false,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Padding(
                padding: const EdgeInsets.only(left: 14, top: 8, bottom: 4),
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: Text(
                    '选择模型（沿用桌面端配置）',
                    style: TextStyle(fontSize: 12, color: fg.withValues(alpha: 0.55)),
                  ),
                ),
              ),
              Flexible(
                child: options.isEmpty
                    ? const Padding(
                        padding: EdgeInsets.all(20),
                        child: Text('没有可用模型：请先在桌面端「AI 供应商」里选择模型'),
                      )
                    : ListView(
                        shrinkWrap: true,
                        children: [
                          for (final o in options)
                            ListTile(
                              dense: true,
                              leading: sel != null && sel.$1 == o.$3 && sel.$2 == o.$2
                                  ? Icon(Icons.check_circle_rounded, size: 18, color: kPrimary)
                                  : Icon(Icons.circle_outlined, size: 18, color: fg.withValues(alpha: 0.25)),
                              title: Text(o.$2, style: const TextStyle(fontSize: 13.5)),
                              subtitle: Text(o.$1, style: const TextStyle(fontSize: 11)),
                              onTap: () {
                                ref.read(aiSelectionProvider.notifier).state = (o.$3, o.$2);
                                Navigator.pop(ctx);
                              },
                            ),
                        ],
                      ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ===== 气泡 =====

class _Bubble extends StatelessWidget {
  const _Bubble({
    required this.role,
    required this.text,
    this.thinking = '',
    this.streaming = false,
    required this.dark,
    required this.fg,
  });

  final String role;
  final String text;
  final String thinking;
  final bool streaming;
  final bool dark;
  final Color fg;

  @override
  Widget build(BuildContext context) {
    final isUser = role == 'user';
    return Align(
      alignment: isUser ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.only(bottom: 10),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        constraints: BoxConstraints(maxWidth: MediaQuery.sizeOf(context).width * 0.82),
        decoration: BoxDecoration(
          color: isUser
              ? kPrimary.withValues(alpha: dark ? 0.45 : 0.9)
              : (dark ? Colors.white.withValues(alpha: 0.07) : Colors.black.withValues(alpha: 0.045)),
          borderRadius: BorderRadius.only(
            topLeft: const Radius.circular(16),
            topRight: const Radius.circular(16),
            bottomLeft: Radius.circular(isUser ? 16 : 4),
            bottomRight: Radius.circular(isUser ? 4 : 16),
          ),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (streaming && thinking.isNotEmpty)
              Padding(
                padding: const EdgeInsets.only(bottom: 6),
                child: Text(
                  thinking.length > 80 ? thinking.substring(thinking.length - 80) : thinking,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(fontSize: 10, height: 1.4, color: fg.withValues(alpha: 0.4)),
                ),
              ),
            Text(
              text.isEmpty && streaming ? '…' : text,
              style: TextStyle(
                fontSize: 13.5,
                height: 1.6,
                color: isUser ? Colors.white : fg,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _Empty extends StatelessWidget {
  const _Empty({required this.fg});

  final Color fg;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 64,
            height: 64,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              gradient: LinearGradient(colors: [kPrimary.withValues(alpha: 0.25), kAccent.withValues(alpha: 0.25)]),
            ),
            child: Icon(Icons.auto_awesome_rounded, size: 30, color: kPrimary),
          ),
          const SizedBox(height: 14),
          Text('有什么可以帮你？', style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600, color: fg)),
          const SizedBox(height: 6),
          Text(
            '模型与 Key 沿用桌面端「AI 供应商」配置\n右上方可切换模型',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 11.5, height: 1.6, color: fg.withValues(alpha: 0.5)),
          ),
        ],
      ),
    );
  }
}

class _Hint extends StatelessWidget {
  const _Hint({required this.text, required this.fg});

  final String text;
  final Color fg;

  @override
  Widget build(BuildContext context) {
    return Center(child: Padding(padding: const EdgeInsets.all(24), child: Text(text, style: TextStyle(fontSize: 12.5, color: fg.withValues(alpha: 0.6)), textAlign: TextAlign.center)));
  }
}

class _InputBar extends StatelessWidget {
  const _InputBar({
    required this.controller,
    required this.dark,
    required this.fg,
    required this.busy,
    required this.onSend,
    required this.onStop,
  });

  final TextEditingController controller;
  final bool dark;
  final Color fg;
  final bool busy;
  final VoidCallback onSend;
  final VoidCallback onStop;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.fromLTRB(14, 8, 14, MediaQuery.viewInsetsOf(context).bottom + 10),
      child: GlassContainer(
        tier: GlassTier.regular,
        preference: GlassPreference.auto,
        borderRadius: BorderRadius.circular(24),
        padding: const EdgeInsets.fromLTRB(16, 6, 6, 6),
        child: Row(
          children: [
            Expanded(
              child: TextField(
                controller: controller,
                style: TextStyle(fontSize: 13.5, color: fg),
                minLines: 1,
                maxLines: 4,
                textInputAction: TextInputAction.newline,
                decoration: InputDecoration(
                  border: InputBorder.none,
                  hintText: busy ? '生成中…' : '问点什么',
                  hintStyle: TextStyle(fontSize: 12.5, color: fg.withValues(alpha: 0.35)),
                ),
                onSubmitted: (_) => onSend(),
              ),
            ),
            const SizedBox(width: 6),
            busy
                ? GestureDetector(
                    onTap: onStop,
                    child: Container(
                      width: 38,
                      height: 38,
                      decoration: BoxDecoration(shape: BoxShape.circle, color: Colors.redAccent.withValues(alpha: 0.85)),
                      child: const Icon(Icons.stop_rounded, size: 18, color: Colors.white),
                    ),
                  )
                : GestureDetector(
                    onTap: onSend,
                    child: Container(
                      width: 38,
                      height: 38,
                      decoration: BoxDecoration(shape: BoxShape.circle, color: kPrimary),
                      child: const Icon(Icons.arrow_upward_rounded, size: 19, color: Colors.white),
                    ),
                  ),
          ],
        ),
      ),
    );
  }
}
