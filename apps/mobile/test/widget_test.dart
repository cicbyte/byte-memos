// 冒烟测试：玻璃容器三档令牌 + 降级渲染不抛异常
import 'package:byte_memos_mobile/design/glass.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('GlassContainer 三档渲染', (tester) async {
    for (final tier in GlassTier.values) {
      for (final pref in GlassPreference.values) {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: GlassContainer(
                tier: tier,
                preference: pref,
                child: const Text('glass'),
              ),
            ),
          ),
        );
        expect(find.text('glass'), findsOneWidget);
      }
    }
  });
}
