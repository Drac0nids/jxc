import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../core/widgets/barcode_scanner_sheet.dart';
import '../../../core/widgets/brand_ui.dart';
import '../../../storage/session_storage.dart';
import '../application/inbound_controller.dart';
import '../application/inbound_logs_controller.dart';
import '../models/inventory_models.dart';
import 'inbound_logs_page.dart';
import '../../products/application/batch_controller.dart';
import '../../products/application/product_controller.dart';
import '../../products/models/product_models.dart';
import '../../products/presentation/batch_link_sheet.dart';
import '../../products/presentation/create_product_sheet.dart';

enum _InboundScanMode { scanConfirm, continuousScan }

class _InboundDraftItem {
  const _InboundDraftItem({
    required this.localId,
    required this.productId,
    required this.barcode,
    required this.productName,
    required this.qty,
    required this.unitCost,
    required this.expectedVersion,
    required this.remark,
  });

  final int localId;
  final String productId;
  final String barcode;
  final String productName;
  final String qty;
  final String unitCost;
  final String expectedVersion;
  final String remark;

  bool matches({required String productId, required String barcode}) {
    final currentProductId = this.productId.trim();
    final currentBarcode = this.barcode.trim();
    final incomingProductId = productId.trim();
    final incomingBarcode = barcode.trim();
    if (currentProductId.isNotEmpty && incomingProductId.isNotEmpty) {
      return currentProductId == incomingProductId;
    }
    if (currentBarcode.isNotEmpty && incomingBarcode.isNotEmpty) {
      return currentBarcode == incomingBarcode;
    }
    return false;
  }
}

class InboundPage extends StatefulWidget {
  const InboundPage({
    super.key,
    required this.controller,
    required this.productController,
    required this.sessionStorage,
    required this.scanPreferenceScope,
    required this.logsController,
    this.batchController,
  });

  final InboundController controller;
  final ProductController productController;
  final SessionStorage sessionStorage;
  final String scanPreferenceScope;
  final InboundLogsController logsController;
  final BatchController? batchController;

  @override
  State<InboundPage> createState() => _InboundPageState();
}

class _InboundPageState extends State<InboundPage> {
  static final RegExp _moneyPattern = RegExp(r'^\d+(\.\d{1,4})?$');

  final FocusNode _scanBarcodeFocusNode = FocusNode();
  final TextEditingController _scanBarcodeController = TextEditingController();
  final TextEditingController _scanConfirmQtyController =
      TextEditingController(text: '1');

  final TextEditingController _productIdController = TextEditingController();
  final TextEditingController _barcodeController = TextEditingController();
  final TextEditingController _qtyController = TextEditingController(text: '1');
  final TextEditingController _unitCostController = TextEditingController();
  final TextEditingController _expectedVersionController =
      TextEditingController();
  final TextEditingController _remarkController = TextEditingController();

  _InboundScanMode _scanMode = _InboundScanMode.scanConfirm;
  String _scanConfirmProductId = '';
  String _scanConfirmProductName = '';
  String _scanConfirmBarcode = '';
  String _scanConfirmUnitCost = '';
  bool _scanConfirmSessionActive = false;
  int _scanConfirmProcessedCount = 0;
  bool _continuousSessionActive = false;
  int _continuousProcessedCount = 0;
  bool _manualFormExpanded = false;

  int _draftSeed = 1;
  final List<_InboundDraftItem> _draftItems = <_InboundDraftItem>[];

  @override
  void initState() {
    super.initState();
    _restoreStoredScanMode();
  }

