import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../../../core/widgets/brand_ui.dart';
import '../../products/application/product_controller.dart';
import '../../products/models/product_models.dart';
import '../models/serial_models.dart';
import '../models/serial_repository.dart';

class SerialInboundPage extends StatefulWidget {
  const SerialInboundPage({
    super.key,
    required this.repository,
    required this.productController,
  });
  final SerialRepository repository;
  final ProductController productController;

  @override
  State<SerialInboundPage> createState() => _SerialInboundPageState();
}

class _SerialInboundPageState extends State<SerialInboundPage> {
  // ── 步骤状态 ──────────────────────────────────────────────────────────────
  int _step = 0; // 0=选商品, 1=填价格, 2=扫码, 3=完成

  // ── 商品选择 ──────────────────────────────────────────────────────────────
  ProductData? _selectedProduct;

  // ── 进货价 ────────────────────────────────────────────────────────────────
  final _priceCtrl = TextEditingController();

  // ── 扫码会话 ──────────────────────────────────────────────────────────────
  final List<_ScanEntry> _entries = [];
  bool _scanning = false;
  MobileScannerController? _scanCtrl;
  String? _lastErrorMsg;

  // ── 提交状态 ──────────────────────────────────────────────────────────────
  bool _submitting = false;
  String? _submitResult;

  @override
  void initState() {
    super.initState();
    _loadSerialProductsFromController();
  }

  Future<void> _loadSerialProductsFromController() async {
    await widget.productController.loadProducts();
    if (mounted) {
      setState(() {
        _loadSerialProducts(widget.productController.list);
      });
    }
  }

  @override
  void dispose() {
    _priceCtrl.dispose();
    _scanCtrl?.dispose();
    super.dispose();
  }

  // ── 商品搜索（从外部传入列表，这里用搜索产品的 API） ────────────────────
  // 简化：直接在页面里输入 product_id 或由调用者传入 serialProducts 列表
  // 实际项目中可通过 SearchDelegate 或底部弹窗搜索
  List<ProductData> _serialProducts = [];

  void _loadSerialProducts(List<ProductData> all) {
    _serialProducts = all.where((p) => p.trackSerials).toList();
  }

  // ── 扫码逻辑 ──────────────────────────────────────────────────────────────

  void _startScanning() {
    _scanCtrl = MobileScannerController(detectionSpeed: DetectionSpeed.normal);
    setState(() {
      _scanning = true;
      _lastErrorMsg = null;
    });
  }

  void _stopScanning() {
    _scanCtrl?.stop();
    _scanCtrl?.dispose();
    _scanCtrl = null;
    setState(() => _scanning = false);
  }

  void _onDetect(BarcodeCapture capture) {
    final code = capture.barcodes.firstOrNull?.rawValue ?? '';
    if (code.isEmpty) return;
    _addSn(code);
  }

  void _addSn(String sn) {
    final trimmed = sn.trim();
    if (trimmed.isEmpty) return;

    // 检查重复
    final dup = _entries.any((e) => e.sn == trimmed && e.isSuccess);
    if (dup) {
      HapticFeedback.mediumImpact();
      setState(() {
        _lastErrorMsg = '序列号「$trimmed」已扫过，跳过';
        _entries.insert(0, _ScanEntry(sn: trimmed, isError: true, errorMsg: '重复'));
      });
      return;
    }

    HapticFeedback.lightImpact();
    setState(() {
      _lastErrorMsg = null;
      _entries.insert(0, _ScanEntry(sn: trimmed, isError: false));
    });
  }

  void _undoLast() {
    final idx = _entries.indexWhere((e) => e.isSuccess);
    if (idx < 0) return;
    setState(() => _entries.removeAt(idx));
  }

  // ── 提交 ──────────────────────────────────────────────────────────────────

  Future<void> _submit() async {
    final validSns = _entries.where((e) => e.isSuccess).map((e) => e.sn).toList();
    if (validSns.isEmpty || _selectedProduct == null) return;

    setState(() {
      _submitting = true;
      _submitResult = null;
    });

    try {
      final result = await widget.repository.inbound(
        SerialInboundRequest(
          productId: _selectedProduct!.id,
          unitCost: _priceCtrl.text.trim().isEmpty ? null : _priceCtrl.text.trim(),
          sns: validSns,
        ),
      );
      setState(() {
        _submitResult = '✅ 入库成功！单号：${result.bizNo}，共 ${result.count} 台';
        _step = 3;
      });
    } catch (e) {
      final msg = e.toString().replaceFirst('Exception: ', '');
      setState(() => _submitResult = '❌ $msg');
    } finally {
      setState(() => _submitting = false);
    }
  }

