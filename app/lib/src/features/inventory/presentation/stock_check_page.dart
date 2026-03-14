import 'package:flutter/material.dart';

import '../../../core/widgets/barcode_scanner_sheet.dart';
import '../../../core/widgets/brand_ui.dart';
import '../application/stock_check_controller.dart';
import '../application/stock_check_logs_controller.dart';
import '../models/inventory_models.dart';
import 'stock_check_logs_page.dart';

class StockCheckPage extends StatefulWidget {
  const StockCheckPage({
    super.key,
    required this.controller,
    required this.logsController,
    this.initialProductId,
  });

  final StockCheckController controller;
  final StockCheckLogsController logsController;
  /// 预填的商品ID，从商品管理页跳转过来时使用
  final int? initialProductId;

  @override
  State<StockCheckPage> createState() => _StockCheckPageState();
}

class _StockCheckPageState extends State<StockCheckPage> {
  int _currentStep = 0;

  final TextEditingController _scanBarcodeController = TextEditingController();

  final TextEditingController _createRemarkController = TextEditingController();

  final TextEditingController _queryIdController = TextEditingController();
  final TextEditingController _startExpectedVersionController =
      TextEditingController();

  final TextEditingController _confirmExpectedVersionController =
      TextEditingController();
  final TextEditingController _confirmRemarkController =
      TextEditingController();

  int _createItemSeed = 1;
  final List<_CreateItemEditor> _createItemEditors = <_CreateItemEditor>[];
  final Map<int, TextEditingController> _confirmActualControllers =
      <int, TextEditingController>{};

  @override
  void initState() {
    super.initState();
    // 若有预填商品ID（从商品管理页跳入），直接填入第一行
    final preId = widget.initialProductId;
    _createItemEditors.add(
      preId != null
          ? _createCreateItemEditorFromInput(
              StockCheckCreateItemInput(productId: preId.toString()))
          : _createCreateItemEditor(),
    );
  }

