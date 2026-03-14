import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../core/widgets/barcode_scanner_sheet.dart';
import '../../../core/widgets/brand_ui.dart';
import '../../../storage/session_storage.dart';
import '../application/outbound_controller.dart';
import '../models/inventory_models.dart';

enum _OutboundScanMode { scanConfirm, continuousScan }

class OutboundPage extends StatefulWidget {
  const OutboundPage({
    super.key,
    required this.controller,
    required this.sessionStorage,
    required this.scanPreferenceScope,
    this.onViewHistory,
  });

  final OutboundController controller;
  final SessionStorage sessionStorage;
  final String scanPreferenceScope;
  final VoidCallback? onViewHistory;

  @override
  State<OutboundPage> createState() => _OutboundPageState();
}

class _OutboundPageState extends State<OutboundPage> {
  static final RegExp _pricePattern = RegExp(r'^\d+(\.\d{1,4})?$');

  final FocusNode _scanBarcodeFocusNode = FocusNode();
  final TextEditingController _scanBarcodeController = TextEditingController();
  final TextEditingController _scanSellPriceController =
      TextEditingController();

  final TextEditingController _scanConfirmQtyController =
      TextEditingController(text: '1');
  final TextEditingController _scanConfirmSellPriceController =
      TextEditingController();

  final TextEditingController _customerIdController = TextEditingController();
  final TextEditingController _expectedVersionController =
      TextEditingController();
  final TextEditingController _remarkController = TextEditingController();

  int _itemSeed = 1;
  final List<_OutboundItemEditors> _itemEditors = <_OutboundItemEditors>[];

  _OutboundScanMode _scanMode = _OutboundScanMode.scanConfirm;
  String _scanConfirmProductId = '';
  String _scanConfirmProductName = '';
  bool _scanConfirmSessionActive = false;
  int _scanConfirmProcessedCount = 0;
  bool _continuousSessionActive = false;
  int _continuousProcessedCount = 0;
  bool _orderHeaderExpanded = false;
  bool _orderAdvancedExpanded = false;

  @override
  void initState() {
    super.initState();
    _restoreStoredScanMode();
  }

  Future<void> _restoreStoredScanMode() async {
    final storedMode = await widget.sessionStorage.readScanMode(
      scope: widget.scanPreferenceScope,
      page: 'outbound',
    );
    final normalizedStoredMode = storedMode?.trim();
    final parsedMode = _modeFromStorage(normalizedStoredMode);
    if (!mounted || parsedMode == null) {
      return;
    }

    setState(() {
      _scanMode = parsedMode;
    });

    if (normalizedStoredMode != _modeToStorage(parsedMode)) {
      unawaited(_persistScanMode(parsedMode));
    }
  }

  Future<void> _persistScanMode(_OutboundScanMode mode) {
    return widget.sessionStorage.writeScanMode(
      scope: widget.scanPreferenceScope,
      page: 'outbound',
      mode: _modeToStorage(mode),
    );
  }

  String _modeToStorage(_OutboundScanMode mode) {
    switch (mode) {
      case _OutboundScanMode.scanConfirm:
        return 'scan_confirm';
      case _OutboundScanMode.continuousScan:
        return 'continuous_scan';
    }
  }

  _OutboundScanMode? _modeFromStorage(String? value) {
    switch (value?.trim()) {
      case 'quick_accumulate':
        return _OutboundScanMode.continuousScan;
      case 'scan_confirm':
        return _OutboundScanMode.scanConfirm;
      case 'continuous_scan':
        return _OutboundScanMode.continuousScan;
      default:
        return null;
    }
  }

