import 'package:flutter_test/flutter_test.dart';

import 'package:byte_memos_mobile/util/checklist.dart';

void main() {
  group('checklist 助手', () {
    const md = '''
# 周会记录

- [ ] 发周报
- [x] 定场地
  - [ ] 订水
* [ ] 复盘归档
1. [ ] 下次排期

（非清单行）[ ] 不算勾选框
''';

    test('计数：未勾选 4 / 总数 5', () {
      expect(countUnchecked(md), 4);
      expect(countAll(md), 5);
    });

    test('翻转第 0 个（- 转 x），其余不动', () {
      final out = toggleCheckbox(md, 0);
      expect(out, contains('- [x] 发周报'));
      expect(out, contains('- [x] 定场地'));
      expect(out, contains('  - [ ] 订水'));
      expect(countUnchecked(out), 3);
    });

    test('翻转第 2 个（嵌套缩进项）', () {
      final out = toggleCheckbox(md, 2);
      expect(out, contains('  - [x] 订水'));
    });

    test('翻转第 4 个（有序列表前缀）', () {
      final out = toggleCheckbox(md, 4);
      expect(out, contains('1. [x] 下次排期'));
    });

    test('x 大小写都能识别为已勾选并翻回', () {
      const md2 = '- [X] 大写勾';
      expect(countUnchecked(md2), 0);
      expect(toggleCheckbox(md2, 0), contains('- [ ] 大写勾'));
    });

    test('越界返回原内容', () {
      expect(toggleCheckbox(md, 99), md);
    });

    test('追加清单：无「## 待办」段时追加', () {
      final out = appendChecklist('正文', '- [ ] A');
      expect(out, contains('## 待办'));
      expect(out.endsWith('- [ ] A\n'), isTrue);
    });

    test('追加清单：已有段时替换不重复', () {
      final once = appendChecklist('正文', '- [ ] A');
      final twice = appendChecklist(once, '- [ ] B');
      expect('## 待办'.allMatches(twice).length, 1);
      expect(twice, contains('- [ ] B'));
      expect(twice, isNot(contains('- [ ] A')));
    });
  });
}