  // ── Build ──────────────────────────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Scaffold(
      backgroundColor: cs.surface,
      appBar: AppBar(
        title: const Text('序列号入库'),
        leading: BackButton(
          onPressed: () {
            if (_scanning) _stopScanning();
            Navigator.pop(context);
          },
        ),
      ),
      floatingActionButton: _buildFab(cs),
      body: _step == 3
          ? _buildDoneView(cs)
          : ListView(
              padding: const EdgeInsets.all(16),
              children: <Widget>[
                _buildStepIndicator(cs),
                const SizedBox(height: 16),
                if (_step == 0) _buildSelectProductStep(cs),
                if (_step == 1) _buildPriceStep(cs),
                if (_step == 2) _buildScanStep(cs),
              ],
            ),
    );
  }

  Widget _buildFab(ColorScheme cs) {
    final validCount = _entries.where((e) => e.isSuccess).length;
    if (_step != 2 || validCount == 0) return const SizedBox.shrink();
    return FloatingActionButton.extended(
      onPressed: _submitting ? null : _submit,
      icon: _submitting
          ? const SizedBox(
              width: 20,
              height: 20,
              child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white))
          : const Icon(Icons.upload_rounded),
      label: Text(_submitting ? '提交中...' : '提交入库（$validCount 台）'),
    );
  }

  // ── Step indicator ────────────────────────────────────────────────────────

  Widget _buildStepIndicator(ColorScheme cs) {
    const labels = ['选商品', '填价格', '扫码入库'];
    return Row(
      children: List.generate(labels.length, (i) {
        final active = i == _step;
        final done = i < _step;
        return Expanded(
          child: Row(
            children: <Widget>[
              CircleAvatar(
                radius: 14,
                backgroundColor: done
                    ? Colors.green
                    : active
                        ? cs.primary
                        : cs.surfaceContainerHighest,
                child: done
                    ? const Icon(Icons.check, size: 16, color: Colors.white)
                    : Text('${i + 1}',
                        style: TextStyle(
                          fontSize: 12,
                          color: active ? Colors.white : cs.onSurfaceVariant,
                        )),
              ),
              const SizedBox(width: 4),
              Expanded(
                child: Text(labels[i],
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight:
                          active ? FontWeight.bold : FontWeight.normal,
                      color: active ? cs.primary : cs.onSurfaceVariant,
                    )),
              ),
              if (i < labels.length - 1)
                Icon(Icons.chevron_right, size: 16, color: cs.outlineVariant),
            ],
          ),
        );
      }),
    );
  }

  // ── Step 0: 选商品 ────────────────────────────────────────────────────────

  Widget _buildSelectProductStep(ColorScheme cs) {
    return SectionCard(
      title: '选择商品（仅显示已开启序列号追踪的商品）',
      child: Column(
        children: <Widget>[
          if (_serialProducts.isEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 16),
              child: Text(
                '暂无已开启「每件独立 SN 码」的商品。\n请先在商品编辑页开启此选项。',
                style: TextStyle(color: cs.onSurfaceVariant),
                textAlign: TextAlign.center,
              ),
            )
          else
            ..._serialProducts.map((p) => ListTile(
                  contentPadding: EdgeInsets.zero,
                  leading: CircleAvatar(
                    backgroundColor: cs.primaryContainer,
                    child: Text(p.name[0],
                        style: TextStyle(color: cs.onPrimaryContainer)),
                  ),
                  title: Text(p.name,
                      style: const TextStyle(fontWeight: FontWeight.w600)),
                  subtitle: Text('库存 ${p.currentStock}${p.unit}  |  条码 ${p.barcode}',
                      style: const TextStyle(fontSize: 12)),
                  trailing: _selectedProduct?.id == p.id
                      ? Icon(Icons.check_circle, color: cs.primary)
                      : null,
                  onTap: () => setState(() => _selectedProduct = p),
                )),
          const SizedBox(height: 12),
          SizedBox(
            width: double.infinity,
            child: FilledButton(
              onPressed: _selectedProduct == null
                  ? null
                  : () => setState(() => _step = 1),
              child: Text(_selectedProduct == null
                  ? '请先选择商品'
                  : '下一步：填写进货价'),
            ),
          ),
        ],
      ),
    );
  }

  // ── Step 1: 填价格 ────────────────────────────────────────────────────────

  Widget _buildPriceStep(ColorScheme cs) {
    return SectionCard(
      title: '进货价格（留空则使用系统默认值）',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          _ProductInfoChip(product: _selectedProduct!, cs: cs),
          const SizedBox(height: 12),
          TextField(
            controller: _priceCtrl,
            keyboardType:
                const TextInputType.numberWithOptions(decimal: true),
            decoration: const InputDecoration(
              labelText: '统一进货价（可选）',
              hintText: '例如：5800.00，留空使用商品历史进价',
              border: OutlineInputBorder(),
              isDense: true,
              prefixText: '¥ ',
            ),
          ),
          const SizedBox(height: 16),
          Row(
            children: <Widget>[
              OutlinedButton(
                onPressed: () => setState(() => _step = 0),
                child: const Text('上一步'),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: FilledButton(
                  onPressed: () {
                    _startScanning();
                    setState(() => _step = 2);
                  },
                  child: const Text('开始扫码入库'),
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  // ── Step 2: 扫码 ──────────────────────────────────────────────────────────

  Widget _buildScanStep(ColorScheme cs) {
    final validCount = _entries.where((e) => e.isSuccess).length;

    return Column(
      children: <Widget>[
        SectionCard(
          title: '商品：${_selectedProduct!.name}',
          child: Column(
            children: <Widget>[
              // 扫码取景框
              if (_scanning && _scanCtrl != null)
                ClipRRect(
                  borderRadius: BorderRadius.circular(12),
                  child: SizedBox(
                    height: 200,
                    child: MobileScanner(
                      controller: _scanCtrl!,
                      onDetect: _onDetect,
                    ),
                  ),
                )
              else
                GestureDetector(
                  onTap: _startScanning,
                  child: Container(
                    height: 200,
                    decoration: BoxDecoration(
                      color: cs.surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Center(
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: <Widget>[
                          Icon(Icons.qr_code_scanner,
                              size: 48, color: cs.primary),
                          const SizedBox(height: 8),
                          Text('点击开始扫码',
                              style: TextStyle(color: cs.onSurfaceVariant)),
                        ],
                      ),
                    ),
                  ),
                ),
              const SizedBox(height: 8),
              // 状态栏
              if (_lastErrorMsg != null)
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: cs.errorContainer,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Row(
                    children: <Widget>[
                      Icon(Icons.warning_amber_outlined,
                          size: 16, color: cs.error),
                      const SizedBox(width: 6),
                      Expanded(
                          child: Text(_lastErrorMsg!,
                              style: TextStyle(
                                  fontSize: 13, color: cs.onErrorContainer))),
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
                      if (_entries.any((e) => e.isSuccess))
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
                          child: const Text('暂停扫码'),
                        )
                      else
                        OutlinedButton(
                          onPressed: _startScanning,
                          child: const Text('继续扫码'),
                        ),
                    ],
                  ),
                ],
              ),
            ],
          ),
        ),
        // SN 列表
        if (_entries.isNotEmpty) ...<Widget>[
          const SizedBox(height: 8),
          SectionCard(
            title: '序列号列表',
            child: Column(
              children: _entries
                  .take(20)
                  .map((e) => _SnEntryTile(entry: e, cs: cs))
                  .toList(),
            ),
          ),
        ],
        const SizedBox(height: 80),
      ],
    );
  }

  // ── Step 3: 完成 ──────────────────────────────────────────────────────────

  Widget _buildDoneView(ColorScheme cs) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            const Icon(Icons.check_circle_outline,
                size: 80, color: Colors.green),
            const SizedBox(height: 16),
            Text(_submitResult ?? '入库完成',
                textAlign: TextAlign.center,
                style: const TextStyle(fontSize: 16)),
            const SizedBox(height: 24),
            FilledButton(
                onPressed: () => Navigator.pop(context),
                child: const Text('返回')),
            const SizedBox(height: 8),
            OutlinedButton(
                onPressed: () => setState(() {
                      _step = 0;
                      _selectedProduct = null;
                      _entries.clear();
                      _priceCtrl.clear();
                      _submitResult = null;
                    }),
                child: const Text('继续入库')),
          ],
        ),
      ),
    );
  }
}