  @override
  void dispose() {
    _scanBarcodeFocusNode.dispose();
    _scanBarcodeController.dispose();
    _scanSellPriceController.dispose();
    _scanConfirmQtyController.dispose();
    _scanConfirmSellPriceController.dispose();
    _customerIdController.dispose();
    _expectedVersionController.dispose();
    _remarkController.dispose();

    for (final item in _itemEditors) {
      item.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.controller,
      builder: (BuildContext context, Widget? child) {
        return Scaffold(
          appBar: AppBar(
            title: const Text('销售出库'),
            actions: <Widget>[
              if (widget.onViewHistory != null)
                TextButton.icon(
                  onPressed: widget.onViewHistory,
                  icon: const Icon(Icons.history_rounded, size: 18),
                  label: const Text('出库记录'),
                ),
              const SizedBox(width: 4),
            ],
          ),
          body: ListView(
            padding: const EdgeInsets.all(16),
            children: <Widget>[
              const BrandHeroBanner(
                title: '销售出库作业',
                subtitle: '支持扫码确认、连续扫码会话',
                icon: Icons.local_shipping_rounded,
                gradientSeedColor: Color(0xFFF59E0B),
              ),
              const SizedBox(height: 12),
              if (!widget.controller.canSubmit)
                const StatusNotice(
                  message: '当前角色无销售出库权限，仅 OWNER/SALES 可操作。',
                  tone: NoticeTone.warning,
                ),
              _buildScanCard(context),
              const SizedBox(height: 12),
              _buildFormCard(context),
              if (widget.controller.result != null) ...<Widget>[
                const SizedBox(height: 12),
                _ResultCard(result: widget.controller.result!),
              ],
            ],
          ),
        );
      },
    );
  }

