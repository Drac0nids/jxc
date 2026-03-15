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
import '../../products/application/supplier_controller.dart';
import '../../products/models/product_models.dart';
import '../../products/presentation/batch_link_sheet.dart';
import '../../products/presentation/create_product_sheet.dart';
import '../../products/presentation/supplier_management_page.dart';

enum InboundScanMode { scanConfirm, continuousScan }

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
    this.supplierController,
    this.autoScan = false,
    this.initialScanMode,
  });

  final InboundController controller;
  final ProductController productController;
  final SessionStorage sessionStorage;
  final String scanPreferenceScope;
  final InboundLogsController logsController;
  final BatchController? batchController;
  final SupplierController? supplierController;
  /// 进入页面后自动触发摄像头扫码
  final bool autoScan;
  /// 预设扫码模式（仅当 autoScan=true 时生效）
  final InboundScanMode? initialScanMode;

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

  InboundScanMode _scanMode = InboundScanMode.scanConfirm;
  String _scanConfirmProductId = '';
  String _scanConfirmProductName = '';
  String _scanConfirmBarcode = '';
  String _scanConfirmUnitCost = '';
  int _scanConfirmCurrentStock = 0;
  String _scanConfirmRetailPrice = '';
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

  void _scheduleAutoScanIfNeeded() {
    if (!widget.autoScan) return;
    if (!mounted) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      if (!widget.controller.canSubmit) return;
      if (widget.initialScanMode != null) {
        setState(() => _scanMode = widget.initialScanMode!);
      }
      if (_scanMode == InboundScanMode.continuousScan) {
        unawaited(_startContinuousScanSession());
      } else {
        unawaited(_scanWithCamera());
      }
    });
  }

  Future<void> _restoreStoredScanMode() async {
    final storedMode = await widget.sessionStorage.readScanMode(
      scope: widget.scanPreferenceScope,
      page: 'inbound',
    );
    final normalizedStoredMode = storedMode?.trim();
    final parsedMode = widget.initialScanMode ??
        _modeFromStorage(normalizedStoredMode);
    if (!mounted) return;
    if (parsedMode != null) {
      setState(() {
        _scanMode = parsedMode;
      });
    }
    if (normalizedStoredMode != _modeToStorage(_scanMode)) {
      unawaited(_persistScanMode(_scanMode));
    }
    _scheduleAutoScanIfNeeded();
  }

  Future<void> _persistScanMode(InboundScanMode mode) {
    return widget.sessionStorage.writeScanMode(
      scope: widget.scanPreferenceScope,
      page: 'inbound',
      mode: _modeToStorage(mode),
    );
  }

  String _modeToStorage(InboundScanMode mode) {
    switch (mode) {
      case InboundScanMode.scanConfirm:
        return 'scan_confirm';
      case InboundScanMode.continuousScan:
        return 'continuous_scan';
    }
  }

  InboundScanMode? _modeFromStorage(String? value) {
    switch (value?.trim()) {
      case 'quick_accumulate':
        return InboundScanMode.continuousScan;
      case 'scan_confirm':
        return InboundScanMode.scanConfirm;
      case 'continuous_scan':
        return InboundScanMode.continuousScan;
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
              // 供应商管理（归属采购场景，放在入库页管理）
              if (widget.supplierController != null)
                IconButton(
                  tooltip: '供应商管理',
                  onPressed: () => Navigator.of(context).push(
                    MaterialPageRoute<void>(
                      builder: (_) => SupplierManagementPage(
                        controller: widget.supplierController!,
                      ),
                    ),
                  ),
                  icon: const Icon(Icons.handshake_outlined),
                ),
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
                onStopScanConfirmSession: _stopScanConfirmSession,
                onClear: _resetScan,
              ),
              const SizedBox(height: 12),
              // ── 手动补录区 ──────────────────────────────────────
              SectionCard(
                title: '手动补录',
                subtitle: _manualFormExpanded
                    ? '手动输入商品条码和数量，加入待提交明细'
                    : '扫码异常时可展开手动输入',
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
                        controller: _barcodeController,
                        label: '条码 *',
                        hint: '输入或扫描商品条码',
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
                              label: '进货单价',
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
                        controller: _remarkController,
                        label: '备注',
                        hint: '可选',
                      ),
                      const SizedBox(height: 12),
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: <Widget>[
                          FilledButton.tonal(
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


  void _onScanBarcodeSubmitted(String _) {
    if (_scanMode == InboundScanMode.continuousScan ||
        widget.controller.scanning ||
        widget.controller.submitting ||
        !widget.controller.canSubmit) {
      return;
    }

    unawaited(_scanByCurrentMode());
  }

  Future<void> _onPrimaryScanPressed() async {
    if (_scanMode != InboundScanMode.continuousScan &&
        _scanBarcodeController.text.trim().isEmpty) {
      await _scanWithCamera();
      return;
    }

    await _scanByCurrentMode();
  }

  Future<void> _scanByCurrentMode() async {
    switch (_scanMode) {
      case InboundScanMode.scanConfirm:
        await _scanForConfirm();
        return;
      case InboundScanMode.continuousScan:
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

    if (_scanMode == InboundScanMode.continuousScan) {
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
      _scanConfirmCurrentStock = preview.currentStock;
      _scanConfirmRetailPrice = preview.retailPrice;
      _scanConfirmQtyController.text = '1';
      _scanConfirmSessionActive = true;
    });

    await _showScanConfirmDialog();
  }

  Future<void> _showScanConfirmDialog() async {
    if (!mounted) return;

    final unitCostEditController = TextEditingController(text: _scanConfirmUnitCost);
    final cs = Theme.of(context).colorScheme;

    final bool? confirmed = await showModalBottomSheet<bool>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (BuildContext ctx) {
        return Padding(
          padding: EdgeInsets.fromLTRB(
            20, 16, 20, MediaQuery.of(ctx).viewInsets.bottom + 20,
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              // 标题
              Row(
                children: <Widget>[
                  Icon(Icons.check_circle_outline_rounded, color: cs.primary, size: 22),
                  const SizedBox(width: 8),
                  const Text('确认入库', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700)),
                ],
              ),
              const SizedBox(height: 16),
              // 商品信息卡
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: cs.primaryContainer.withValues(alpha: 0.3),
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(color: cs.primary.withValues(alpha: 0.2)),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      _scanConfirmProductName,
                      style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w700),
                    ),
                    const SizedBox(height: 6),
                    Wrap(
                      spacing: 16,
                      runSpacing: 4,
                      children: <Widget>[
                        _infoChip(cs, '当前库存', '$_scanConfirmCurrentStock'),
                        _infoChip(cs, '零售价', '¥$_scanConfirmRetailPrice'),
                        if (_scanConfirmUnitCost.isNotEmpty)
                          _infoChip(cs, '上次进货价', '¥$_scanConfirmUnitCost'),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
              // 数量 + 进货价
              Row(
                children: <Widget>[
                  Expanded(
                    child: TextField(
                      controller: _scanConfirmQtyController,
                      keyboardType: TextInputType.number,
                      autofocus: true,
                      textInputAction: TextInputAction.next,
                      onTap: () {
                        _scanConfirmQtyController.selection = TextSelection(
                          baseOffset: 0,
                          extentOffset: _scanConfirmQtyController.text.length,
                        );
                      },
                      decoration: const InputDecoration(
                        labelText: '入库数量 *',
                        hintText: '1',
                        border: OutlineInputBorder(),
                        isDense: true,
                      ),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: TextField(
                      controller: unitCostEditController,
                      keyboardType: const TextInputType.numberWithOptions(decimal: true),
                      textInputAction: TextInputAction.done,
                      onSubmitted: (_) {
                        _scanConfirmUnitCost = unitCostEditController.text.trim();
                        final ok = _applyScanConfirm();
                        if (ok) Navigator.of(ctx).pop(true);
                      },
                      decoration: const InputDecoration(
                        labelText: '进货单价',
                        hintText: '可选',
                        border: OutlineInputBorder(),
                        isDense: true,
                        prefixText: '¥ ',
                      ),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 20),
              // 操作按钮
              Row(
                children: <Widget>[
                  Expanded(
                    child: OutlinedButton(
                      onPressed: () => Navigator.of(ctx).pop(false),
                      child: const Text('取消'),
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    flex: 2,
                    child: FilledButton(
                      onPressed: widget.controller.submitting
                          ? null
                          : () {
                              _scanConfirmUnitCost = unitCostEditController.text.trim();
                              final ok = _applyScanConfirm();
                              if (ok) Navigator.of(ctx).pop(true);
                            },
                      child: const Text('确认入库'),
                    ),
                  ),
                ],
              ),
            ],
          ),
        );
      },
    );

    unitCostEditController.dispose();

    if (!mounted) return;

    if (confirmed != true && _scanMode == InboundScanMode.scanConfirm) {
      _scanBarcodeController.clear();
      _scanBarcodeFocusNode.requestFocus();
      _showSnack('已取消本次确认，可继续扫码');
    }
  }

  Widget _infoChip(ColorScheme cs, String label, String value) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Text(label, style: TextStyle(fontSize: 12, color: cs.onSurfaceVariant)),
        const SizedBox(width: 4),
        Text(value, style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600)),
      ],
    );
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
      if (_scanMode == InboundScanMode.scanConfirm) {
        _scanConfirmSessionActive = true;
        _scanConfirmProcessedCount += 1;
      }
    });

    if (_scanMode == InboundScanMode.scanConfirm) {
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
        if (!mounted) return null;

        _scanBarcodeController.text = barcode;
        final InboundScanResult? result =
            await widget.controller.scanAndAccumulate(
          barcode: barcode,
          currentProductId: _productIdController.text,
          currentBarcode: _barcodeController.text,
          currentQty: _qtyController.text,
        );
        if (!mounted || result == null) return null;

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
        // 返回显示标签供弹窗历史记录展示
        final int qty = int.tryParse(result.qtyText) ?? 1;
        return '${result.productName}  x$qty';
      },
      onUndo: (String barcode) {
        // 找到最近一条与该条码匹配的 draft item，qty - 1；变 0 则整行删除
        final int idx = _draftItems.lastIndexWhere(
          (item) => item.barcode == barcode,
        );
        if (idx < 0) return false;

        final item = _draftItems[idx];
        final int currentQty = int.tryParse(item.qty.trim()) ?? 0;
        if (currentQty <= 0) return false;

        setState(() {
          if (currentQty <= 1) {
            _draftItems.removeAt(idx);
          } else {
            _draftItems[idx] = _InboundDraftItem(
              localId: item.localId,
              productId: item.productId,
              barcode: item.barcode,
              productName: item.productName,
              qty: (currentQty - 1).toString(),
              unitCost: item.unitCost,
              expectedVersion: item.expectedVersion,
              remark: item.remark,
            );
          }
          _continuousProcessedCount =
              (_continuousProcessedCount - 1).clamp(0, 99999);
        });
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
      _scanConfirmCurrentStock = 0;
      _scanConfirmRetailPrice = '';
      _scanConfirmSessionActive = false;
      _scanConfirmProcessedCount = 0;
      _continuousSessionActive = false;
      _continuousProcessedCount = 0;
    });
    widget.controller.clearScanMessages();
  }

  void _addCurrentFormToDraft() {
    final barcode = _barcodeController.text.trim();
    final qty = _qtyController.text.trim();
    final unitCost = _unitCostController.text.trim();
    final remark = _remarkController.text.trim();

    if (barcode.isEmpty) {
      _showSnack('请先填写商品条码');
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

    _upsertDraftItem(
      productId: '',
      barcode: barcode,
      productName: remark.isNotEmpty ? remark : '商品#$barcode',
      qty: qty,
      unitCost: unitCost,
      expectedVersion: '',
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

    // —— 成功反馈弹窗 ——————————————————————————
    if (mounted) {
      await _showSuccessSheet();
    }

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

  Future<void> _showSuccessSheet() async {
    final cs = Theme.of(context).colorScheme;
    final singleResult = widget.controller.result;
    final batchResult = widget.controller.batchResult;

    // 构造摘要信息
    String bizNo = '';
    List<String> details = [];

    if (singleResult != null) {
      bizNo = singleResult.bizNo;
      details.add('商品#${singleResult.productId}  当前库存：${singleResult.currentStock}');
    }
    if (batchResult != null) {
      bizNo = batchResult.bizNo;
      for (final item in batchResult.items) {
        details.add('商品#${item.productId}  当前库存：${item.currentStock}');
      }
    }
    if (bizNo.isEmpty) return;

    await showModalBottomSheet<void>(
      context: context,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) {
        // 3秒后自动关闭
        Future.delayed(const Duration(seconds: 3), () {
          // ignore: use_build_context_synchronously
          if (Navigator.of(ctx).canPop()) {
            // ignore: use_build_context_synchronously
            Navigator.of(ctx).pop();
          }
        });
        return Padding(
          padding: const EdgeInsets.fromLTRB(24, 20, 24, 32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Container(
                padding: const EdgeInsets.all(14),
                decoration: BoxDecoration(
                  color: const Color(0xFF10B981).withValues(alpha: 0.12),
                  shape: BoxShape.circle,
                ),
                child: const Icon(
                  Icons.check_rounded,
                  color: Color(0xFF10B981),
                  size: 36,
                ),
              ),
              const SizedBox(height: 14),
              const Text(
                '入库成功',
                style: TextStyle(fontSize: 20, fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 6),
              Text(
                '单号：$bizNo',
                style: TextStyle(fontSize: 13, color: cs.onSurfaceVariant),
              ),
              if (details.isNotEmpty) ...<Widget>[
                const SizedBox(height: 12),
                ...details.map(
                  (d) => Padding(
                    padding: const EdgeInsets.only(bottom: 4),
                    child: Text(d, style: const TextStyle(fontSize: 13)),
                  ),
                ),
              ],
              const SizedBox(height: 16),
              SizedBox(
                width: double.infinity,
                child: FilledButton(
                  onPressed: () => Navigator.of(ctx).pop(),
                  child: const Text('继续入库'),
                ),
              ),
            ],
          ),
        );
      },
    );
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
    required this.onStopScanConfirmSession,
    required this.onClear,
  });

  final TextEditingController barcodeController;
  final bool scanning;
  final bool submitting;
  final bool canSubmit;
  final String? scanErrorMessage;
  final String? scanSuccessMessage;
  final InboundScanMode scanMode;
  final bool scanConfirmSessionActive;
  final int scanConfirmProcessedCount;
  final bool continuousSessionActive;
  final int continuousProcessedCount;
  final FocusNode scanBarcodeFocusNode;
  final VoidCallback onScan;
  final VoidCallback onCameraScan;
  final ValueChanged<String> onBarcodeSubmitted;
  final VoidCallback onStopScanConfirmSession;
  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) {
    return SectionCard(
      title: '扫码',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[        const SizedBox(height: 4),

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
                  scanMode == InboundScanMode.continuousScan
                      ? (continuousSessionActive ? '扫码中...' : '开始连续扫码')
                      : (scanning ? '识别中...' : '扫码入库'),
                ),
              ),
              OutlinedButton(
                onPressed: scanning || submitting ? null : onClear,
                child: const Text('清空'),
              ),
              if (scanMode == InboundScanMode.scanConfirm &&
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
            scanMode == InboundScanMode.continuousScan
                ? '连续扫码：${continuousSessionActive ? '进行中' : '未开始'}，已处理 $continuousProcessedCount 条'
                : '确认续扫：${scanConfirmSessionActive ? '进行中' : '未开始'}，已处理 $scanConfirmProcessedCount 条',
            style: TextStyle(fontSize: 12, color: Theme.of(context).colorScheme.onSurfaceVariant),
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
    final cs = Theme.of(context).colorScheme;
    return SectionCard(
      title: '待提交明细（${items.length}）',
      footer: Row(
        children: <Widget>[
          Expanded(
            child: FilledButton.icon(
              onPressed:
                  submitting || !canSubmit || items.isEmpty ? null : onSubmit,
              icon: submitting
                  ? const SizedBox(
                      width: 16, height: 16,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.upload_rounded, size: 18),
              label: Text(submitting ? '提交中...' : '提交入库'),
            ),
          ),
        ],
      ),
      child: Column(
        children: <Widget>[
          if (items.isEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 24),
              child: Column(
                children: <Widget>[
                  Icon(Icons.inbox_outlined,
                      size: 40, color: cs.onSurfaceVariant.withValues(alpha: 0.4)),
                  const SizedBox(height: 8),
                  Text(
                    '暂无明细，扫码后自动加入',
                    style: TextStyle(
                      color: cs.onSurfaceVariant.withValues(alpha: 0.6),
                      fontSize: 13,
                    ),
                  ),
                ],
              ),
            )
          else
            ...items.asMap().entries.map(
              (entry) {
                final index = entry.key;
                final item = entry.value;
                final unitCostDisplay = item.unitCost.isNotEmpty
                    ? '¥${item.unitCost}'
                    : '';
                return Dismissible(
                  key: ValueKey('draft-${item.localId}'),
                  direction: DismissDirection.endToStart,
                  background: Container(
                    alignment: Alignment.centerRight,
                    padding: const EdgeInsets.only(right: 20),
                    margin: const EdgeInsets.only(bottom: 6),
                    decoration: BoxDecoration(
                      color: cs.errorContainer,
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Icon(Icons.delete_outline_rounded, color: cs.error),
                  ),
                  confirmDismiss: (_) async => !submitting,
                  onDismissed: (_) => onRemove(item.localId),
                  child: Card(
                    margin: const EdgeInsets.only(bottom: 6),
                    elevation: 0,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(12),
                      side: BorderSide(color: cs.outlineVariant.withValues(alpha: 0.5)),
                    ),
                    child: Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
                      child: Row(
                        children: <Widget>[
                          Container(
                            width: 24, height: 24,
                            alignment: Alignment.center,
                            decoration: BoxDecoration(
                              color: cs.primaryContainer.withValues(alpha: 0.5),
                              borderRadius: BorderRadius.circular(6),
                            ),
                            child: Text(
                              '${index + 1}',
                              style: TextStyle(
                                fontSize: 12, fontWeight: FontWeight.w700, color: cs.primary,
                              ),
                            ),
                          ),
                          const SizedBox(width: 10),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: <Widget>[
                                Text(
                                  item.productName.isNotEmpty
                                      ? item.productName
                                      : '商品#${item.productId.isNotEmpty ? item.productId : item.barcode}',
                                  style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 14),
                                  maxLines: 1, overflow: TextOverflow.ellipsis,
                                ),
                                if (item.barcode.isNotEmpty)
                                  Text(
                                    item.barcode,
                                    style: TextStyle(fontSize: 11, color: cs.onSurfaceVariant),
                                    maxLines: 1, overflow: TextOverflow.ellipsis,
                                  ),
                              ],
                            ),
                          ),
                          const SizedBox(width: 8),
                          Column(
                            crossAxisAlignment: CrossAxisAlignment.end,
                            children: <Widget>[
                              Text(
                                'x${item.qty}',
                                style: TextStyle(
                                  fontSize: 18, fontWeight: FontWeight.w800, color: cs.primary,
                                ),
                              ),
                              if (unitCostDisplay.isNotEmpty)
                                Text(
                                  unitCostDisplay,
                                  style: TextStyle(fontSize: 12, color: cs.onSurfaceVariant),
                                ),
                            ],
                          ),
                        ],
                      ),
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