// ── 辅助组件 ──────────────────────────────────────────────────────────────────

class _ScanEntry {
  _ScanEntry({required this.sn, required this.isError, this.errorMsg})
      : scannedAt = DateTime.now();
  final String sn;
  final bool isError;
  final String? errorMsg;
  final DateTime scannedAt;
  bool get isSuccess => !isError;
}

class _ProductInfoChip extends StatelessWidget {
  const _ProductInfoChip({required this.product, required this.cs});
  final ProductData product;
  final ColorScheme cs;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: cs.primaryContainer.withOpacity(.4),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: <Widget>[
          Icon(Icons.inventory_2_outlined, color: cs.primary, size: 20),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(product.name,
                    style: const TextStyle(fontWeight: FontWeight.bold)),
                Text('库存 ${product.currentStock}${product.unit} | 条码 ${product.barcode}',
                    style: TextStyle(fontSize: 12, color: cs.onSurfaceVariant)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _SnEntryTile extends StatelessWidget {
  const _SnEntryTile({required this.entry, required this.cs});
  final _ScanEntry entry;
  final ColorScheme cs;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: <Widget>[
          Icon(
            entry.isError ? Icons.cancel_outlined : Icons.check_circle_outline,
            size: 18,
            color: entry.isError ? cs.error : Colors.green,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              entry.sn,
              style: TextStyle(
                fontSize: 13,
                fontFamily: 'monospace',
                color: entry.isError ? cs.error : null,
                decoration: entry.isError ? TextDecoration.lineThrough : null,
              ),
            ),
          ),
          if (entry.isError && entry.errorMsg != null)
            Text(entry.errorMsg!,
                style: TextStyle(fontSize: 11, color: cs.error)),
        ],
      ),
    );
  }
}