  @override
  void dispose() {
    _scanBarcodeController.dispose();

    _createRemarkController.dispose();

    _queryIdController.dispose();
    _startExpectedVersionController.dispose();

    _confirmExpectedVersionController.dispose();
    _confirmRemarkController.dispose();

    for (final editor in _createItemEditors) {
      editor.dispose();
    }
    for (final controller in _confirmActualControllers.values) {
      controller.dispose();
    }

    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.controller,
      builder: (BuildContext context, Widget? child) {
        _syncConfirmEditorsFromController();

        return Scaffold(
          appBar: AppBar(
            title: const Text('库存盘点状态流'),
            actions: [
              TextButton.icon(
                onPressed: () {
                  final now = DateTime.now();
                  final String today =
                      '${now.year.toString().padLeft(4, '0')}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}';
                  Navigator.of(context).push(
                    MaterialPageRoute<void>(
                      builder: (BuildContext context) => StockCheckLogsPage(
                        controller: widget.logsController,
                        initialStartDate: today,
                        initialEndDate: today,
                        initialPageSize: 10,
                      ),
                    ),
                  );
                },
                icon: const Icon(Icons.history_rounded, size: 18),
                label: const Text('盘点历史'),
              ),
              const SizedBox(width: 4),
            ],
          ),
          body: ListView(
            padding: const EdgeInsets.all(16),
            children: <Widget>[
              const BrandHeroBanner(
                title: '库存盘点流程',
                subtitle: '创建、开始、确认三步闭环完成库存校准',
                icon: Icons.fact_check_rounded,
                gradientSeedColor: Color(0xFFEF4444),
              ),
              const SizedBox(height: 12),
              // ── History shortcut card ──────────────────────────────────────
              _HistoryShortcutCard(
                onTap: () {
                  final now = DateTime.now();
                  final String today =
                      '${now.year.toString().padLeft(4, '0')}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}';
                  Navigator.of(context).push(
                    MaterialPageRoute<void>(
                      builder: (BuildContext context) => StockCheckLogsPage(
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
              if (!widget.controller.canOperate)
                const StatusNotice(
                  message: '当前角色无库存盘点操作权限，仅 OWNER/PURCHASER 可操作。',
                  tone: NoticeTone.warning,
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
              if (widget.controller.result != null) ...<Widget>[
                const SizedBox(height: 12),
                _ResultCard(result: widget.controller.result!),
              ],
              const SizedBox(height: 12),
              Stepper(
                currentStep: _currentStep,
                controlsBuilder:
                    (BuildContext context, ControlsDetails details) {
                  return const SizedBox
                      .shrink(); // Hide default continue/cancel buttons
                },
                onStepTapped: (int step) {
                  setState(() {
                    _currentStep = step;
                  });
                },
                physics: const ClampingScrollPhysics(),
                steps: <Step>[
                  Step(
                    title: const Text('步骤 1：创建盘点单'),
                    subtitle: const Text('扫码或手工组织商品清单并创建'),
                    isActive: _currentStep == 0,
                    state: _currentStep > 0
                        ? StepState.complete
                        : StepState.indexed,
                    content: Column(
                      children: <Widget>[
                        _buildScanCard(context),
                        const SizedBox(height: 12),
                        _buildCreateCard(context),
                      ],
                    ),
                  ),
                  Step(
                    title: const Text('步骤 2：查询单据 / 开始盘点'),
                    subtitle: const Text('查询已有盘点单并流转到盘点中状态'),
                    isActive: _currentStep == 1,
                    state: _currentStep > 1
                        ? StepState.complete
                        : StepState.indexed,
                    content: _buildQueryAndStartCard(context),
                  ),
                  Step(
                    title: const Text('步骤 3：确认盘点差异'),
                    subtitle: const Text('录入实盘库存后回传系统，完成差异调整'),
                    isActive: _currentStep == 2,
                    content: _buildConfirmCard(context),
                  ),
                ],
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _buildScanCard(BuildContext context) {
    return SectionCard(
      title: '条码选品（创建盘点单）',
      subtitle: '按条码命中后自动追加到创建盘点明细',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          TextField(
            controller: _scanBarcodeController,
            decoration: InputDecoration(
              labelText: '条码',
              hintText: '扫码枪回车，例如：690123456789',
              border: const OutlineInputBorder(),
              isDense: true,
              suffixIcon: IconButton(
                tooltip: '摄像头扫码加入明细',
                onPressed:
                    widget.controller.busy || !widget.controller.canOperate
                        ? null
                        : _scanAndAppendCreateItemByCamera,
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
                onPressed:
                    widget.controller.busy || !widget.controller.canOperate
                        ? null
                        : _scanAndAppendCreateItem,
                child:
                    Text(widget.controller.loadingScan ? '识别中...' : '按条码加入明细'),
              ),
              OutlinedButton(
                onPressed: widget.controller.busy ? null : _resetScan,
                child: const Text('清空'),
              ),
            ],
          ),
          const SizedBox(height: 8),
          const Text('扫码命中后会自动追加到创建盘点明细；重复商品不会重复添加（支持扫码枪与手机摄像头）。'),
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

  Widget _buildCreateCard(BuildContext context) {
    return SectionCard(
      title: '组织盘点明细清单',
      subtitle: '先组织盘点商品清单并提交创建',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          _buildTextField(
            controller: _createRemarkController,
            label: '备注',
            hint: '可选备注',
          ),
          const SizedBox(height: 8),
          Row(
            children: <Widget>[
              const Text('盘点明细', style: TextStyle(fontWeight: FontWeight.w600)),
              const Spacer(),
              OutlinedButton(
                onPressed: widget.controller.busy ? null : _addCreateItem,
                child: const Text('新增明细'),
              ),
            ],
          ),
          const SizedBox(height: 8),
          ..._createItemEditors.map(_buildCreateItemCard),
          const SizedBox(height: 12),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: <Widget>[
              FilledButton(
                onPressed:
                    widget.controller.busy || !widget.controller.canOperate
                        ? null
                        : _createStockCheck,
                child:
                    Text(widget.controller.loadingCreate ? '创建中...' : '创建盘点单'),
              ),
              OutlinedButton(
                onPressed: widget.controller.busy ? null : _resetCreateForm,
                child: const Text('重置创建表单'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildQueryAndStartCard(BuildContext context) {
    return SectionCard(
      title: '查询与流转控制',
      subtitle: '查询盘点单详情并进入盘点中状态',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: _buildTextField(
                  controller: _queryIdController,
                  label: '盘点单ID *',
                  hint: '输入盘点单ID，例如 1',
                  keyboardType: TextInputType.number,
                ),
              ),
              const SizedBox(width: 8),
              FilledButton(
                onPressed: widget.controller.busy ? null : _queryStockCheck,
                child:
                    Text(widget.controller.loadingQuery ? '查询中...' : '查询盘点单'),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Row(
            children: <Widget>[
              Expanded(
                child: _buildTextField(
                  controller: _startExpectedVersionController,
                  label: '开始盘点 expected_version',
                  hint: '可选，建议默认当前版本',
                  keyboardType: TextInputType.number,
                ),
              ),
              const SizedBox(width: 8),
              FilledButton(
                onPressed: widget.controller.busy ||
                        !widget.controller.canOperate ||
                        widget.controller.result == null
                    ? null
                    : _startStockCheck,
                child: Text(widget.controller.loadingStart ? '开始中...' : '开始盘点'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildConfirmCard(BuildContext context) {
    final confirmItems = widget.controller.confirmItems;

    return SectionCard(
      title: '录入实盘',
      subtitle: '录入实盘库存并提交确认，生成差异结果',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          _buildTextField(
            controller: _confirmExpectedVersionController,
            label: '确认盘点 expected_version',
            hint: '可选，建议默认当前版本',
            keyboardType: TextInputType.number,
          ),
          const SizedBox(height: 8),
          _buildTextField(
            controller: _confirmRemarkController,
            label: '确认备注',
            hint: '可选备注',
          ),
          const SizedBox(height: 8),
          if (confirmItems.isEmpty)
            const Text('请先查询盘点单以加载确认明细')
          else
            ...confirmItems.map(_buildConfirmItemCard),
          const SizedBox(height: 12),
          FilledButton(
            onPressed: widget.controller.busy ||
                    !widget.controller.canOperate ||
                    widget.controller.result == null
                ? null
                : _confirmStockCheck,
            child: Text(widget.controller.loadingConfirm ? '确认中...' : '确认盘点'),
          ),
        ],
      ),
    );
  }

  Widget _buildCreateItemCard(_CreateItemEditor editor) {
    return SectionCard(
      title: '盘点明细 #${editor.localId}',
      action: TextButton(
        onPressed: widget.controller.busy || _createItemEditors.length <= 1
            ? null
            : () => _removeCreateItem(editor.localId),
        child: const Text('删除'),
      ),
      child: _buildTextField(
        controller: editor.productIdController,
        label: '商品ID *',
        hint: '1001',
        keyboardType: TextInputType.number,
      ),
    );
  }

  Widget _buildConfirmItemCard(StockCheckConfirmItemInput item) {
    final controller = _confirmActualControllers[item.productId];
    if (controller == null) {
      return const SizedBox.shrink();
    }

    return SectionCard(
      title: '商品ID：${item.productId}',
      subtitle: '账面库存：${item.bookStock}',
      child: _buildTextField(
        controller: controller,
        label: '实盘库存 *',
        hint: '请输入实盘库存',
        keyboardType: TextInputType.number,
      ),
    );
  }

  Widget _buildTextField({
    required TextEditingController controller,
    required String label,
    required String hint,
    TextInputType? keyboardType,
  }) {
    return TextField(
      controller: controller,
      keyboardType: keyboardType,
      decoration: InputDecoration(
        labelText: label,
        hintText: hint,
        border: const OutlineInputBorder(),
        isDense: true,
      ),
    );
  }

  Future<void> _scanAndAppendCreateItem() async {
    FocusScope.of(context).unfocus();

    final result = await widget.controller.scanAndAppendCreateItem(
      barcode: _scanBarcodeController.text,
      items: _collectCreateItems(),
    );

    if (result == null) {
      return;
    }

    _replaceCreateItems(result.items);
  }

  Future<void> _scanAndAppendCreateItemByCamera() async {
    if (widget.controller.busy || !widget.controller.canOperate) {
      return;
    }

    FocusScope.of(context).unfocus();

    final String? barcode = await BarcodeScannerSheet.scan(
      context,
      title: '盘点扫码',
      hint: '将条码对准取景框，识别成功后会自动回填并加入盘点明细。',
    );

    if (!mounted || barcode == null || barcode.isEmpty) {
      return;
    }

    _scanBarcodeController.text = barcode;
    await _scanAndAppendCreateItem();
  }

  Future<void> _createStockCheck() async {
    FocusScope.of(context).unfocus();

    final ok = await widget.controller.createStockCheck(
      remark: _createRemarkController.text,
      items: _collectCreateItems(),
    );
    if (!ok) {
      return;
    }

    final check = widget.controller.result;
    if (check != null) {
      _queryIdController.text = check.id.toString();
      _startExpectedVersionController.text = check.version.toString();
      _confirmExpectedVersionController.text = check.version.toString();
      _confirmRemarkController.text = check.remark ?? '';

      if (mounted) {
        setState(() {
          _currentStep = 1;
        });
      }
    }

    _resetCreateForm();
  }

  Future<void> _queryStockCheck() async {
    FocusScope.of(context).unfocus();

    final ok =
        await widget.controller.queryStockCheck(id: _queryIdController.text);
    if (!ok) {
      return;
    }

    final check = widget.controller.result;
    if (check != null) {
      _startExpectedVersionController.text = check.version.toString();
      _confirmExpectedVersionController.text = check.version.toString();
      _confirmRemarkController.text = check.remark ?? '';

      if (mounted) {
        if (check.status == 'DRAFT') {
          setState(() {
            _currentStep = 1;
          });
        } else if (check.status == 'COUNTING') {
          setState(() {
            _currentStep = 2;
          });
        }
      }
    }
  }

  Future<void> _startStockCheck() async {
    FocusScope.of(context).unfocus();

    final ok = await widget.controller.startStockCheck(
      expectedVersion: _startExpectedVersionController.text,
    );
    if (!ok) {
      return;
    }

    final check = widget.controller.result;
    if (check != null) {
      _confirmExpectedVersionController.text = check.version.toString();

      if (mounted) {
        setState(() {
          _currentStep = 2;
        });
      }
    }
  }

  Future<void> _confirmStockCheck() async {
    FocusScope.of(context).unfocus();

    final ok = await widget.controller.confirmStockCheck(
      expectedVersion: _confirmExpectedVersionController.text,
      remark: _confirmRemarkController.text,
    );
    if (!ok) {
      return;
    }

    final check = widget.controller.result;
    if (check != null) {
      _confirmExpectedVersionController.text = check.version.toString();
    }
  }

  void _resetScan() {
    _scanBarcodeController.clear();
    widget.controller.clearScanMessages();
  }

  void _resetCreateForm() {
    _createRemarkController.clear();
    _replaceCreateItems(
      <StockCheckCreateItemInput>[
        const StockCheckCreateItemInput(productId: '')
      ],
    );
    _resetScan();
  }

  void _addCreateItem() {
    setState(() {
      _createItemEditors.add(_createCreateItemEditor());
    });
  }

  void _removeCreateItem(int localId) {
    if (_createItemEditors.length <= 1) {
      return;
    }

    setState(() {
      final index =
          _createItemEditors.indexWhere((item) => item.localId == localId);
      if (index < 0) {
        return;
      }
      _createItemEditors[index].dispose();
      _createItemEditors.removeAt(index);
    });
  }

  List<StockCheckCreateItemInput> _collectCreateItems() {
    return _createItemEditors
        .map((item) =>
            StockCheckCreateItemInput(productId: item.productIdController.text))
        .toList();
  }

  void _replaceCreateItems(List<StockCheckCreateItemInput> items) {
    final nextItems = items.isEmpty
        ? <StockCheckCreateItemInput>[
            const StockCheckCreateItemInput(productId: '')
          ]
        : items;

    setState(() {
      for (final item in _createItemEditors) {
        item.dispose();
      }

      _createItemEditors
        ..clear()
        ..addAll(nextItems.map(_createCreateItemEditorFromInput));
    });
  }

  _CreateItemEditor _createCreateItemEditor() {
    return _createCreateItemEditorFromInput(
        const StockCheckCreateItemInput(productId: ''));
  }

  _CreateItemEditor _createCreateItemEditorFromInput(
      StockCheckCreateItemInput input) {
    return _CreateItemEditor(
      localId: _createItemSeed++,
      productIdController: TextEditingController(text: input.productId),
    );
  }

  void _syncConfirmEditorsFromController() {
    final confirmItems = widget.controller.confirmItems;
    final ids = confirmItems.map((item) => item.productId).toSet();

    final removedIds = _confirmActualControllers.keys
        .where((id) => !ids.contains(id))
        .toList();
    for (final id in removedIds) {
      _confirmActualControllers[id]?.dispose();
      _confirmActualControllers.remove(id);
    }

    for (final item in confirmItems) {
      final existing = _confirmActualControllers[item.productId];
      if (existing == null) {
        final controller = TextEditingController(text: item.actualStock);
        controller.addListener(() {
          widget.controller.updateConfirmActualStock(
            productId: item.productId,
            actualStock: controller.text,
          );
        });
        _confirmActualControllers[item.productId] = controller;
        continue;
      }

      if (existing.text != item.actualStock) {
        existing.text = item.actualStock;
      }
    }
  }
}

class _CreateItemEditor {
  _CreateItemEditor({
    required this.localId,
    required this.productIdController,
  });

  final int localId;
  final TextEditingController productIdController;

  void dispose() {
    productIdController.dispose();
  }
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({required this.result});

  final StockCheckData result;

  @override
  Widget build(BuildContext context) {
    return SectionCard(
      title: '最近一次盘点单结果',
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
              const Text('状态',
                  style: TextStyle(color: Colors.grey, fontSize: 13)),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: Colors.grey.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(999),
                ),
                child: Text(
                  result.status,
                  style: const TextStyle(
                    color: Colors.grey,
                    fontWeight: FontWeight.w700,
                    fontSize: 11,
                  ),
                ),
              ),
            ],
          ),
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 8),
            child: Divider(height: 1),
          ),
          Text('盘点单ID：${result.id}', style: const TextStyle(fontSize: 13)),
          Text('版本：${result.version}', style: const TextStyle(fontSize: 13)),
          Text('开始时间：${result.countingAt ?? '-'}',
              style: const TextStyle(fontSize: 13)),
          Text('确认时间：${result.confirmedAt ?? '-'}',
              style: const TextStyle(fontSize: 13)),
          Text('创建时间：${result.createdAt}',
              style: const TextStyle(fontSize: 13)),
          Text('更新时间：${result.updatedAt}',
              style: const TextStyle(fontSize: 13)),
          Text('备注：${result.remark ?? '-'}',
              style: const TextStyle(fontSize: 13)),
          const SizedBox(height: 8),
          ...result.items.map(
            (item) => Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: Text(
                '商品ID=${item.productId}，账面库存=${item.bookStock}，实盘库存=${item.actualStock ?? '-'}，差异数量=${item.deltaQty ?? '-'}',
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
// History Shortcut Card
// ════════════════════════════════════════════════════════════════════════════

class _HistoryShortcutCard extends StatelessWidget {
  const _HistoryShortcutCard({required this.onTap});
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    const Color accent = Color(0xFF8B5CF6); // purple

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
                  Icons.history_rounded,
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
                      '盘点历史流水',
                      style: TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '查看历史盘点差异记录，支持日期筛选',
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
