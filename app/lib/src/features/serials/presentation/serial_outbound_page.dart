import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../../../core/widgets/brand_ui.dart';
import '../models/serial_models.dart';
import '../models/serial_repository.dart';
import 'serial_logs_page.dart';

class SerialOutboundPage extends StatefulWidget {
  const SerialOutboundPage({super.key, required this.repository});
  final SerialRepository repository;

  @override
  State<SerialOutboundPage> createState() => _SerialOutboundPageState();
}

class _SerialOutboundPageState extends State<SerialOutboundPage> {
  // ── 扫码会话 ──────────────────────────────────────────────────────────────
  final List<_OutboundEntry> _entries = [];
  bool _scanning = false;
  MobileScannerController? _scanCtrl;
  String? _lastMsg;
  bool _lastIsError = false;
  bool _isLookingUp = false;

  static const int _deduplicateWindowMs = 1500;
  String _lastScanCode = '';
  int _lastScanAtMs = 0;

  // ── 出售价 ────────────────────────────────────────────────────────────────
  final _priceCtrl = TextEditingController();
  bool _showPriceField = false;

  // ── 提交 ──────────────────────────────────────────────────────────────────
  bool _submitting = false;
  bool _done = false;
  String? _submitResult;

  @override
  void dispose() {
    _priceCtrl.dispose();
    _scanCtrl?.dispose();
    super.dispose();
  }

  // ── 扫码 ──────────────────────────────────────────────────────────────────

  void _startScanning() {
    _scanCtrl = MobileScannerController(detectionSpeed: DetectionSpeed.normal);
    setState(() {
      _scanning = true;
      _lastMsg = null;
    });
  }

  void _stopScanning() {
    _scanCtrl?.stop();
    _scanCtrl?.dispose();
    _scanCtrl = null;
    setState(() => _scanning = false);
  }

  void _onDetect(BarcodeCapture cap) {
    final code = (cap.barcodes.firstOrNull?.rawValue ?? '').trim();
    if (code.isEmpty || _isLookingUp) return;

    final now = DateTime.now().millisecondsSinceEpoch;
    if (code == _lastScanCode && now - _lastScanAtMs <= _deduplicateWindowMs) {
      return;
    }
    _lastScanCode = code;
    _lastScanAtMs = now;

    _lookupAndAdd(code);
  }

  Future<void> _lookupAndAdd(String sn) async {
    // 重复检测
    if (_entries.any((e) => e.sn == sn && !e.isError)) {
      HapticFeedback.mediumImpact();
      _showMsg('序列号「$sn」已在列表中', isError: true);
      return;
    }

    setState(() => _isLookingUp = true);
    try {
      final serial = await widget.repository.findBySn(sn);
      if (serial == null) {
        HapticFeedback.mediumImpact();
        _showMsg('序列号「$sn」不存在于系统中', isError: true);
        setState(() => _entries.insert(
            0, _OutboundEntry(sn: sn, isError: true, errorMsg: '不存在')));
        return;
      }
      if (!serial.isInStock) {
        HapticFeedback.mediumImpact();
        _showMsg('序列号「$sn」当前状态 ${serial.status}，不可出库', isError: true);
        setState(() => _entries.insert(
            0,
            _OutboundEntry(
                sn: sn, isError: true, errorMsg: serial.status)));
        return;
      }

      HapticFeedback.lightImpact();
      _showMsg('✓  ${serial.sn}  已加入出库列表', isError: false);
      setState(() => _entries.insert(
          0, _OutboundEntry(sn: sn, serial: serial, isError: false)));
    } catch (e) {
      _showMsg(e.toString(), isError: true);
    } finally {
      setState(() => _isLookingUp = false);
    }
  }

  void _showMsg(String msg, {required bool isError}) {
    setState(() {
      _lastMsg = msg;
      _lastIsError = isError;
    });
  }

  void _undoLast() {
    final idx = _entries.indexWhere((e) => !e.isError);
    if (idx < 0) return;
    setState(() => _entries.removeAt(idx));
  }

  // ── 提交 ──────────────────────────────────────────────────────────────────

