/// 待办外发 .ics（T4：AI 抽待办 → 系统日历/钉钉，外发不内建）
///
/// 纯本地生成 RFC 5545 文本（CRLF 行尾），零新依赖；
/// 每个待办一条全天 VEVENT（兼容性优于 VTODO），UID 按 备忘+序号 幂等。
library;

import '../conn/models.dart';
import 'checklist.dart';

/// 生成 .ics：未完成待办 → 全天事件（明天开始，可导入后自行拖期）
String buildTodoIcs(Memo memo, {DateTime? day}) {
  final todos = checklistItems(memo.content).where((it) => !it.$2).map((it) => it.$1).toList();
  if (todos.isEmpty) return '';
  final d = (day ?? DateTime.now().add(const Duration(days: 1)));
  final date = '${d.year}${_two(d.month)}${_two(d.day)}';
  final stamp = _utcStamp(DateTime.now());
  final buf = StringBuffer()
    ..write('BEGIN:VCALENDAR\r\n')
    ..write('VERSION:2.0\r\n')
    ..write('PRODID:-//byte-memos//TODO Export//CN\r\n')
    ..write('CALSCALE:GREGORIAN\r\n')
    ..write('METHOD:PUBLISH\r\n')
    ..write('X-WR-CALNAME:${_esc(memo.title.isEmpty ? 'byte-memos 待办' : memo.title)}\r\n');
  for (var i = 0; i < todos.length; i++) {
    buf
      ..write('BEGIN:VEVENT\r\n')
      ..write('UID:bm-${memo.shortId}-$i@byte-memos\r\n')
      ..write('DTSTAMP:$stamp\r\n')
      ..write('DTSTART;VALUE=DATE:$date\r\n')
      ..write('DTEND;VALUE=DATE:$date\r\n')
      ..write('SUMMARY:${_esc(todos[i])}\r\n')
      ..write('DESCRIPTION:${_esc('来自备忘：${memo.title.isEmpty ? memo.shortId : memo.title}')}\r\n')
      ..write('STATUS:CONFIRMED\r\n')
      ..write('TRANSP:TRANSPARENT\r\n')
      ..write('END:VEVENT\r\n');
  }
  buf.write('END:VCALENDAR\r\n');
  return buf.toString();
}

String _two(int v) => v.toString().padLeft(2, '0');

String _utcStamp(DateTime t) {
  final u = t.toUtc();
  return '${u.year}${_two(u.month)}${_two(u.day)}T${_two(u.hour)}${_two(u.minute)}${_two(u.second)}Z';
}

/// RFC 5545 文本转义：反斜杠/分号/逗号/换行
String _esc(String s) => s
    .replaceAll(r'\', r'\\')
    .replaceAll(';', r'\;')
    .replaceAll(',', r'\,')
    .replaceAll('\n', r'\n');