  Future<void> _restoreStoredScanMode() async {
    final storedMode = await widget.sessionStorage.readScanMode(
      scope: widget.scanPreferenceScope,
      page: 'inbound',
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

  Future<void> _persistScanMode(_InboundScanMode mode) {
    return widget.sessionStorage.writeScanMode(
      scope: widget.scanPreferenceScope,
      page: 'inbound',
      mode: _modeToStorage(mode),
    );
  }

  String _modeToStorage(_InboundScanMode mode) {
    switch (mode) {
      case _InboundScanMode.scanConfirm:
        return 'scan_confirm';
      case _InboundScanMode.continuousScan:
        return 'continuous_scan';
    }
  }

  _InboundScanMode? _modeFromStorage(String? value) {
    switch (value?.trim()) {
      case 'quick_accumulate':
        return _InboundScanMode.continuousScan;
      case 'scan_confirm':
        return _InboundScanMode.scanConfirm;
      case 'continuous_scan':
        return _InboundScanMode.continuousScan;
      default:
        return null;
    }
  }

  @override
  void dispose() {
    _scanBarcodeFocusNode.dispose();
    _scanBarcodeController.dispose();
    _scanConfirmQtyController.dispose();
    _productIdController.dispose();
    _barcodeController.dispose();
    _qtyController.dispose();
    _unitCostController.dispose();
    _expectedVersionController.dispose();
    _remarkController.dispose();
    super.dispose();
  }

  List<_InboundDraftItem> get _effectiveDraftItems =>
      _draftItems.where((item) => _isDraftNotBlank(item)).toList();

  bool _isDraftNotBlank(_InboundDraftItem item) {
    return item.productId.trim().isNotEmpty ||
        item.barcode.trim().isNotEmpty ||
        item.qty.trim().isNotEmpty ||
        item.unitCost.trim().isNotEmpty ||
        item.expectedVersion.trim().isNotEmpty ||
        item.remark.trim().isNotEmpty;
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.controller,
      builder: (BuildContext context, Widget? child) {
        final effectiveDraftItems = _effectiveDraftItems;
        return Scaffold(
          appBar: AppBar(
            title: const Text('采购入库'),
            actions: [
              TextButton.icon(
                onPressed: () {
                  final now = DateTime.now();
                  final String today =
                      '${now.year.toString().padLeft(4, '0')}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}';
                  Navigator.of(context).push(
                    MaterialPageRoute<void>(
                      builder: (BuildContext context) => InboundLogsPage(
                        controller: widget.logsController,
                        initialStartDate: today,
                        initialEndDate: today,
                        initialPageSize: 10,
                      ),
                    ),
                  );
                },
                icon: const Icon(Icons.history_rounded, size: 18),
                label: const Text('入库记录'),
              ),
              const SizedBox(width: 4),
            ],
          ),
          body: ListView(
            padding: const EdgeInsets.all(16),
            children: <Widget>[
              const BrandHeroBanner(
                title: '采购入库作业',
                subtitle: '支持扫码确认、连续扫码会话与多商品明细批量提交',
                icon: Icons.move_to_inbox_rounded,
                gradientSeedColor: Color(0xFF10B981),
              ),
              const SizedBox(height: 12),
              // ── Inbound logs shortcut card ─────────────────────────────
              _InboundLogsShortcutCard(
                onTap: () {
                  final now = DateTime.now();
                  final String today =
                      '${now.year.toString().padLeft(4, '0')}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}';
                  Navigator.of(context).push(
                    MaterialPageRoute<void>(
                      builder: (BuildContext context) => InboundLogsPage(
                        controller: widget.logsController,
                        initialStartDate: today,
                        initialEndDate: today,
                        initialPageSize: 10,
                      ),
                    ),
                  );
                },
              ),
              const SizedBox(height: 12),
              if (!widget.controller.canSubmit)
                const StatusNotice(
                  message: '当前角色无采购入库权限，仅 OWNER/PURCHASER 可操作。',
                  tone: NoticeTone.warning,
                ),
              _ScanCard(
                barcodeController: _scanBarcodeController,
                scanning: widget.controller.scanning,
                submitting: widget.controller.submitting,
                canSubmit: widget.controller.canSubmit,
                scanErrorMessage: widget.controller.scanErrorMessage,
                scanSuccessMessage: widget.controller.scanSuccessMessage,
                scanMode: _scanMode,
                scanConfirmSessionActive: _scanConfirmSessionActive,
                scanConfirmProcessedCount: _scanConfirmProcessedCount,
                continuousSessionActive: _continuousSessionActive,
                continuousProcessedCount: _continuousProcessedCount,
                scanBarcodeFocusNode: _scanBarcodeFocusNode,
                onScan: _onPrimaryScanPressed,
                onCameraScan: _scanWithCamera,
                onBarcodeSubmitted: _onScanBarcodeSubmitted,
                onScanModeChanged: _onScanModeChanged,
                onStopScanConfirmSession: _stopScanConfirmSession,
                onClear: _resetScan,
              ),
              const SizedBox(height: 12),
              SectionCard(
                title: '高级录入区（手工）',
                subtitle: _manualFormExpanded
                    ? '可将当前表单加入“待提交明细”，用于异常补录'
                    : '默认折叠。需要手工补录时可展开。',
                action: TextButton(
                  onPressed: widget.controller.submitting
                      ? null
                      : () {
                          setState(() {
                            _manualFormExpanded = !_manualFormExpanded;
                          });
                        },
                  child: Text(_manualFormExpanded ? '收起' : '展开'),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    if (_manualFormExpanded) ...<Widget>[
                      _buildTextField(
                        controller: _productIdController,
                        label: '商品ID',
                        hint: '可选，正整数',
                      ),
                      const SizedBox(height: 8),
                      _buildTextField(
                        controller: _barcodeController,
                        label: '条码',
                        hint: '可选，product_id 与 barcode 二选一即可（支持扫码枪与手机摄像头）',
                        suffixIcon: IconButton(
                          tooltip: '扫码填充条码',
                          onPressed: widget.controller.scanning ||
                                  widget.controller.submitting ||
                                  !widget.controller.canSubmit
                              ? null
                              : _fillFormBarcodeByCamera,
                          icon: const Icon(Icons.qr_code_scanner_rounded),
                        ),
                      ),
                      const SizedBox(height: 8),
                      Row(
                        children: <Widget>[
                          Expanded(
                            child: _buildTextField(
                              controller: _qtyController,
                              label: '数量 *',
                              hint: '正整数',
                              keyboardType: TextInputType.number,
                            ),
                          ),
                          const SizedBox(width: 8),
                          Expanded(
                            child: _buildTextField(
                              controller: _unitCostController,
                              label: '进货价格',
                              hint: '例如：2.20',
                              keyboardType:
                                  const TextInputType.numberWithOptions(
                                      decimal: true),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 8),
                      _buildTextField(
                        controller: _expectedVersionController,
                        label: '版本',
                        hint: '可选 expected_version',
                        keyboardType: TextInputType.number,
                      ),
                      const SizedBox(height: 8),
                      _buildTextField(
                        controller: _remarkController,
                        label: '备注',
                        hint: '可选备注',
                      ),
                      const SizedBox(height: 12),
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: <Widget>[
                          OutlinedButton(
                            onPressed: widget.controller.submitting ||
                                    !widget.controller.canSubmit
                                ? null
                                : _addCurrentFormToDraft,
                            child: const Text('加入明细'),
                          ),
                          OutlinedButton(
                            onPressed: widget.controller.submitting
                                ? null
                                : _resetForm,
                            child: const Text('重置'),
                          ),
                        ],
                      ),
                    ],
                    const SizedBox(height: 8),
                    Text(
                      '提交规则：1 条明细走单条接口；2 条及以上走批量接口。当前待提交 ${effectiveDraftItems.length} 条。',
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                    if (widget.controller.errorMessage != null) ...<Widget>[
                      const SizedBox(height: 12),
                      StatusNotice(
                        message: widget.controller.errorMessage!,
                        tone: NoticeTone.error,
                      ),
                    ],
                    if (widget.controller.successMessage != null) ...<Widget>[
                      const SizedBox(height: 12),
                      StatusNotice(
                        message: widget.controller.successMessage!,
                        tone: NoticeTone.success,
                      ),
                    ],
                  ],
                ),
              ),
              const SizedBox(height: 12),
              _DraftItemsCard(
                items: effectiveDraftItems,
                submitting: widget.controller.submitting,
                canSubmit: widget.controller.canSubmit,
                onSubmit: _submit,
                onRemove: _removeDraftItem,
                onUpdateUnitCost: _updateDraftItemUnitCost,
              ),
              if (widget.controller.result != null) ...<Widget>[
                const SizedBox(height: 12),
                _ResultCard(result: widget.controller.result!),
              ],
              if (widget.controller.batchResult != null) ...<Widget>[
                const SizedBox(height: 12),
                _BatchResultCard(result: widget.controller.batchResult!),
              ],
            ],
          ),
        );
      },
    );
  }

  Widget _buildTextField({
    required TextEditingController controller,
    required String label,
    required String hint,
    TextInputType? keyboardType,
    Widget? suffixIcon,
  }) {
    return TextField(
      controller: controller,
      keyboardType: keyboardType,
      decoration: InputDecoration(
        labelText: label,
        hintText: hint,
        border: const OutlineInputBorder(),
        isDense: true,
        suffixIcon: suffixIcon,
      ),
    );
  }

  void _onScanModeChanged(_InboundScanMode mode) {
    setState(() {
      _scanMode = mode;
      if (mode != _InboundScanMode.scanConfirm) {
        _scanConfirmSessionActive = false;
        _scanConfirmProcessedCount = 0;
      }
      if (mode != _InboundScanMode.continuousScan) {
        _continuousSessionActive = false;
      }
    });
    unawaited(_persistScanMode(mode));
  }

  void _onScanBarcodeSubmitted(String _) {
    if (_scanMode == _InboundScanMode.continuousScan ||
        widget.controller.scanning ||
        widget.controller.submitting ||
        !widget.controller.canSubmit) {
      return;
    }

    unawaited(_scanByCurrentMode());
  }

  Future<void> _onPrimaryScanPressed() async {
    if (_scanMode != _InboundScanMode.continuousScan &&
        _scanBarcodeController.text.trim().isEmpty) {
      await _scanWithCamera();
      return;
    }

    await _scanByCurrentMode();
  }

  Future<void> _scanByCurrentMode() async {
    switch (_scanMode) {
      case _InboundScanMode.scanConfirm:
        await _scanForConfirm();
        return;
      case _InboundScanMode.continuousScan:
        await _startContinuousScanSession();
        return;
    }
  }

  Future<void> _scanForConfirm() async {
    await _scanForConfirmByBarcode(_scanBarcodeController.text.trim());
  }

  Future<void> _scanWithCamera() async {
    if (widget.controller.scanning ||
        widget.controller.submitting ||
        !widget.controller.canSubmit) {
      return;
    }

    FocusScope.of(context).unfocus();

    if (_scanMode == _InboundScanMode.continuousScan) {
      await _startContinuousScanSession();
      return;
    }

    final String? barcode = await BarcodeScannerSheet.scan(
      context,
      title: '入库扫码',
      hint: '将条码对准取景框，识别成功后会自动回填并按当前模式处理。',
    );
    if (!mounted || barcode == null || barcode.isEmpty) {
      return;
    }

    _scanBarcodeController.text = barcode;
    await _scanByCurrentMode();
  }

  Future<void> _scanForConfirmByBarcode(String barcode) async {
    FocusScope.of(context).unfocus();

    final InboundScanPreview? preview = await widget.controller.scanForConfirm(
      barcode: barcode,
    );
    if (preview == null) {
      await _handleScanNotFoundAndCreate(barcode: barcode.trim());
      return;
    }

    setState(() {
      _scanConfirmProductId = preview.productIdText;
      _scanConfirmProductName = preview.productName;
      _scanConfirmBarcode = preview.barcodeText;
      _scanConfirmUnitCost = preview.unitCostText;
      _scanConfirmQtyController.text = '1';
      _scanConfirmSessionActive = true;
    });

    await _showScanConfirmDialog();
  }

  Future<void> _showScanConfirmDialog() async {
    if (!mounted) {
      return;
    }

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
                textInputAction: TextInputAction.done,
                onTap: () {
                  _scanConfirmQtyController.selection = TextSelection(
                    baseOffset: 0,
                    extentOffset: _scanConfirmQtyController.text.length,
                  );
                },
                onSubmitted: (_) {
                  final ok = _applyScanConfirm();
                  if (ok) {
                    Navigator.of(context).pop(true);
                  }
                },
                decoration: const InputDecoration(
                  labelText: '数量 *',
                  hintText: '1',
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

    if (!mounted) {
      return;
    }

    if (confirmed != true && _scanMode == _InboundScanMode.scanConfirm) {
      _scanBarcodeController.clear();
      _scanBarcodeFocusNode.requestFocus();
      _showSnack('已取消本次确认，可继续扫码');
    }
  }

  bool _applyScanConfirm() {
    final InboundScanResult? result = widget.controller.applyScanConfirmed(
      productId: _scanConfirmProductId,
      productName: _scanConfirmProductName,
      barcode: _scanConfirmBarcode,
      qty: _scanConfirmQtyController.text,
      unitCost: _scanConfirmUnitCost,
      currentProductId: _productIdController.text,
      currentBarcode: _barcodeController.text,
      currentQty: _qtyController.text,
    );
    if (result == null) {
      return false;
    }

    _upsertDraftItem(
      productId: result.productIdText,
      barcode: result.barcodeText,
      productName: result.productName,
      qty: result.qtyText,
      unitCost: result.unitCostText,
      expectedVersion: '',
      remark: '',
    );
    _scanBarcodeController.clear();
    setState(() {
      if (_scanMode == _InboundScanMode.scanConfirm) {
        _scanConfirmSessionActive = true;
        _scanConfirmProcessedCount += 1;
      }
    });

    if (_scanMode == _InboundScanMode.scanConfirm) {
      _showSnack('确认写入成功，已加入入库明细（共 ${_effectiveDraftItems.length} 条）');
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

  Future<void> _startContinuousScanSession() async {
    if (_continuousSessionActive ||
        widget.controller.submitting ||
        !widget.controller.canSubmit) {
      return;
    }

    FocusScope.of(context).unfocus();
    setState(() {
      _continuousSessionActive = true;
      _continuousProcessedCount = 0;
    });

    await BarcodeScannerSheet.scanContinuous(
      context,
      title: '入库连续扫码',
      hint: '摄像头持续开启，识别到条码后自动加入入库明细，完成后点击“结束扫码”。',
      onScanned: (String barcode) async {
        if (!mounted) {
          return false;
        }

        _scanBarcodeController.text = barcode;
        final InboundScanResult? result =
            await widget.controller.scanAndAccumulate(
          barcode: barcode,
          currentProductId: _productIdController.text,
          currentBarcode: _barcodeController.text,
          currentQty: _qtyController.text,
        );
        if (!mounted || result == null) {
          return false;
        }

        _upsertDraftItem(
          productId: result.productIdText,
          barcode: result.barcodeText,
          productName: result.productName,
          qty: result.qtyText,
          unitCost: result.unitCostText,
          expectedVersion: '',
          remark: '',
        );
        _scanBarcodeController.clear();
        setState(() {
          _continuousProcessedCount += 1;
        });
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

  Future<void> _fillFormBarcodeByCamera() async {
    if (widget.controller.scanning ||
        widget.controller.submitting ||
        !widget.controller.canSubmit) {
      return;
    }

    FocusScope.of(context).unfocus();

    final String? barcode = await BarcodeScannerSheet.scan(
      context,
      title: '入库表单条码录入',
      hint: '将条码对准取景框，识别成功后会自动回填到入库表单条码字段。',
    );
    if (!mounted || barcode == null || barcode.isEmpty) {
      return;
    }

    _barcodeController.text = barcode;
  }

  void _resetScan() {
    _scanBarcodeController.clear();
    _scanConfirmQtyController.text = '1';
    setState(() {
      _scanConfirmProductId = '';
      _scanConfirmProductName = '';
      _scanConfirmBarcode = '';
      _scanConfirmUnitCost = '';
      _scanConfirmSessionActive = false;
      _scanConfirmProcessedCount = 0;
      _continuousSessionActive = false;
      _continuousProcessedCount = 0;
    });
    widget.controller.clearScanMessages();
  }

  void _addCurrentFormToDraft() {
    final productId = _productIdController.text.trim();
    final barcode = _barcodeController.text.trim();
    final qty = _qtyController.text.trim();
    final unitCost = _unitCostController.text.trim();
    final expectedVersion = _expectedVersionController.text.trim();
    final remark = _remarkController.text.trim();

    if (productId.isEmpty && barcode.isEmpty) {
      _showSnack('加入明细前请先填写商品ID或条码');
      return;
    }
    if (productId.isNotEmpty && int.tryParse(productId) == null) {
      _showSnack('商品ID必须为正整数');
      return;
    }
    final qtyValue = int.tryParse(qty);
    if (qtyValue == null || qtyValue <= 0) {
      _showSnack('数量必须为正整数');
      return;
    }
    if (unitCost.isNotEmpty && !_moneyPattern.hasMatch(unitCost)) {
      _showSnack('进货价格式错误（示例：2.20）');
      return;
    }
    if (expectedVersion.isNotEmpty) {
      final expectedVersionValue = int.tryParse(expectedVersion);
      if (expectedVersionValue == null || expectedVersionValue < 0) {
        _showSnack('版本必须为大于等于 0 的整数');
        return;
      }
    }

    _upsertDraftItem(
      productId: productId,
      barcode: barcode,
      productName: remark.isNotEmpty
          ? remark
          : '商品#${productId.isNotEmpty ? productId : barcode}',
      qty: qty,
      unitCost: unitCost,
      expectedVersion: expectedVersion,
      remark: remark,
    );
    _clearFormOnly();
    _showSnack('已加入入库明细（共 ${_effectiveDraftItems.length} 条）');
  }

  Future<void> _submit() async {
    FocusScope.of(context).unfocus();
    final effectiveDraftItems = _effectiveDraftItems;

    bool ok = false;
    if (effectiveDraftItems.length >= 2) {
      ok = await widget.controller.submitBatch(
        items: effectiveDraftItems
            .map(
              (item) => InboundBatchFormItemInput(
                productId: item.productId,
                barcode: item.barcode,
                qty: item.qty,
                unitCost: item.unitCost,
                expectedVersion: item.expectedVersion,
                remark: item.remark,
              ),
            )
            .toList(),
      );
    } else if (effectiveDraftItems.length == 1) {
      final item = effectiveDraftItems.first;
      ok = await widget.controller.submit(
        productId: item.productId,
        barcode: item.barcode,
        qty: item.qty,
        unitCost: item.unitCost,
        expectedVersion: item.expectedVersion,
        remark: item.remark,
      );
    } else {
      _showSnack('请先加入至少 1 条待提交明细，再提交入库。');
      return;
    }

    if (!ok) {
      return;
    }

    _clearFormOnly();
    _draftItems.clear();
    _resetScan();
    setState(() {});

    // —— 批次关联弹窗（入库成功后） ——————————————————————————
    final bc = widget.batchController;
    if (bc == null || !mounted) return;

    // 从入库结果中收集需要批次管理的商品（去重，每个商品弹一次）
    final List<({int productId, String productName, bool trackBatches})>
        pendingBatchProducts = [];
    final Set<int> seen = {};

    // 单条入库
    final singleResult = widget.controller.result;
    if (singleResult != null && singleResult.trackBatches) {
      final int pid = singleResult.productId;
      if (!seen.contains(pid)) {
        seen.add(pid);
        // 寻找对应的商品名
        final String name = effectiveDraftItems
            .firstWhere(
              (i) => i.productId == pid.toString(),
              orElse: () => effectiveDraftItems.first,
            )
            .productName;
        pendingBatchProducts.add(
          (productId: pid, productName: name, trackBatches: true),
        );
      }
    }

    // 批量入库
    final batchResult = widget.controller.batchResult;
    if (batchResult != null) {
      for (final item in batchResult.items) {
        if (item.trackBatches && !seen.contains(item.productId)) {
          seen.add(item.productId);
          final String name = effectiveDraftItems
              .firstWhere(
                (d) => d.productId == item.productId.toString(),
                orElse: () => effectiveDraftItems.first,
              )
              .productName;
          pendingBatchProducts.add(
            (productId: item.productId, productName: name, trackBatches: true),
          );
        }
      }
    }

    // 逐一弹出 BatchLinkSheet
    for (final p in pendingBatchProducts) {
      if (!mounted) break;
      // ignore: use_build_context_synchronously
      await BatchLinkSheet.show(
        context,
        productId: p.productId,
        productName: p.productName,
        controller: bc,
      );
    }
  }

  void _resetForm() {
    _clearFormOnly();
    _draftItems.clear();
    _resetScan();
    widget.controller.clearMessages();
    widget.controller.clearResult();
    setState(() {});
  }

  void _clearFormOnly() {
    _productIdController.clear();
    _barcodeController.clear();
    _qtyController.text = '1';
    _unitCostController.clear();
    _expectedVersionController.clear();
    _remarkController.clear();
  }

  void _upsertDraftItem({
    required String productId,
    required String barcode,
    required String productName,
    required String qty,
    required String unitCost,
    required String expectedVersion,
    required String remark,
  }) {
    final parsedQty = int.tryParse(qty.trim()) ?? 0;
    if (parsedQty <= 0) {
      return;
    }

    final existedIndex = _draftItems.indexWhere(
      (item) => item.matches(productId: productId, barcode: barcode),
    );
    if (existedIndex >= 0) {
      final existed = _draftItems[existedIndex];
      final currentQty = int.tryParse(existed.qty.trim()) ?? 0;
      _draftItems[existedIndex] = _InboundDraftItem(
        localId: existed.localId,
        productId: existed.productId,
        barcode: existed.barcode,
        productName: existed.productName.trim().isNotEmpty
            ? existed.productName
            : productName,
        qty: (currentQty + parsedQty).toString(),
        unitCost:
            existed.unitCost.trim().isNotEmpty ? existed.unitCost : unitCost,
        expectedVersion: existed.expectedVersion.trim().isNotEmpty
            ? existed.expectedVersion
            : expectedVersion,
        remark: existed.remark.trim().isNotEmpty ? existed.remark : remark,
      );
      setState(() {});
      return;
    }

    _draftItems.add(
      _InboundDraftItem(
        localId: _draftSeed++,
        productId: productId.trim(),
        barcode: barcode.trim(),
        productName: productName.trim(),
        qty: parsedQty.toString(),
        unitCost: unitCost.trim(),
        expectedVersion: expectedVersion.trim(),
        remark: remark.trim(),
      ),
    );
    setState(() {});
  }

  void _removeDraftItem(int localId) {
    _draftItems.removeWhere((item) => item.localId == localId);
    setState(() {});
  }

  void _updateDraftItemUnitCost(int localId, String rawValue) {
    final value = rawValue.trim();
    if (value.isNotEmpty && !_moneyPattern.hasMatch(value)) {
      _showSnack('进货价格式错误（示例：2.20）');
      return;
    }

    final index = _draftItems.indexWhere((item) => item.localId == localId);
    if (index < 0) {
      return;
    }

    final current = _draftItems[index];
    _draftItems[index] = _InboundDraftItem(
      localId: current.localId,
      productId: current.productId,
      barcode: current.barcode,
      productName: current.productName,
      qty: current.qty,
      unitCost: value,
      expectedVersion: current.expectedVersion,
      remark: current.remark,
    );
    setState(() {});
  }

  void _showSnack(String message) {
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(message)));
  }

  Future<void> _handleScanNotFoundAndCreate({required String barcode}) async {
    final String? missingBarcodeRaw = widget.controller.pendingMissingBarcode;
    final String effectiveBarcode = missingBarcodeRaw?.trim() ?? '';

    if (effectiveBarcode.isEmpty || !widget.controller.canSubmit) {
      return;
    }

    final bool shouldCreate = await showDialog<bool>(
          context: context,
          builder: (BuildContext context) {
            return AlertDialog(
              title: const Text('条码未建档'),
              content: Text('条码 $effectiveBarcode 未找到对应商品，是否立即新建商品？'),
              actions: <Widget>[
                TextButton(
                  onPressed: () => Navigator.of(context).pop(false),
                  child: const Text('继续扫码'),
                ),
                FilledButton(
                  onPressed: () => Navigator.of(context).pop(true),
                  child: const Text('新建商品'),
                ),
              ],
            );
          },
        ) ??
        false;

    if (!shouldCreate || !mounted) {
      return;
    }

    widget.controller.consumePendingMissingBarcode();

    final ProductData? created = await showModalBottomSheet<ProductData>(
      context: context,
      useSafeArea: true,
      isScrollControlled: true,
      builder: (BuildContext context) {
        return CreateProductSheet(
          controller: widget.productController,
          moneyPattern: _moneyPattern,
          presetBarcode: effectiveBarcode,
        );
      },
    );

    if (!mounted || created == null) {
      return;
    }

    _productIdController.text = created.id.toString();
    _barcodeController.text = created.barcode;
    _scanBarcodeController.text = created.barcode;
    if (_qtyController.text.trim().isEmpty ||
        _qtyController.text.trim() == '0') {
      _qtyController.text = '1';
    }

    _showSnack('商品已新建并回填：#${created.id} ${created.name}，请继续加入明细或提交');
  }
}

class _ScanCard extends StatelessWidget {
  const _ScanCard({
    required this.barcodeController,
    required this.scanning,
    required this.submitting,
    required this.canSubmit,
    required this.scanErrorMessage,
    required this.scanSuccessMessage,
    required this.scanMode,
    required this.scanConfirmSessionActive,
    required this.scanConfirmProcessedCount,
    required this.continuousSessionActive,
    required this.continuousProcessedCount,
    required this.scanBarcodeFocusNode,
    required this.onScan,
    required this.onCameraScan,
    required this.onBarcodeSubmitted,
    required this.onScanModeChanged,
    required this.onStopScanConfirmSession,
    required this.onClear,
  });

  final TextEditingController barcodeController;
  final bool scanning;
  final bool submitting;
  final bool canSubmit;
  final String? scanErrorMessage;
  final String? scanSuccessMessage;
  final _InboundScanMode scanMode;
  final bool scanConfirmSessionActive;
  final int scanConfirmProcessedCount;
  final bool continuousSessionActive;
  final int continuousProcessedCount;
  final FocusNode scanBarcodeFocusNode;
  final VoidCallback onScan;
  final VoidCallback onCameraScan;
  final ValueChanged<String> onBarcodeSubmitted;
  final ValueChanged<_InboundScanMode> onScanModeChanged;
  final VoidCallback onStopScanConfirmSession;
  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) {
    return SectionCard(
      title: '条码入库',
      subtitle: '支持确认写入与连续扫码会话（命中后加入待提交明细）',
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
                selected: scanMode == _InboundScanMode.scanConfirm,
                onSelected: scanning || submitting
                    ? null
                    : (bool selected) {
                        if (selected) {
                          onScanModeChanged(_InboundScanMode.scanConfirm);
                        }
                      },
              ),
              ChoiceChip(
                label: const Text('连续扫码会话'),
                selected: scanMode == _InboundScanMode.continuousScan,
                onSelected: scanning || submitting
                    ? null
                    : (bool selected) {
                        if (selected) {
                          onScanModeChanged(_InboundScanMode.continuousScan);
                        }
                      },
              ),
            ],
          ),
          const SizedBox(height: 8),
          TextField(
            controller: barcodeController,
            focusNode: scanBarcodeFocusNode,
            onSubmitted: onBarcodeSubmitted,
            decoration: InputDecoration(
              labelText: '条码',
              hintText: '可输入/粘贴条码，或扫码枪回车，例如：690123456789',
              border: const OutlineInputBorder(),
              isDense: true,
              suffixIcon: IconButton(
                tooltip: '摄像头扫码',
                onPressed:
                    scanning || submitting || !canSubmit ? null : onCameraScan,
                icon: const Icon(Icons.qr_code_scanner),
              ),
            ),
          ),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
              FilledButton(
                onPressed: scanning || submitting || !canSubmit ? null : onScan,
                child: Text(
                  scanMode == _InboundScanMode.continuousScan
                      ? (continuousSessionActive ? '会话扫码中...' : '开始连续扫码')
                      : (scanning ? '识别中...' : '按条码处理'),
                ),
              ),
              OutlinedButton(
                onPressed: scanning || submitting ? null : onClear,
                child: const Text('清空'),
              ),
              if (scanMode == _InboundScanMode.scanConfirm &&
                  scanConfirmSessionActive)
                OutlinedButton(
                  onPressed:
                      scanning || submitting ? null : onStopScanConfirmSession,
                  child: const Text('结束确认续扫'),
                ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            scanMode == _InboundScanMode.continuousScan
                ? '连续扫码会话：${continuousSessionActive ? '进行中' : '未开始'}，已处理 $continuousProcessedCount 条。'
                : '确认续扫会话：${scanConfirmSessionActive ? '进行中' : '未开始'}，已处理 $scanConfirmProcessedCount 条。',
          ),
          if (scanErrorMessage != null) ...<Widget>[
            const SizedBox(height: 8),
            StatusNotice(
              message: scanErrorMessage!,
              tone: NoticeTone.error,
            ),
          ],
          if (scanSuccessMessage != null) ...<Widget>[
            const SizedBox(height: 8),
            StatusNotice(
              message: scanSuccessMessage!,
              tone: NoticeTone.success,
            ),
          ],
        ],
      ),
    );
  }
}

class _DraftItemsCard extends StatelessWidget {
  const _DraftItemsCard({
    required this.items,
    required this.submitting,
    required this.canSubmit,
    required this.onSubmit,
    required this.onRemove,
    required this.onUpdateUnitCost,
  });