  Future<void> _submit() async {
    final validSns =
        _entries.where((e) => !e.isError).map((e) => e.sn).toList();
    if (validSns.isEmpty) return;

    setState(() {
      _submitting = true;
      _submitResult = null;
    });
    try {
      final result = await widget.repository.outbound(
        SerialOutboundRequest(
          sellPrice: _priceCtrl.text.trim().isEmpty
              ? null
              : _priceCtrl.text.trim(),
          sns: validSns,
        ),
      );
      setState(() {
        _submitResult = '✅ 出库成功！单号：${result.bizNo}，共 ${result.count} 台';
        _done = true;
      });
    } catch (e) {
      setState(
          () => _submitResult = '❌ ${e.toString().replaceFirst('Exception: ', '')}');
    } finally {
      setState(() => _submitting = false);
    }
  }

  // ── Build ──────────────────────────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final validCount = _entries.where((e) => !e.isError).length;

    if (_done) return _buildDoneScreen(cs);

    return Scaffold(
      backgroundColor: cs.surface,
      appBar: AppBar(
        title: const Text('序列号出库'),
        leading: BackButton(onPressed: () {
          if (_scanning) _stopScanning();
          Navigator.pop(context);
        }),
        actions: <Widget>[
          IconButton(
            tooltip: '设置出售价格',
            icon: const Icon(Icons.price_change_outlined),
            onPressed: () => setState(() => _showPriceField = !_showPriceField),
          ),
          IconButton(
            tooltip: 'SN 历史记录',
            icon: const Icon(Icons.history_rounded),
            onPressed: () => Navigator.push(
              context,
              MaterialPageRoute<void>(
                builder: (_) => SerialLogsPage(repository: widget.repository),
              ),
            ),
          ),
        ],
      ),
      floatingActionButton: validCount > 0
          ? FloatingActionButton.extended(
              onPressed: _submitting ? null : _submit,
              icon: _submitting
                  ? const SizedBox(
                      width: 20,
                      height: 20,
                      child: CircularProgressIndicator(
                          strokeWidth: 2, color: Colors.white))
                  : const Icon(Icons.upload_rounded),
              label: Text(_submitting ? '出库中...' : '提交出库（$validCount 台）'),
            )
          : null,
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: <Widget>[
          // 说明
          Container(
            padding: const EdgeInsets.all(12),
            decoration: BoxDecoration(
              color: cs.primaryContainer.withOpacity(.3),
              borderRadius: BorderRadius.circular(10),
            ),
            child: Row(
              children: <Widget>[
                Icon(Icons.info_outline, size: 18, color: cs.primary),
                const SizedBox(width: 8),
                const Expanded(
                  child: Text(
                    '直接扫描商品上的序列号（IMEI/SN），系统自动识别商品型号并验证库存',
                    style: TextStyle(fontSize: 13),
                  ),
                ),
              ],
            ),
          ),
          // 出售价字段
          if (_showPriceField) ...<Widget>[
            const SizedBox(height: 12),
            TextField(
              controller: _priceCtrl,
              keyboardType:
                  const TextInputType.numberWithOptions(decimal: true),
              decoration: const InputDecoration(
                labelText: '出售价（可选，留空用商品零售价）',
                border: OutlineInputBorder(),
                isDense: true,
                prefixText: '¥ ',
              ),
            ),
          ],
          const SizedBox(height: 12),
          // 扫码区
          SectionCard(
            title: '扫码区',
            child: Column(
              children: <Widget>[
                if (_scanning && _scanCtrl != null)
                  ClipRRect(
                    borderRadius: BorderRadius.circular(12),
                    child: SizedBox(
                      height: 220,
                      child: Stack(
                        children: <Widget>[
                          MobileScanner(
                            controller: _scanCtrl!,
                            onDetect: _onDetect,
                          ),
                          if (_isLookingUp)
                            const Center(
                              child: CircularProgressIndicator(
                                  color: Colors.white),
                            ),
                        ],
                      ),
                    ),
                  )
                else
                  GestureDetector(
                    onTap: _startScanning,
                    child: Container(
                      height: 220,
                      decoration: BoxDecoration(
                        color: cs.surfaceContainerHighest,
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: Center(
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: <Widget>[
                            Icon(Icons.qr_code_scanner,
                                size: 52, color: cs.primary),
                            const SizedBox(height: 8),
                            Text('点击开始扫码',
                                style: TextStyle(color: cs.primary)),
                          ],
                        ),
                      ),
                    ),
                  ),
                const SizedBox(height: 8),
                // 状态消息
                if (_lastMsg != null)
                  AnimatedContainer(
                    duration: const Duration(milliseconds: 300),
                    padding: const EdgeInsets.all(8),
                    decoration: BoxDecoration(
                      color: _lastIsError
                          ? cs.errorContainer
                          : Colors.green.shade50,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Row(
                      children: <Widget>[
                        Icon(
                          _lastIsError
                              ? Icons.warning_amber_outlined
                              : Icons.check_circle_outline,
                          size: 16,
                          color: _lastIsError ? cs.error : Colors.green,
                        ),
                        const SizedBox(width: 6),
                        Expanded(
                          child: Text(_lastMsg!,
                              style: TextStyle(
                                  fontSize: 13,
                                  color: _lastIsError
                                      ? cs.onErrorContainer
                                      : Colors.green.shade800)),
                        ),
                      ],
                    ),
                  ),
                const SizedBox(height: 8),
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: <Widget>[
                    Text('已扫：$validCount 台',
                        style: const TextStyle(
                            fontWeight: FontWeight.bold, fontSize: 16)),
                    Row(
                      children: <Widget>[
                        if (_entries.any((e) => !e.isError))
                          TextButton.icon(
                            onPressed: _undoLast,
                            icon: const Icon(Icons.undo, size: 16),
                            label: const Text('撤销'),
                            style: TextButton.styleFrom(
                                foregroundColor: cs.error),
                          ),
                        if (_scanning)
                          OutlinedButton(
                            onPressed: _stopScanning,
                            child: const Text('暂停'),
                          )
                        else
                          OutlinedButton(
                            onPressed: _startScanning,
                            child: const Text('扫码'),
                          ),
                      ],
                    ),
                  ],
                ),
              ],
            ),
          ),
          // 出库列表
          if (_entries.isNotEmpty) ...<Widget>[
            const SizedBox(height: 12),
            SectionCard(
              title: '出库明细',
              child: Column(
                children: _entries
                    .take(30)
                    .map((e) => _buildEntryTile(e, cs))
                    .toList(),
              ),
            ),
          ],
          const SizedBox(height: 100),
        ],
      ),
    );
  }

  Widget _buildEntryTile(_OutboundEntry e, ColorScheme cs) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: Row(
        children: <Widget>[
          Icon(
            e.isError ? Icons.cancel_outlined : Icons.check_circle_outline,
            size: 18,
            color: e.isError ? cs.error : Colors.green,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(e.sn,
                    style: TextStyle(
                        fontSize: 13,
                        fontFamily: 'monospace',
                        color: e.isError ? cs.error : null,
                        decoration: e.isError
                            ? TextDecoration.lineThrough
                            : null)),
                if (e.serial != null)
                  Text('product_id: ${e.serial!.productId}',
                      style: TextStyle(
                          fontSize: 11, color: cs.onSurfaceVariant)),
              ],
            ),
          ),
          if (e.isError && e.errorMsg != null)
            Text(e.errorMsg!,
                style: TextStyle(fontSize: 11, color: cs.error)),
        ],
      ),
    );
  }

  Widget _buildDoneScreen(ColorScheme cs) {
    return Scaffold(
      appBar: AppBar(title: const Text('序列号出库')),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: <Widget>[
              const Icon(Icons.check_circle_outline,
                  size: 80, color: Colors.green),
              const SizedBox(height: 16),
              Text(_submitResult ?? '出库完成',
                  textAlign: TextAlign.center,
                  style: const TextStyle(fontSize: 16)),
              const SizedBox(height: 24),
              FilledButton(
                  onPressed: () => Navigator.pop(context),
                  child: const Text('返回')),
              const SizedBox(height: 8),
              OutlinedButton(
                  onPressed: () => setState(() {
                        _done = false;
                        _entries.clear();
                        _submitResult = null;
                        _lastMsg = null;
                      }),
                  child: const Text('继续出库')),
            ],
          ),
        ),
      ),
    );
  }
}

// ── 辅助类 ────────────────────────────────────────────────────────────────────

class _OutboundEntry {
  const _OutboundEntry({
    required this.sn,
    this.serial,
    required this.isError,
    this.errorMsg,
  });
  final String sn;
  final SerialNumber? serial;
  final bool isError;
  final String? errorMsg;
}
