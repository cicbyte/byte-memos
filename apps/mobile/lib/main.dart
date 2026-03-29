/// 入口：ProviderScope + 主题 + 外壳
library;

import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'design/theme.dart';
import 'pages/shell.dart';
import 'share/share_incoming.dart';
import 'state/conn_state.dart';

void main() {
  runApp(const ProviderScope(child: ByteMemosApp()));
}

class ByteMemosApp extends ConsumerWidget {
  const ByteMemosApp({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final themeMode = ref.watch(themeModeProvider);
    return MaterialApp(
      title: 'byte-memos',
      debugShowCheckedModeBanner: false,
      theme: AppTheme.light(),
      darkTheme: AppTheme.dark(),
      themeMode: themeMode,
      localizationsDelegates: const [
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: const [Locale('zh', 'CN'), Locale('en', 'US')],
      home: const _Home(),
    );
  }
}

class _Home extends ConsumerStatefulWidget {
  const _Home();

  @override
  ConsumerState<_Home> createState() => _HomeState();
}

class _HomeState extends ConsumerState<_Home> {
  @override
  void initState() {
    super.initState();
    // 系统分享进来的文字（F06）：预填新建弹层
    bindShareIncoming((text) {
      if (!mounted || text.trim().isEmpty) return;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted) return;
        // 延后到 shell 渲染完成再弹层
        showShareSheet(context, ref, text);
      });
    });
  }

  @override
  Widget build(BuildContext context) {
    return const AppShell();
  }
}