  Widget _buildScanCard(BuildContext context) {
    return SectionCard(
      title: '条码出库',
      subtitle: '支持确认写入与连续扫码会话',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            '扫码模式',
            style: Theme.of(context).textTheme.labelLarge,
          ),
          const SizedBox(height: 6),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
              ChoiceChip(
                label: const Text('确认写入（默认）'),
                selected: _scanMode == _OutboundScanMode.scanConfirm,
                onSelected:
                    widget.controller.scanning || widget.controller.submitting
                        ? null
                        : (bool selected) {
                            if (selected) {
                              _onScanModeChanged(_OutboundScanMode.scanConfirm);
                            }
                          },
              ),
              ChoiceChip(
                label: const Text('连续扫码会话'),
                selected: _scanMode == _OutboundScanMode.continuousScan,
                onSelected: widget.controller.scanning ||
                        widget.controller.submitting
                    ? null
                    : (bool selected) {
                        if (selected) {
                          _onScanModeChanged(_OutboundScanMode.continuousScan);
                        }
                      },
              ),
            ],
          ),
          const SizedBox(height: 8),
          TextField(
            controller: _scanBarcodeController,
            focusNode: _scanBarcodeFocusNode,
            onSubmitted: _onScanBarcodeSubmitted,
            decoration: InputDecoration(
              labelText: '条码',
              hintText: '扫码枪回车，例如：690123456789',
              border: const OutlineInputBorder(),
              isDense: true,
              suffixIcon: IconButton(
                tooltip: '摄像头扫码',
                onPressed: widget.controller.scanning ||
                        widget.controller.submitting ||
                        !widget.controller.canSubmit
                    ? null
                    : _scanWithCamera,
                icon: const Icon(Icons.qr_code_scanner_rounded),
              ),
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: _scanSellPriceController,
            decoration: const InputDecoration(
              labelText: '销售单价（可选）',
              hintText: '留空使用商品零售价',
              border: OutlineInputBorder(),
              isDense: true,
            ),
            keyboardType: const TextInputType.numberWithOptions(decimal: true),
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
              FilledButton(
                onPressed: widget.controller.scanning ||
                        widget.controller.submitting ||
                        !widget.controller.canSubmit
                    ? null
                    : _onPrimaryScanPressed,
                child: Text(
                  _scanMode == _OutboundScanMode.continuousScan
                      ? (_continuousSessionActive ? '会话扫码中...' : '开始连续扫码')
                      : (widget.controller.scanning ? '识别中...' : '按条码处理'),
                ),
              ),
              OutlinedButton(
                onPressed:
                    widget.controller.scanning || widget.controller.submitting
                        ? null
                        : _resetScan,
                child: const Text('清空'),
              ),
              if (_scanMode == _OutboundScanMode.scanConfirm &&
                  _scanConfirmSessionActive)
                OutlinedButton(
                  onPressed:
                      widget.controller.scanning || widget.controller.submitting
                          ? null
                          : _stopScanConfirmSession,
                  child: const Text('结束确认续扫'),
                ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            _scanMode == _OutboundScanMode.continuousScan
                ? '连续扫码会话：${_continuousSessionActive ? '进行中' : '未开始'}，已处理 $_continuousProcessedCount 条。'
                : '确认续扫会话：${_scanConfirmSessionActive ? '进行中' : '未开始'}，已处理 $_scanConfirmProcessedCount 条。',
          ),
          if (widget.controller.scanErrorMessage != null) ...<Widget>[
            const SizedBox(height: 8),
            StatusNotice(
              message: widget.controller.scanErrorMessage!,
              tone: NoticeTone.error,
            ),
          ],
          if (widget.controller.scanSuccessMessage != null) ...<Widget>[
            const SizedBox(height: 8),
            StatusNotice(
              message: widget.controller.scanSuccessMessage!,
              tone: NoticeTone.success,
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildFormCard(BuildContext context) {
    return SectionCard(
      title: '出库表单',
      subtitle: '主路径：扫码/手工明细 -> 明细核对 -> 提交出库',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          SectionCard(
            title: '单据头（可选）',
            subtitle: _orderHeaderExpanded ? '可补充客户、备注与高级字段' : '默认折叠。需要时再展开。',
            action: TextButton(
              onPressed: widget.controller.submitting
                  ? null
                  : () {
                      setState(() {
                        _orderHeaderExpanded = !_orderHeaderExpanded;
                      });
                    },
              child: Text(_orderHeaderExpanded ? '收起' : '展开'),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                if (_orderHeaderExpanded) ...<Widget>[
                  _buildTextField(
                    controller: _customerIdController,
                    label: '客户ID',
                    hint: '可选，正整数',
                    keyboardType: TextInputType.number,
                  ),
                  const SizedBox(height: 8),
                  _buildTextField(
                    controller: _remarkController,
                    label: '备注',
                    hint: '可选备注',
                  ),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: <Widget>[
                      OutlinedButton(
                        onPressed: widget.controller.submitting
                            ? null
                            : () {
                                setState(() {
                                  _orderAdvancedExpanded =
                                      !_orderAdvancedExpanded;
                                });
                              },
                        child: Text(
                          _orderAdvancedExpanded ? '收起高级字段' : '展开高级字段',
                        ),
                      ),
                    ],
                  ),
                  if (_orderAdvancedExpanded) ...<Widget>[
                    const SizedBox(height: 8),
                    _buildTextField(
                      controller: _expectedVersionController,
                      label: '单据版本（高级）',
                      hint: '可选 expected_version',
                      keyboardType: TextInputType.number,
                    ),
                  ],
                ],
              ],
            ),
          ),
          const SizedBox(height: 12),
          Row(
            children: <Widget>[
              const Text('出库明细', style: TextStyle(fontWeight: FontWeight.w600)),
              const Spacer(),
              OutlinedButton(
                onPressed: widget.controller.submitting ? null : _addItem,
                child: const Text('新增明细'),
              ),
            ],
          ),
          const SizedBox(height: 8),
          if (_itemEditors.isEmpty)
            const StatusNotice(
              message: '暂无明细，请先扫码或点击“新增明细”。',
              tone: NoticeTone.info,
            ),
          ..._itemEditors.map(_buildItemCard),
          const SizedBox(height: 12),
          // ── 操作消息（放在按钮上方，避免遮挡提交按钮）───────
          if (widget.controller.errorMessage != null)
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: StatusNotice(
                message: widget.controller.errorMessage!,
                tone: NoticeTone.error,
              ),
            ),
          if (widget.controller.successMessage != null)
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: StatusNotice(
                message: widget.controller.successMessage!,
                tone: NoticeTone.success,
              ),
            ),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
              FilledButton(
                onPressed:
                    widget.controller.submitting || !widget.controller.canSubmit
                        ? null
                        : _submit,
                child: Text(widget.controller.submitting ? '提交中...' : '提交出库'),
              ),
              OutlinedButton(
                onPressed: widget.controller.submitting ? null : _resetForm,
                child: const Text('重置'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildItemCard(_OutboundItemEditors item) {
    return SectionCard(
      title: '明细 #${item.localId}',
      action: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          TextButton(
            onPressed: widget.controller.submitting
                ? null
                : () => _toggleItemEditable(item.localId),
            child: Text(item.editable ? '完成' : '编辑'),
          ),
          TextButton(
            onPressed: widget.controller.submitting
                ? null
                : () => _removeItem(item.localId),
            child: const Text('删除'),
          ),
        ],
      ),
      child: Column(
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: _buildTextField(
                  controller: item.productIdController,
                  label: '商品ID *',
                  hint: '1001',
                  keyboardType: TextInputType.number,
                  readOnly: !item.editable,
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: _buildTextField(
                  controller: item.productNameController,
                  label: '商品名称',
                  hint: '扫码自动带出，可手动补充',
                  readOnly: !item.editable,
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Row(
            children: <Widget>[
              Expanded(
                child: _buildTextField(
                  controller: item.qtyController,
                  label: '数量 *',
                  hint: '1',
                  keyboardType: TextInputType.number,
                  readOnly: !item.editable,
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: _buildTextField(
                  controller: item.sellPriceController,
                  label: '销售单价 *',
                  hint: '3.50',
                  keyboardType:
                      const TextInputType.numberWithOptions(decimal: true),
                  readOnly: !item.editable,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildTextField({
    required TextEditingController controller,
    required String label,
    required String hint,
    TextInputType? keyboardType,
    bool readOnly = false,
  }) {
    return TextField(
      controller: controller,
      keyboardType: keyboardType,
      readOnly: readOnly,
      enabled: !widget.controller.submitting,
      decoration: InputDecoration(
        labelText: label,
        hintText: hint,
        border: const OutlineInputBorder(),
        isDense: true,
      ),
    );
  }

  void _onScanModeChanged(_OutboundScanMode value) {
    setState(() {
      _scanMode = value;
      if (value != _OutboundScanMode.scanConfirm) {
        _scanConfirmSessionActive = false;
        _scanConfirmProcessedCount = 0;
      }
      if (value != _OutboundScanMode.continuousScan) {
        _continuousSessionActive = false;
      }
    });
    unawaited(_persistScanMode(value));
  }

  Future<void> _onPrimaryScanPressed() async {
    if (_scanMode != _OutboundScanMode.continuousScan &&
        _scanBarcodeController.text.trim().isEmpty) {
      await _scanWithCamera();
      return;
    }

    await _scanByCurrentMode();
  }

  Future<void> _scanByCurrentMode() async {
    switch (_scanMode) {
      case _OutboundScanMode.scanConfirm:
        await _scanForConfirm();
        return;
      case _OutboundScanMode.continuousScan:
        await _startContinuousScanSession();
        return;
    }
  }

  void _onScanBarcodeSubmitted(String _) {
    if (_scanMode == _OutboundScanMode.continuousScan ||
        widget.controller.scanning ||
        widget.controller.submitting ||
        !widget.controller.canSubmit) {
      return;
    }

    unawaited(_scanByCurrentMode());
  }

  Future<void> _scanForConfirm() async {
    FocusScope.of(context).unfocus();
    _compactBlankRows();

    final preview = await widget.controller.scanForConfirm(
      barcode: _scanBarcodeController.text.trim(),
      customSellPrice: _scanSellPriceController.text,
    );
    if (preview == null) {
      return;
    }
    if (!mounted) {
      return;
    }

    _scanConfirmProductId = preview.productId;
    _scanConfirmProductName = preview.productName;
    _scanConfirmQtyController.text = '1';
    _scanConfirmSellPriceController.text = preview.suggestedSellPrice;
    _scanConfirmSessionActive = true;

    final bool? confirmed = await showDialog<bool>(
      context: context,
      builder: (BuildContext context) {
        return AlertDialog(
          title: const Text('确认写入'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text('已命中商品：#$_scanConfirmProductId $_scanConfirmProductName'),
              const SizedBox(height: 8),
              TextField(
                controller: _scanConfirmQtyController,
                keyboardType: TextInputType.number,
                autofocus: true,
                textInputAction: TextInputAction.next,
                decoration: const InputDecoration(
                  labelText: '数量 *',
                  hintText: '1',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
              ),
              const SizedBox(height: 8),
              TextField(
                controller: _scanConfirmSellPriceController,
                keyboardType:
                    const TextInputType.numberWithOptions(decimal: true),
                textInputAction: TextInputAction.done,
                onSubmitted: (_) {
                  final ok = _applyScanConfirm();
                  if (ok) {
                    Navigator.of(context).pop(true);
                  }
                },
                decoration: const InputDecoration(
                  labelText: '销售单价 *',
                  hintText: '3.50',
                  border: OutlineInputBorder(),
                  isDense: true,
                ),
              ),
            ],
          ),
          actions: <Widget>[
            TextButton(
              onPressed: widget.controller.submitting
                  ? null
                  : () => Navigator.of(context).pop(false),
              child: const Text('取消'),
            ),
            FilledButton(
              onPressed: widget.controller.submitting
                  ? null
                  : () {
                      final ok = _applyScanConfirm();
                      if (ok) {
                        Navigator.of(context).pop(true);
                      }
                    },
              child: const Text('确认写入（回车）'),
            ),
          ],
        );
      },
    );

    if (confirmed != true && mounted) {
      _scanBarcodeController.clear();
      _scanBarcodeFocusNode.requestFocus();
      _showSnack('已取消本次确认，可继续扫码');
    }
  }

  Future<void> _scanWithCamera() async {
    if (widget.controller.scanning ||
        widget.controller.submitting ||
        !widget.controller.canSubmit) {
      return;
    }

    FocusScope.of(context).unfocus();

    if (_scanMode == _OutboundScanMode.continuousScan) {
      await _startContinuousScanSession();
      return;
    }

    final String? barcode = await BarcodeScannerSheet.scan(
      context,
      title: '出库扫码',
      hint: '将条码对准取景框，识别成功后会自动回填并按当前模式处理。',
    );
    if (!mounted || barcode == null || barcode.isEmpty) {
      return;
    }

    _scanBarcodeController.text = barcode;
    await _scanByCurrentMode();
  }

  Future<void> _startContinuousScanSession() async {
    if (_continuousSessionActive ||
        widget.controller.submitting ||
        !widget.controller.canSubmit) {
      return;
    }

    FocusScope.of(context).unfocus();
    _compactBlankRows();
    setState(() {
      _continuousSessionActive = true;
      _continuousProcessedCount = 0;
    });

    await BarcodeScannerSheet.scanContinuous(
      context,
      title: '出库连续扫码',
      hint: '摄像头持续开启，识别到条码后自动累加，完成后点击“结束扫码”。',
      onScanned: (String barcode) async {
        if (!mounted) {
          return false;
        }
        _scanBarcodeController.text = barcode;
        final result = await widget.controller.scanAndAccumulate(
          barcode: barcode,
          customSellPrice: _scanSellPriceController.text,
          items: _collectItemInputs(),
        );
        if (!mounted || result == null) {
          return false;
        }

        setState(() {
          _continuousProcessedCount += 1;
        });
        _replaceItems(result.items);
        SystemSound.play(SystemSoundType.click);
        return true;
      },
    );

    if (!mounted) {
      return;
    }

    setState(() {
      _continuousSessionActive = false;
    });
    _showSnack('连续扫码会话结束，本次共处理 $_continuousProcessedCount 条');
  }

  bool _applyScanConfirm() {
    final qtyRaw = _scanConfirmQtyController.text.trim();
    final qty = int.tryParse(qtyRaw);
    if (qty == null || qty <= 0) {
      _showSnack('确认区“数量”必须为正整数');
      return false;
    }

    final sellPriceRaw = _scanConfirmSellPriceController.text.trim();
    if (!_pricePattern.hasMatch(sellPriceRaw)) {
      _showSnack('确认区“销售单价”格式错误（示例：3.50）');
      return false;
    }

    _mergeConfirmedItem(
      OutboundFormItemInput(
        productId: _scanConfirmProductId,
        productName: _scanConfirmProductName,
        qty: qty.toString(),
        sellPrice: sellPriceRaw,
        expectedVersion: '',
      ),
      productName: _scanConfirmProductName,
    );

    setState(() {
      if (_scanMode == _OutboundScanMode.scanConfirm) {
        _scanConfirmSessionActive = true;
        _scanConfirmProcessedCount += 1;
      }
    });

    _scanBarcodeController.clear();
    if (_scanMode == _OutboundScanMode.scanConfirm) {
      _showSnack('确认写入成功，确认续扫会话进行中（已处理 $_scanConfirmProcessedCount 条）');
      _scanBarcodeFocusNode.requestFocus();
    }

    return true;
  }

  void _stopScanConfirmSession() {
    setState(() {
      _scanConfirmSessionActive = false;
    });
    _showSnack('确认续扫会话已结束，本次共处理 $_scanConfirmProcessedCount 条');
  }

  void _resetScan() {
    _scanBarcodeController.clear();
    _scanSellPriceController.clear();
    _scanConfirmQtyController.text = '1';
    _scanConfirmSellPriceController.clear();
    setState(() {
      _scanConfirmProductId = '';
      _scanConfirmProductName = '';
      _scanConfirmSessionActive = false;
      _scanConfirmProcessedCount = 0;
      _continuousSessionActive = false;
      _continuousProcessedCount = 0;
    });
    widget.controller.clearScanMessages();
  }

  Future<void> _submit() async {
    FocusScope.of(context).unfocus();
    _compactBlankRows();

    final ok = await widget.controller.submit(
      customerId: _customerIdController.text,
      expectedVersion: _expectedVersionController.text,
      remark: _remarkController.text,
      items: _collectItemInputs(),
    );
    if (!ok) {
      return;
    }

    _customerIdController.clear();
    _expectedVersionController.clear();
    _remarkController.clear();
    _replaceItems(const <OutboundFormItemInput>[]);
  }

  void _resetForm() {
    _customerIdController.clear();
    _expectedVersionController.clear();
    _remarkController.clear();
    _resetScan();

    _replaceItems(const <OutboundFormItemInput>[]);
    widget.controller.clearMessages();
    widget.controller.clearResult();
  }

  void _addItem() {
    setState(() {
      _itemEditors.add(_createItemEditors());
    });
  }

  void _toggleItemEditable(int localId) {
    setState(() {
      final index = _itemEditors.indexWhere((item) => item.localId == localId);
      if (index < 0) {
        return;
      }
      _itemEditors[index].editable = !_itemEditors[index].editable;
    });
  }

  void _removeItem(int localId) {
    setState(() {
      final index = _itemEditors.indexWhere((item) => item.localId == localId);
      if (index < 0) {
        return;
      }

      _itemEditors[index].dispose();
      _itemEditors.removeAt(index);
    });
  }

  void _compactBlankRows() {
    final toRemove = _itemEditors.where(_isBlankEditor).toList();
    if (toRemove.isEmpty) {
      return;
    }

    setState(() {
      for (final item in toRemove) {
        item.dispose();
        _itemEditors.remove(item);
      }
    });
  }

  bool _isBlankEditor(_OutboundItemEditors item) {
    return item.productIdController.text.trim().isEmpty &&
        item.productNameController.text.trim().isEmpty &&
        item.qtyController.text.trim().isEmpty &&
        item.sellPriceController.text.trim().isEmpty;
  }

  void _mergeConfirmedItem(
    OutboundFormItemInput input, {
    required String productName,
  }) {
    _compactBlankRows();

    final index = _itemEditors.indexWhere(
      (item) => item.productIdController.text.trim() == input.productId.trim(),
    );

    setState(() {
      if (index >= 0) {
        final existed = _itemEditors[index];
        final existedQty = int.tryParse(existed.qtyController.text.trim()) ?? 0;
        final appendQty = int.tryParse(input.qty.trim()) ?? 0;
        final nextQty = existedQty + appendQty;
        existed.qtyController.text = nextQty.toString();
        if (existed.productNameController.text.trim().isEmpty) {
          existed.productNameController.text = input.productName;
        }
        if (existed.sellPriceController.text.trim().isEmpty) {
          existed.sellPriceController.text = input.sellPrice;
        }
        existed.editable = false;
      } else {
        _itemEditors.add(_createItemEditorsFromInput(input));
      }
    });

    _showSnack('已写入明细：#${input.productId} $productName');
  }

  List<OutboundFormItemInput> _collectItemInputs() {
    return _itemEditors
        .map(
          (item) => OutboundFormItemInput(
            productId: item.productIdController.text,
            productName: item.productNameController.text,
            qty: item.qtyController.text,
            sellPrice: item.sellPriceController.text,
            expectedVersion: '',
          ),
        )
        .toList();
  }

  void _replaceItems(List<OutboundFormItemInput> inputs) {
    setState(() {
      for (final item in _itemEditors) {
        item.dispose();
      }
      _itemEditors
        ..clear()
        ..addAll(inputs.map(_createItemEditorsFromInput));
    });
  }

  _OutboundItemEditors _createItemEditors() {
    return _createItemEditorsFromInput(
      const OutboundFormItemInput(
        productId: '',
        productName: '',
        qty: '',
        sellPrice: '',
        expectedVersion: '',
      ),
    );
  }

  _OutboundItemEditors _createItemEditorsFromInput(
      OutboundFormItemInput input) {
    final localId = _itemSeed++;
    return _OutboundItemEditors(
      localId: localId,
      productIdController: TextEditingController(text: input.productId),
      productNameController: TextEditingController(text: input.productName),
      qtyController: TextEditingController(text: input.qty),
      sellPriceController: TextEditingController(text: input.sellPrice),
      editable: false,
    );
  }

  void _showSnack(String message) {
    final messenger = ScaffoldMessenger.of(context);
    messenger.clearSnackBars();
    messenger.showSnackBar(SnackBar(
      content: Text(message, style: const TextStyle(fontSize: 13)),
      behavior: SnackBarBehavior.floating,
      margin: const EdgeInsets.fromLTRB(16, 0, 16, 80),
      duration: const Duration(milliseconds: 1500),
      dismissDirection: DismissDirection.horizontal,
    ));
  }
}

class _OutboundItemEditors {
  _OutboundItemEditors({
    required this.localId,
    required this.productIdController,
    required this.productNameController,
    required this.qtyController,
    required this.sellPriceController,
    required this.editable,
  });

  final int localId;
  final TextEditingController productIdController;
  final TextEditingController productNameController;
  final TextEditingController qtyController;
  final TextEditingController sellPriceController;
  bool editable;

  void dispose() {
    productIdController.dispose();
    productNameController.dispose();
    qtyController.dispose();
    sellPriceController.dispose();
  }
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({required this.result});

  final OutboundResultData result;

  @override
  Widget build(BuildContext context) {
    return SectionCard(
      title: '最近一次出库结果',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: <Widget>[
              const Text('业务单号',
                  style: TextStyle(color: Colors.grey, fontSize: 13)),
              Text(result.bizNo,
                  style: const TextStyle(fontWeight: FontWeight.w700)),
            ],
          ),
          const SizedBox(height: 8),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: <Widget>[
              const Text('总金额',
                  style: TextStyle(color: Colors.grey, fontSize: 13)),
              Text(
                result.totalAmount,
                style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w900,
                    fontFamily: 'RobotoMono',
                    letterSpacing: -0.5),
              ),
            ],
          ),
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 8),
            child: Divider(height: 1),
          ),
          ...result.items.map(
            (item) => Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Text(
                '商品ID=${item.productId}，出库数量=${item.qty}，剩余库存=${item.currentStock}，版本=${item.version}',
                style: const TextStyle(fontSize: 12),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