  final List<_InboundDraftItem> items;
  final bool submitting;
  final bool canSubmit;
  final VoidCallback onSubmit;
  final ValueChanged<int> onRemove;
  final void Function(int localId, String unitCost) onUpdateUnitCost;

  @override
  Widget build(BuildContext context) {
    return SectionCard(
      title: '待提交入库明细（${items.length}）',
      footer: Wrap(
        spacing: 8,
        runSpacing: 8,
        children: <Widget>[
          FilledButton(
            onPressed:
                submitting || !canSubmit || items.isEmpty ? null : onSubmit,
            child: Text(submitting ? '提交中...' : '提交入库'),
          ),
          Text(
            items.isEmpty ? '请先加入明细，再提交入库。' : '请核对明细后提交入库。',
          ),
        ],
      ),
      child: Column(
        children: <Widget>[
          if (items.isEmpty)
            const Align(
              alignment: Alignment.centerLeft,
              child: Text('暂无待提交明细，可扫码或填写表单后点击“加入明细”。'),
            )
          else
            ...items.asMap().entries.map(
              (entry) {
                final index = entry.key;
                final item = entry.value;
                return Card(
                  margin: const EdgeInsets.only(bottom: 8),
                  child: Padding(
                    padding: const EdgeInsets.all(12),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          '#${index + 1} 商品ID：${item.productId.isEmpty ? '-' : item.productId} / 条码：${item.barcode.isEmpty ? '-' : item.barcode}',
                        ),
                        const SizedBox(height: 4),
                        Text(
                            '商品名称：${item.productName.isEmpty ? '-' : item.productName}'),
                        const SizedBox(height: 4),
                        Row(
                          children: <Widget>[
                            Expanded(
                              child: Text('数量：${item.qty}'),
                            ),
                            const SizedBox(width: 8),
                            SizedBox(
                              width: 180,
                              child: TextFormField(
                                key: ValueKey(
                                    'unit-cost-${item.localId}-${item.unitCost}'),
                                initialValue: item.unitCost,
                                enabled: !submitting,
                                keyboardType:
                                    const TextInputType.numberWithOptions(
                                        decimal: true),
                                decoration: const InputDecoration(
                                  labelText: '进货价格',
                                  hintText: '可选，例如：2.20',
                                  border: OutlineInputBorder(),
                                  isDense: true,
                                ),
                                onFieldSubmitted: (value) =>
                                    onUpdateUnitCost(item.localId, value),
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 4),
                        Text(
                            '版本：${item.expectedVersion.isEmpty ? '-' : item.expectedVersion}，备注：${item.remark.isEmpty ? '-' : item.remark}'),
                        const SizedBox(height: 8),
                        Align(
                          alignment: Alignment.centerRight,
                          child: OutlinedButton(
                            onPressed: submitting
                                ? null
                                : () => onRemove(item.localId),
                            child: const Text('删除'),
                          ),
                        ),
                      ],
                    ),
                  ),
                );
              },
            ),
        ],
      ),
    );
  }
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({required this.result});

