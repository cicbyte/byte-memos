/// AI 对话状态（D3）：消息历史 + SSE 流式接收 + 模型选择 + 可停止
library;

import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../conn/models.dart';
import 'conn_state.dart';

/// 供应商列表（在线时自动刷新）
final aiProvidersProvider = FutureProvider<List<AiProvider>>((ref) async {
  final client = ref.watch(connProvider).client;
  if (client == null) throw Exception('离线');
  return client.aiProviders();
});

/// 当前选择 (providerId, modelName)；null = 用服务端默认（已启用的供应商）
/// 持久化（重装/换机前提下的本地记忆）
final aiSelectionProvider = StateNotifierProvider<AiSelectionNotifier, (int, String)?>((ref) {
  return AiSelectionNotifier(ref);
});

class AiSelectionNotifier extends StateNotifier<(int, String)?> {
  AiSelectionNotifier(this._ref) : super(null) {
    _load();
  }

  static const _key = 'bm.ai.selection';

  final Ref _ref;

  Future<void> _load() async {
    try {
      final prefs = await _ref.read(sharedPrefsProvider.future);
      final raw = prefs.getString(_key);
      if (raw == null) return;
      final parts = raw.split('|');
      final id = int.tryParse(parts[0]);
      if (id != null && parts.length == 2) state = (id, parts[1]);
    } catch (_) {}
  }

  Future<void> set((int, String)? sel) async {
    state = sel;
    try {
      final prefs = await _ref.read(sharedPrefsProvider.future);
      await prefs.setString(_key, sel == null ? '' : '${sel.$1}|${sel.$2}');
    } catch (_) {}
  }
}

class ChatMsg {
  ChatMsg({required this.role, required this.content});

  final String role; // user | assistant
  final String content;
}

class AiChatState {
  const AiChatState({
    this.messages = const [],
    this.streaming = false,
    this.streamBuf = '',
    this.thinkingBuf = '',
    this.error,
  });

  final List<ChatMsg> messages;
  final bool streaming;
  final String streamBuf;
  final String thinkingBuf;
  final String? error;

  AiChatState copyWith({
    List<ChatMsg>? messages,
    bool? streaming,
    String? streamBuf,
    String? thinkingBuf,
    bool clearThinking = false,
    String? error,
    bool clearError = false,
  }) =>
      AiChatState(
        messages: messages ?? this.messages,
        streaming: streaming ?? this.streaming,
        streamBuf: streamBuf ?? this.streamBuf,
        thinkingBuf: clearThinking ? '' : (thinkingBuf ?? this.thinkingBuf),
        error: clearError ? null : (error ?? this.error),
      );
}

class AiChatNotifier extends StateNotifier<AiChatState> {
  AiChatNotifier(this.ref) : super(const AiChatState());

  final Ref ref;
  CancelToken? _cancelToken;

  Future<void> send(String text) async {
    final client = ref.read(connProvider).client;
    if (client == null) {
      state = state.copyWith(error: '离线：请先连接服务');
      return;
    }
    final history = [...state.messages, ChatMsg(role: 'user', content: text)];
    state = state.copyWith(messages: history, streaming: true, clearError: true);

    final sel = ref.read(aiSelectionProvider);
    final token = CancelToken();
    _cancelToken = token;

    try {
      await client.aiChatStream(
        history.map((m) => {'role': m.role, 'content': m.content}).toList(),
        providerId: sel?.$1,
        model: sel?.$2,
        cancelToken: token,
        onEvent: _onEvent,
      );
      // 流自然结束但没收到 done（连接中断）：落袋为安
      if (state.streaming) {
        state = state.copyWith(
          messages: state.streamBuf.isEmpty
              ? state.messages
              : [...state.messages, ChatMsg(role: 'assistant', content: state.streamBuf)],
          streaming: false,
          streamBuf: '',
          clearThinking: true,
          error: '连接中断',
        );
      }
    } on DioException catch (e) {
      if (CancelToken.isCancel(e)) {
        // 用户主动停止：保留半截内容
        state = state.copyWith(
          messages: state.streamBuf.isEmpty
              ? state.messages
              : [...state.messages, ChatMsg(role: 'assistant', content: state.streamBuf)],
          streaming: false,
          streamBuf: '',
          clearThinking: true,
        );
      } else {
        state = state.copyWith(streaming: false, error: '发送失败：${e.message ?? e}');
      }
    } catch (e) {
      state = state.copyWith(streaming: false, error: '发送失败：$e');
    } finally {
      _cancelToken = null;
    }
  }

  void _onEvent(String event, Map<String, dynamic> data) {
    switch (event) {
      case 'delta':
        state = state.copyWith(streamBuf: state.streamBuf + (data['text'] as String? ?? ''));
      case 'thinking':
        state = state.copyWith(thinkingBuf: state.thinkingBuf + (data['text'] as String? ?? ''));
      case 'done':
        final full = data['full'] as String? ?? state.streamBuf;
        state = state.copyWith(
          messages: [...state.messages, ChatMsg(role: 'assistant', content: full)],
          streaming: false,
          streamBuf: '',
          clearThinking: true,
        );
      case 'error':
        final msg = data['message'] as String? ?? '未知错误';
        // 出错也保留已生成的半截内容
        state = state.copyWith(
          messages: state.streamBuf.isEmpty
              ? state.messages
              : [...state.messages, ChatMsg(role: 'assistant', content: state.streamBuf)],
          streaming: false,
          streamBuf: '',
          clearThinking: true,
          error: msg,
        );
    }
  }

  /// 用户主动停止：掐断 SSE 订阅（服务端检测断开后 CancellationToken 取消上游）
  void stop() {
    _cancelToken?.cancel();
  }

  void reset() {
    _cancelToken?.cancel();
    state = const AiChatState();
  }
}

final aiChatProvider = StateNotifierProvider<AiChatNotifier, AiChatState>(AiChatNotifier.new);
