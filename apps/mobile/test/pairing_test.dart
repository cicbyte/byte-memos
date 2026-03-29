import 'package:flutter_test/flutter_test.dart';
import 'package:byte_memos_mobile/conn/pairing.dart';

void main() {
  group('parsePairing', () {
    test('bmemos://pair 配对串（桌面端 buildPairingString 同构）', () {
      // 模拟桌面端 new URLSearchParams({...}).toString()（空格转 +、中文百分号编码）
      const raw = 'bmemos://pair?v=1&name=DESKTOP-ZHYJ&host=192.168.1.5&port=8318&key=H2P3C5wEDd60xCwrnmfNfmGWcWD34EkX';
      final p = parsePairing(raw);
      expect(p, isNotNull);
      expect(p!.name, 'DESKTOP-ZHYJ');
      expect(p.baseUrl, 'http://192.168.1.5:8318');
      expect(p.apiKey, 'H2P3C5wEDd60xCwrnmfNfmGWcWD34EkX');
    });

    test('中文名与编码字符', () {
      const raw = 'bmemos://pair?v=1&name=%E5%85%AC%E5%8F%B8%E7%94%B5%E8%84%91&host=10.0.0.8&port=8318&key=';
      final p = parsePairing(raw);
      expect(p, isNotNull);
      expect(p!.name, '公司电脑');
      expect(p.apiKey, '');
    });

    test('JSON 形态', () {
      const raw = '{"v":1,"name":"test","host":"127.0.0.1","port":8318,"key":"k1"}';
      final p = parsePairing(raw);
      expect(p!.baseUrl, 'http://127.0.0.1:8318');
      expect(p.apiKey, 'k1');
    });

    test('裸 URL 兜底', () {
      final p = parsePairing('http://192.168.1.10:8318');
      expect(p!.baseUrl, 'http://192.168.1.10:8318');
    });

    test('非法输入返回 null', () {
      expect(parsePairing(''), isNull);
      expect(parsePairing('hello world'), isNull);
      expect(parsePairing('bmemos://other?a=1'), isNull);
      expect(parsePairing('bmemos://pair?host=&port=8318'), isNull);
    });
  });
}
