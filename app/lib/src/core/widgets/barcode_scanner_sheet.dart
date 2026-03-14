import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
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
              if (resolved) return;

              final String? scannedValue = extractRawBarcode(capture);
              if (scannedValue == null) return;

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

  /// 连续扫码模式。
  ///
  /// [onScanned] 回调语义：
  /// - 返回非 null String → 命中成功，该字符串用作弹窗内显示标签（如商品名 + 数量）
  /// - 返回 null           → 未命中 / 处理失败，弹窗内以红色提示
  static Future<void> scanContinuous(
    BuildContext context, {
    String title = '连续扫码',
    String hint = '将条码对准取景框，系统会持续识别；完成后手动结束。',
    int deduplicateWindowMs = 1200,
    required FutureOr<String?> Function(String barcode) onScanned,
  }) async {
    final MobileScannerController cameraController = MobileScannerController(
      // 连续扫码必须用 normal，noDuplicates 会在扫到一个码后暂停检测
      // 直到该条码离开取景框，导致用户需要手动触屏才能继续扫下一个。
      // 去重由内部 deduplicateWindowMs 时间窗口负责。
      detectionSpeed: DetectionSpeed.normal,
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

// ════════════════════════════════════════════════════════════════════════════
// 手电筒按钮（通用，叠加在取景框右上角）
// ════════════════════════════════════════════════════════════════════════════

class _TorchButton extends StatelessWidget {
  const _TorchButton({required this.controller});

  final MobileScannerController controller;

  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<MobileScannerState>(
      valueListenable: controller,
      builder: (_, MobileScannerState state, __) {
        // 手电筒不可用时（如前置摄像头）不显示按钮
        if (!state.isInitialized || state.torchState == TorchState.unavailable) {
          return const SizedBox.shrink();
        }

        final bool isOn = state.torchState == TorchState.on;

        return GestureDetector(
          onTap: () => controller.toggleTorch(),
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 200),
            width: 44,
            height: 44,
            decoration: BoxDecoration(
              color: isOn
                  ? const Color(0xFFFBBF24).withValues(alpha: 0.92)
                  : Colors.black.withValues(alpha: 0.45),
              shape: BoxShape.circle,
            ),
            child: Icon(
              isOn ? Icons.flashlight_on_rounded : Icons.flashlight_off_rounded,
              color: isOn ? Colors.black : Colors.white,
              size: 22,
            ),
          ),
        );
      },
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// 取景框 + 右上角手电筒叠加
// ════════════════════════════════════════════════════════════════════════════

Widget _buildCameraView({
  required MobileScannerController controller,
  required void Function(BarcodeCapture) onDetect,
}) {
  return Stack(
    children: <Widget>[
      // 取景框
      Positioned.fill(
        child: ClipRRect(
          borderRadius: BorderRadius.circular(12),
          child: MobileScanner(
            controller: controller,
            onDetect: onDetect,
          ),
        ),
      ),
      // 手电筒按钮（右上角）
      Positioned(
        top: 10,
        right: 10,
        child: _TorchButton(controller: controller),
      ),
    ],
  );
}

// ════════════════════════════════════════════════════════════════════════════
// 单次扫码弹窗
// ════════════════════════════════════════════════════════════════════════════

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
            Text(
              title,
              style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 4),
            Text(hint),
            const SizedBox(height: 12),
            Expanded(
              child: _buildCameraView(
                controller: controller,
                onDetect: onDetect,
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

// ════════════════════════════════════════════════════════════════════════════
// 扫码历史条目
// ════════════════════════════════════════════════════════════════════════════

class _ScanEntry {
  _ScanEntry({
    required this.barcode,
    required this.label, // null = 未命中
    required this.time,
  });

  final String barcode;
  final String? label;
  final DateTime time;

  bool get success => label != null;
}

// ════════════════════════════════════════════════════════════════════════════
// 连续扫码弹窗（优化版）
// ════════════════════════════════════════════════════════════════════════════

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
  final FutureOr<String?> Function(String barcode) onScanned;

  @override
  State<_ContinuousBarcodeScannerBottomSheet> createState() =>
      _ContinuousBarcodeScannerBottomSheetState();
}

class _ContinuousBarcodeScannerBottomSheetState
    extends State<_ContinuousBarcodeScannerBottomSheet> {
  bool _processing = false;
  int _successCount = 0;
  String _lastBarcode = '';
  int _lastAtMs = 0;

  /// 最近 4 条扫码记录（最新在最前）
  final List<_ScanEntry> _recentScans = [];
  static const int _maxHistory = 4;

  Future<void> _onDetect(BarcodeCapture capture) async {
    if (_processing) return;

    final String? barcode = BarcodeScannerSheet.extractRawBarcode(capture);
    if (barcode == null) return;

    final int now = DateTime.now().millisecondsSinceEpoch;
    if (_lastBarcode == barcode &&
        now - _lastAtMs <= widget.deduplicateWindowMs) {
      return;
    }

    _lastBarcode = barcode;
    _lastAtMs = now;
    _processing = true;

    try {
      final String? label = await widget.onScanned(barcode);
      if (!mounted) return;

      final entry = _ScanEntry(
        barcode: barcode,
        label: label,
        time: DateTime.now(),
      );

      if (label != null) {
        // ✅ 成功：轻触觉
        HapticFeedback.lightImpact();
        setState(() {
          _successCount += 1;
          _recentScans.insert(0, entry);
          if (_recentScans.length > _maxHistory) _recentScans.removeLast();
        });
      } else {
        // ❌ 未命中：中等触觉（与成功不同，让手感提示操作员注意）
        HapticFeedback.mediumImpact();
        setState(() {
          _recentScans.insert(0, entry);
          if (_recentScans.length > _maxHistory) _recentScans.removeLast();
        });
      }
    } finally {
      _processing = false;
    }
  }

  String _fmtTime(DateTime t) {
    final h = t.hour.toString().padLeft(2, '0');
    final m = t.minute.toString().padLeft(2, '0');
    final s = t.second.toString().padLeft(2, '0');
    return '$h:$m:$s';
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final double sheetHeight = MediaQuery.of(context).size.height * 0.88;
    final _ScanEntry? lastEntry =
        _recentScans.isNotEmpty ? _recentScans.first : null;

    return SizedBox(
      height: sheetHeight,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            // ── 拖拽指示条 ──────────────────────────────────────────
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

            // ── 标题行 + 成功计数 ───────────────────────────────────
            Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    widget.title,
                    style: const TextStyle(
                        fontSize: 16, fontWeight: FontWeight.w600),
                  ),
                ),
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                  decoration: BoxDecoration(
                    color: const Color(0xFF10B981).withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(999),
                  ),
                  child: Text(
                    '已入库 $_successCount 条',
                    style: const TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w700,
                      color: Color(0xFF059669),
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              widget.hint,
              style: TextStyle(fontSize: 12, color: cs.onSurfaceVariant),
            ),
            const SizedBox(height: 10),

            // ── 实时状态条（最新一条结果） ──────────────────────────
            AnimatedSwitcher(
              duration: const Duration(milliseconds: 250),
              child: lastEntry == null
                  ? _buildWaitingBanner(cs)
                  : _buildStatusBanner(cs, lastEntry),
            ),
            const SizedBox(height: 10),

            // ── 摄像头取景框（含手电筒按钮）──────────────────────────
            Expanded(
              child: _buildCameraView(
                controller: widget.controller,
                onDetect: _onDetect,
              ),
            ),
            const SizedBox(height: 10),

            // ── 最近扫码记录列表 ────────────────────────────────────
            if (_recentScans.isNotEmpty) ...<Widget>[
              Text(
                '最近扫码记录',
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                  color: cs.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 6),
              ..._recentScans.map((e) => _buildHistoryRow(cs, e)),
              const SizedBox(height: 6),
            ],

            // ── 结束按钮 ────────────────────────────────────────────
            SizedBox(
              width: double.infinity,
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

  Widget _buildWaitingBanner(ColorScheme cs) {
    return Container(
      key: const ValueKey('waiting'),
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: cs.surfaceContainerHighest.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(10),
      ),
      child: Text(
        '等待扫码…',
        style: TextStyle(fontSize: 13, color: cs.onSurfaceVariant),
      ),
    );
  }

  Widget _buildStatusBanner(ColorScheme cs, _ScanEntry entry) {
    final isOk = entry.success;
    final bannerColor = isOk ? const Color(0xFF10B981) : cs.error;

    return AnimatedContainer(
      key: ValueKey(entry.time),
      duration: const Duration(milliseconds: 200),
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: bannerColor.withValues(alpha: 0.1),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: bannerColor.withValues(alpha: 0.4)),
      ),
      child: Row(
        children: <Widget>[
          Icon(
            isOk ? Icons.check_circle_rounded : Icons.error_outline_rounded,
            size: 16,
            color: bannerColor,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  isOk ? entry.label! : '未命中：${entry.barcode}',
                  style: TextStyle(
                    fontSize: 13,
                    fontWeight: FontWeight.w600,
                    color: bannerColor,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
                if (!isOk)
                  Text(
                    '该条码未建档，请退出后手工录入或新建商品',
                    style: TextStyle(
                        fontSize: 11,
                        color: cs.error.withValues(alpha: 0.8)),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildHistoryRow(ColorScheme cs, _ScanEntry e) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4),
      child: Row(
        children: <Widget>[
          Icon(
            e.success
                ? Icons.check_circle_outline_rounded
                : Icons.highlight_off_rounded,
            size: 14,
            color: e.success ? const Color(0xFF10B981) : cs.error,
          ),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              e.success ? e.label! : e.barcode,
              style: TextStyle(
                fontSize: 12,
                color: e.success ? cs.onSurface : cs.error,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          Text(
            _fmtTime(e.time),
            style: TextStyle(fontSize: 11, color: cs.onSurfaceVariant),
          ),
        ],
      ),
    );
  }
}
