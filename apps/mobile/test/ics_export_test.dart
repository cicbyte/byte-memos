/// 待办外发 .ics（T4）单测：结构/转义/CRLF/空清单短路
library;

import 'package:flutter_test/flutter_test.dart';
import 'package:byte_memos_mobile/conn/models.dart';
import 'package:byte_memos_mobile/util/checklist.dart';
import 'package:byte_memos_mobile/util/ics_export.dart';

Memo _memo(String content) => Memo(
      shortId: 'Ab12Cd',
      title: '出差准备',
      content: content,
      visibility: 'PRIVATE',
      pinned: false,
      tags: const [],
      createdAt: '2026-10-26T10:00:00Z',
      updatedAt: '2026-10-26T10:00:00Z',
    );

void main() {
  test('checklistItems 提取文本与勾选态', () {
    final items = checklistItems('- [ ] 买票\n正文\n- [x] 拿证件\n1. [ ] 订酒店');
    expect(items.length, 3);
    expect(items[0], ('买票', false));
    expect(items[1], ('拿证件', true));
    expect(items[2].$1, '订酒店');
  });

  test('buildTodoIcs 只导出未完成项，CRLF 行尾 + VEVENT 数正确', () {
    final ics = buildTodoIcs(_memo('- [x] 拿证件\n- [ ] 买票\n- [ ] 订酒店'));
    expect(ics, isNotEmpty);
    expect(ics.split('BEGIN:VEVENT').length - 1, 2);
    expect(ics.contains('SUMMARY:买票\r\n'), isTrue);
    expect(ics.contains('拿证件'), isFalse);
    expect(ics.endsWith('END:VCALENDAR\r\n'), isTrue);
    // UID 幂等（备忘+序号）
    expect(ics.contains('UID:bm-Ab12Cd-0@byte-memos'), isTrue);
  });

  test('文本转义：逗号/分号', () {
    final ics = buildTodoIcs(_memo('- [ ] 周一, 交报告; 抄送'));
    expect(ics.contains(r'周一\, 交报告\; 抄送'), isTrue);
  });

  test('全勾选/无清单 → 空串短路', () {
    expect(buildTodoIcs(_memo('- [x] 已完成')), '');
    expect(buildTodoIcs(_memo('普通文本')), '');
  });
}
