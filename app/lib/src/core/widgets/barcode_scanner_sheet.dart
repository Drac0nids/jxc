import 'dart:async';

import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

class BarcodeScannerSheet {
  const BarcodeScannerSheet._();

  // 限制识别类型：仅识别标准商品条码(EAN/UPC)、通用的一维码(Code128/39)与二维码。
  // 可以有效防止错扫外箱上的 ITF-14 内部物流码、Codabar 快递单号或其他冷门条码。
  static const List<BarcodeFormat> _supportedFormats = <BarcodeFormat>[
    BarcodeFormat.ean13,
    BarcodeFormat.ean8,
    BarcodeFormat.upcA,
    BarcodeFormat.upcE,
    BarcodeFormat.code128,
    BarcodeFormat.code39,
    BarcodeFormat.qrCode,
  ];

  static Future<String?> scan(
    BuildContext context, {
    String title = '手机扫码',
    String hint = '将条码对准取景框，识别成功后会自动回填。',
  }) async {
    final MobileScannerController cameraController = MobileScannerController(
      detectionSpeed: DetectionSpeed.noDuplicates,
      formats: _supportedFormats,
    );
    bool resolved = false;

    try {
      final String? value = await showModalBottomSheet<String>(
        context: context,
        useSafeArea: true,
        isScrollControlled: true,
        builder: (BuildContext sheetContext) {
          return _BarcodeScannerBottomSheet(
            controller: cameraController,
            title: title,
            hint: hint,
            onDetect: (BarcodeCapture capture) {
              if (resolved) {
                return;
              }

              final String? scannedValue = extractRawBarcode(capture);
              if (scannedValue == null) {
                return;
              }

              resolved = true;
              Navigator.of(sheetContext).pop(scannedValue);
            },
          );
        },
      );

      return value?.trim();
    } finally {
      cameraController.dispose();
    }
  }

  static Future<void> scanContinuous(
    BuildContext context, {
    String title = '连续扫码',
    String hint = '将条码对准取景框，系统会持续识别；完成后手动结束。',
    int deduplicateWindowMs = 1200,
    required FutureOr<bool> Function(String barcode) onScanned,
  }) async {
    final MobileScannerController cameraController = MobileScannerController(
      detectionSpeed: DetectionSpeed.noDuplicates,
      formats: _supportedFormats,
    );

    try {
      await showModalBottomSheet<void>(
        context: context,
        useSafeArea: true,
        isScrollControlled: true,
        builder: (BuildContext sheetContext) {
          return _ContinuousBarcodeScannerBottomSheet(
            controller: cameraController,
            title: title,
            hint: hint,
            deduplicateWindowMs: deduplicateWindowMs,
            onScanned: onScanned,
          );
        },
      );
    } finally {
      cameraController.dispose();
    }
  }

  static String? extractRawBarcode(BarcodeCapture capture) {
    for (final Barcode code in capture.barcodes) {
      final String? rawValue = code.rawValue?.trim();
      if (rawValue != null && rawValue.isNotEmpty) {
        return rawValue;
      }
    }

    return null;
  }
}

class _BarcodeScannerBottomSheet extends StatelessWidget {
  const _BarcodeScannerBottomSheet({
    required this.controller,
    required this.title,
    required this.hint,
    required this.onDetect,
  });

  final MobileScannerController controller;
  final String title;
  final String hint;
  final void Function(BarcodeCapture capture) onDetect;

  @override
  Widget build(BuildContext context) {
    final double sheetHeight = MediaQuery.of(context).size.height * 0.78;

    return SizedBox(
      height: sheetHeight,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Center(
              child: Container(
                width: 36,
                height: 4,
                decoration: BoxDecoration(
                  color: Theme.of(context).dividerColor,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
            ),
            const SizedBox(height: 12),
            Text(title,
                style:
                    const TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
            const SizedBox(height: 4),
            Text(hint),
            const SizedBox(height: 12),
            Expanded(
              child: ClipRRect(
                borderRadius: BorderRadius.circular(12),
                child: MobileScanner(
                  controller: controller,
                  onDetect: onDetect,
                ),
              ),
            ),
            const SizedBox(height: 12),
            Align(
              alignment: Alignment.centerRight,
              child: OutlinedButton(
                onPressed: () => Navigator.of(context).pop(),
                child: const Text('取消'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ContinuousBarcodeScannerBottomSheet extends StatefulWidget {
  const _ContinuousBarcodeScannerBottomSheet({
    required this.controller,
    required this.title,
    required this.hint,
    required this.deduplicateWindowMs,
    required this.onScanned,
  });

  final MobileScannerController controller;
  final String title;
  final String hint;
  final int deduplicateWindowMs;
  final FutureOr<bool> Function(String barcode) onScanned;

  @override
  State<_ContinuousBarcodeScannerBottomSheet> createState() =>
      _ContinuousBarcodeScannerBottomSheetState();
}

class _ContinuousBarcodeScannerBottomSheetState
    extends State<_ContinuousBarcodeScannerBottomSheet> {
  bool _processing = false;
  int _processedCount = 0;
  String _lastBarcode = '';
  int _lastAtMs = 0;

  Future<void> _onDetect(BarcodeCapture capture) async {
    if (_processing) {
      return;
    }

    final String? barcode = BarcodeScannerSheet.extractRawBarcode(capture);
    if (barcode == null) {
      return;
    }

    final int now = DateTime.now().millisecondsSinceEpoch;
    if (_lastBarcode == barcode &&
        now - _lastAtMs <= widget.deduplicateWindowMs) {
      return;
    }

    _lastBarcode = barcode;
    _lastAtMs = now;
    _processing = true;
    try {
      final bool processed = await widget.onScanned(barcode);
      if (!mounted) {
        return;
      }
      if (processed) {
        setState(() {
          _processedCount += 1;
        });
      }
    } finally {
      _processing = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final double sheetHeight = MediaQuery.of(context).size.height * 0.82;

    return SizedBox(
      height: sheetHeight,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Center(
              child: Container(
                width: 36,
                height: 4,
                decoration: BoxDecoration(
                  color: Theme.of(context).dividerColor,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
            ),
            const SizedBox(height: 12),
            Text(
              widget.title,
              style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 4),
            Text(widget.hint),
            const SizedBox(height: 8),
            Text('已处理：$_processedCount 条'),
            const SizedBox(height: 12),
            Expanded(
              child: ClipRRect(
                borderRadius: BorderRadius.circular(12),
                child: MobileScanner(
                  controller: widget.controller,
                  onDetect: _onDetect,
                ),
              ),
            ),
            const SizedBox(height: 12),
            Align(
              alignment: Alignment.centerRight,
              child: FilledButton.tonal(
                onPressed: () => Navigator.of(context).pop(),
                child: const Text('结束扫码'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