  final InboundResultData result;

  @override
  Widget build(BuildContext context) {
    return SectionCard(
      title: '最近一次入库结果',
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
              const Text('当前进货价格',
                  style: TextStyle(color: Colors.grey, fontSize: 13)),
              Text(
                result.costPrice,
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
          Text('商品ID：${result.productId}',
              style: const TextStyle(fontSize: 13)),
          Text('当前库存：${result.currentStock}',
              style: const TextStyle(fontSize: 13)),
          Text('版本：${result.version}',
              style: const TextStyle(fontSize: 13, color: Colors.grey)),
        ],
      ),
    );
  }
}

class _BatchResultCard extends StatelessWidget {
  const _BatchResultCard({required this.result});

  final InboundBatchResultData result;

  @override
  Widget build(BuildContext context) {
    return SectionCard(
      title: '最近一次批量入库结果',
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
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 8),
            child: Divider(height: 1),
          ),
          ...result.items.map(
            (item) => Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: Text(
                '商品ID：${item.productId}，当前库存：${item.currentStock}，进货价：${item.costPrice}，版本：${item.version}',
                style: const TextStyle(fontSize: 12),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Inbound Logs Shortcut Card
// ════════════════════════════════════════════════════════════════════════════

class _InboundLogsShortcutCard extends StatelessWidget {
  const _InboundLogsShortcutCard({required this.onTap});
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    const Color accent = Color(0xFF10B981); // emerald green — matches inbound brand

    return Material(
      color: accent.withValues(alpha: 0.07),
      borderRadius: BorderRadius.circular(16),
      child: InkWell(
        borderRadius: BorderRadius.circular(16),
        onTap: onTap,
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: accent.withValues(alpha: 0.25)),
          ),
          child: Row(
            children: <Widget>[
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: accent.withValues(alpha: 0.14),
                  shape: BoxShape.circle,
                ),
                child: const Icon(
                  Icons.receipt_long_rounded,
                  color: accent,
                  size: 22,
                ),
              ),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    const Text(
                      '入库记录查询',
                      style: TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '查看历史入库流水，支持日期范围筛选',
                      style: TextStyle(
                        fontSize: 12,
                        color: cs.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              Icon(
                Icons.chevron_right_rounded,
                color: accent.withValues(alpha: 0.7),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
