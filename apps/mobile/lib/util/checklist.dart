/// Markdown 勾选清单助手（轻量待办）
///
/// 识别行首任务列表语法：`- [ ]` / `- [x]`（兼容 * + 和有序列表前缀、缩进）。
/// 序号按文档顺序从 0 递增，与 MarkdownBody 的 checkboxBuilder 构建顺序一致。
library;

final _boxRe = RegExp(r'^(\s*(?:[-*+]|\d+[.)])\s+)\[( |x|X)\](?=\s)');

/// 第 n 个勾选框所在行号；找不到返回 -1
int _lineOfNth(String content, int n) {
  var idx = 0;
  final lines = content.split('\n');
  for (var i = 0; i < lines.length; i++) {
    if (_boxRe.hasMatch(lines[i])) {
      if (idx == n) return i;
      idx++;
    }
  }
  return -1;
}

/// 未勾选数（待办视图/统计卡用）
int countUnchecked(String content) {
  var n = 0;
  for (final line in content.split('\n')) {
    final m = _boxRe.firstMatch(line);
    if (m != null && m.group(2) != 'x' && m.group(2) != 'X') n++;
  }
  return n;
}

/// 勾选框总数
int countAll(String content) {
  var n = 0;
  for (final line in content.split('\n')) {
    if (_boxRe.hasMatch(line)) n++;
  }
  return n;
}

/// 勾选清单条目（文档顺序）：(文本, 是否已勾)
List<(String, bool)> checklistItems(String content) {
  final items = <(String, bool)>[];
  for (final line in content.split('\n')) {
    final m = _boxRe.firstMatch(line);
    if (m != null) {
      final text = line.substring(m.end).trim();
      if (text.isNotEmpty) {
        items.add((text, m.group(2) == 'x' || m.group(2) == 'X'));
      }
    }
  }
  return items;
}

/// 翻转第 n 个勾选框（0 基，文档顺序）；越界返回原内容
String toggleCheckbox(String content, int n) {
  final lineNo = _lineOfNth(content, n);
  if (lineNo < 0) return content;
  final lines = content.split('\n');
  final m = _boxRe.firstMatch(lines[lineNo])!;
  final checked = m.group(2) == 'x' || m.group(2) == 'X';
  lines[lineNo] = lines[lineNo].replaceFirst('[${m.group(2)}]', checked ? '[ ]' : '[x]');
  return lines.join('\n');
}

/// 追加 AI 抽取的待办清单（幂等：已有「## 待办」段则替换该段之后的旧清单）
String appendChecklist(String content, String checklist) {
  final clean = checklist.trim();
  if (content.contains('## 待办')) {
    final i = content.indexOf('## 待办');
    return '${content.substring(0, i)}## 待办\n$clean\n';
  }
  return '${content.trimRight()}\n\n## 待办\n$clean\n';
}
