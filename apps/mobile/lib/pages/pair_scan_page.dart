/// 扫码配对页：识别 bmemos://pair 二维码（mobile_scanner）
library;

import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../conn/pairing.dart';
import '../design/theme.dart';

class PairScanPage extends StatefulWidget {
  const PairScanPage({super.key});

  @override
  State<PairScanPage> createState() => _PairScanPageState();
}

class _PairScanPageState extends State<PairScanPage> {
  bool _done = false;

  void _onDetect(BarcodeCapture capture) {
    if (_done) return;
    final raw = capture.barcodes.firstOrNull?.rawValue;
    if (raw == null) return;
    final payload = parsePairing(raw);
    if (payload == null) return; // 非配对码继续扫
    _done = true;
    Navigator.of(context).pop(payload);
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      appBar: AppBar(
        backgroundColor: Colors.black,
        foregroundColor: Colors.white,
        title: const Text('扫码配对', style: TextStyle(fontSize: 15, color: Colors.white)),
        iconTheme: const IconThemeData(color: Colors.white),
      ),
      body: Stack(
        children: [
          MobileScanner(onDetect: _onDetect),
          // 取景框提示
          Center(
            child: Container(
              width: 230,
              height: 230,
              decoration: BoxDecoration(
                border: Border.all(color: kPrimary.withValues(alpha: 0.9), width: 2.5),
                borderRadius: BorderRadius.circular(18),
              ),
            ),
            // ignore: avoid_unnecessary_containers
          ),
          Positioned(
            left: 0,
            right: 0,
            bottom: 90,
            child: Text(
              '对准桌面端「REST API 服务」页的二维码',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 13, color: Colors.white.withValues(alpha: 0.85)),
            ),
          ),
        ],
      ),
    );
  }
}
