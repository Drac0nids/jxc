import 'package:flutter/material.dart';

import '../../../core/widgets/barcode_scanner_sheet.dart';
import '../../../core/widgets/brand_ui.dart';
import '../application/stock_check_controller.dart';
import '../application/stock_check_logs_controller.dart';
import '../models/inventory_models.dart';
import 'stock_check_logs_page.dart';

// ════════════════════════════════════════════════════════════════════════════
// Page
// ════════════════════════════════════════════════════════════════════════════

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
  // 当前步骤：0=准备盘点单  1=开始盘点  2=录入实盘
  int _currentStep = 0;

  // ── 步骤 0：创建 ──────────────────────────────────────────────────────────
  final TextEditingController _scanBarcodeController = TextEditingController();
  final TextEditingController _createRemarkController = TextEditingController();
  int _createItemSeed = 1;
  final List<_CreateItemEditor> _createItemEditors = <_CreateItemEditor>[];

  // ── 步骤 1：查询 / 开始 ───────────────────────────────────────────────────
  final TextEditingController _queryIdController = TextEditingController();

  // ── 步骤 2：确认 ──────────────────────────────────────────────────────────
  final TextEditingController _confirmRemarkController =
      TextEditingController();
  final Map<int, TextEditingController> _confirmActualControllers =
      <int, TextEditingController>{};

  @override
  void initState() {
    super.initState();
    final preId = widget.initialProductId;
    _createItemEditors.add(
      preId != null
          ? _makeEditorFromInput(
              StockCheckCreateItemInput(productId: preId.toString()))
          : _makeEditor(),
    );
  }

  @override
  void dispose() {
    _scanBarcodeController.dispose();
    _createRemarkController.dispose();
    _queryIdController.dispose();
    _confirmRemarkController.dispose();
    for (final e in _createItemEditors) {
      e.dispose();
    }
    for (final c in _confirmActualControllers.values) {
      c.dispose();
    }
    super.dispose();
  }

  // ══════════════════════════════════════════════════════════════════════════
  // Build
  // ══════════════════════════════════════════════════════════════════════════

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.controller,
      builder: (BuildContext context, Widget? child) {
        _syncConfirmEditors();

        final result = widget.controller.result;

        return Scaffold(
          appBar: AppBar(
            title: const Text('库存盘点'),
            actions: <Widget>[
              TextButton.icon(
                onPressed: _goHistory,
                icon: const Icon(Icons.history_rounded, size: 18),
                label: const Text('历史'),
              ),
              const SizedBox(width: 4),
            ],
          ),
          body: ListView(
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 32),
            children: <Widget>[
              const SizedBox(height: 12),
              // ── 操作状态提示 ──────────────────────────────────────────────
              if (!widget.controller.canOperate)
                const Padding(
                  padding: EdgeInsets.only(bottom: 12),
                  child: StatusNotice(
                    message: '当前角色无操作权限，仅 OWNER / 采购员 可操作',
                    tone: NoticeTone.warning,
                  ),
                ),
              if (widget.controller.errorMessage != null)
                Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: StatusNotice(
                    message: widget.controller.errorMessage!,
                    tone: NoticeTone.error,
                  ),
                ),
              if (widget.controller.successMessage != null)
                Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: StatusNotice(
                    message: widget.controller.successMessage!,
                    tone: NoticeTone.success,
                  ),
                ),

              // ── 当前盘点单信息 ────────────────────────────────────────────
              if (result != null) ...<Widget>[
                _ActiveCheckCard(result: result),
                const SizedBox(height: 16),
              ],

              // ── 步骤向导 ──────────────────────────────────────────────────
              _WizardStepper(
                currentStep: _currentStep,
                result: result,
                onStepTap: (int s) => setState(() => _currentStep = s),
                step0Content: _buildStep0(context),
                step1Content: _buildStep1(context, result),
                step2Content: _buildStep2(context, result),
              ),
            ],
          ),
        );
      },
    );
  }

  // ══════════════════════════════════════════════════════════════════════════
  // Step 0 — 组织商品清单并创建盘点单
  // ══════════════════════════════════════════════════════════════════════════

  Widget _buildStep0(BuildContext context) {
    final bool canAct =
        widget.controller.canOperate && !widget.controller.busy;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        // 扫码条
        _ScanBar(
          controller: _scanBarcodeController,
          loading: widget.controller.loadingScan,
          canAct: canAct,
          onScan: _scanAndAppend,
          onCamera: _scanByCamera,
          onClear: _clearScan,
          scanError: widget.controller.scanErrorMessage,
          scanSuccess: widget.controller.scanSuccessMessage,
        ),
        const SizedBox(height: 12),

        // 明细列表标题
        Row(
          children: <Widget>[
            const Text(
              '盘点商品',
              style: TextStyle(fontWeight: FontWeight.w700, fontSize: 14),
            ),
            const Spacer(),
            TextButton.icon(
              onPressed: canAct ? _addItem : null,
              icon: const Icon(Icons.add, size: 16),
              label: const Text('手工添加'),
            ),
          ],
        ),
        ..._createItemEditors.map(
          (e) => Padding(
            padding: const EdgeInsets.only(top: 8),
            child: _ItemInputRow(
              editor: e,
              canRemove: _createItemEditors.length > 1 && canAct,
              onRemove: () => _removeItem(e.localId),
            ),
          ),
        ),
        const SizedBox(height: 16),

        // 备注
        TextField(
          controller: _createRemarkController,
          decoration: const InputDecoration(
            labelText: '备注（可选）',
            border: OutlineInputBorder(),
            isDense: true,
          ),
        ),
        const SizedBox(height: 16),

        // 操作按钮
        Row(
          children: <Widget>[
            Expanded(
              child: FilledButton.icon(
                onPressed: canAct ? _createStockCheck : null,
                icon: widget.controller.loadingCreate
                    ? const _SmallSpinner()
                    : const Icon(Icons.playlist_add_check_rounded, size: 18),
                label: Text(
                    widget.controller.loadingCreate ? '创建中...' : '创建盘点单'),
              ),
            ),
            const SizedBox(width: 8),
            OutlinedButton(
              onPressed: canAct ? _resetStep0 : null,
              child: const Text('清空'),
            ),
          ],
        ),
      ],
    );
  }

  // ══════════════════════════════════════════════════════════════════════════
  // Step 1 — 开始盘点
  // ══════════════════════════════════════════════════════════════════════════

  Widget _buildStep1(BuildContext context, StockCheckData? result) {
    final bool canAct =
        widget.controller.canOperate && !widget.controller.busy;
    final bool hasResult = result != null;
    final bool isDraft = result?.status == 'DRAFT';
    final bool isCounting = result?.status == 'COUNTING';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        // 查询框（已有单据时收起）
        if (!hasResult || isDraft) ...<Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: TextField(
                  controller: _queryIdController,
                  keyboardType: TextInputType.number,
                  decoration: const InputDecoration(
                    labelText: '盘点单编号',
                    hintText: '输入编号查询，例如：1',
                    border: OutlineInputBorder(),
                    isDense: true,
                    prefixIcon:
                        Icon(Icons.search_rounded, size: 18),
                  ),
                ),
              ),
              const SizedBox(width: 8),
              FilledButton(
                onPressed: !widget.controller.busy ? _queryStockCheck : null,
                child: widget.controller.loadingQuery
                    ? const _SmallSpinner()
                    : const Text('查询'),
              ),
            ],
          ),
          const SizedBox(height: 16),
        ],

        if (isCounting)
          const StatusNotice(
            message: '该盘点单已处于「盘点中」状态，可直接前往步骤三录入实盘',
            tone: NoticeTone.info,
          )
        else if (hasResult && isDraft) ...<Widget>[
          _InfoRow(label: '盘点单', value: '#${result.id}（${result.bizNo}）'),
          _InfoRow(
              label: '商品数量',
              value: '${result.items.length} 种'),
          const SizedBox(height: 16),
          SizedBox(
            width: double.infinity,
            child: FilledButton.icon(
              onPressed: canAct ? _startStockCheck : null,
              icon: widget.controller.loadingStart
                  ? const _SmallSpinner()
                  : const Icon(Icons.play_arrow_rounded, size: 20),
              label: Text(widget.controller.loadingStart ? '开始中...' : '开始盘点'),
            ),
          ),
        ] else if (!hasResult)
          const Text(
            '创建盘点单后系统会自动跳到此步骤，也可输入编号查询已有的盘点单。',
            style: TextStyle(fontSize: 13, color: Colors.grey),
          ),
      ],
    );
  }

  // ══════════════════════════════════════════════════════════════════════════
  // Step 2 — 录入实盘并确认
  // ══════════════════════════════════════════════════════════════════════════

  Widget _buildStep2(BuildContext context, StockCheckData? result) {
    final bool canAct =
        widget.controller.canOperate && !widget.controller.busy;
    final confirmItems = widget.controller.confirmItems;
    final bool isCounting = result?.status == 'COUNTING';
    final bool isConfirmed = result?.status == 'CONFIRMED';

    if (isConfirmed && result != null) {
      return _ConfirmedResultPanel(result: result);
    }

    if (!isCounting) {
      return const Text(
        '请先在步骤二开始盘点，系统会自动跳到此步骤。',
        style: TextStyle(fontSize: 13, color: Colors.grey),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Text(
          '请逐一录入实际清点到的库存数量（账面库存已预填，修改后提交即可）',
          style: TextStyle(
              fontSize: 13,
              color: Theme.of(context).colorScheme.onSurfaceVariant),
        ),
        const SizedBox(height: 12),
        ...confirmItems.map((item) {
          final ctrl = _confirmActualControllers[item.productId];
          if (ctrl == null) return const SizedBox.shrink();
          return Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: _ConfirmItemRow(
              item: item,
              controller: ctrl,
              enabled: canAct,
            ),
          );
        }),
        const SizedBox(height: 8),
        TextField(
          controller: _confirmRemarkController,
          decoration: const InputDecoration(
            labelText: '确认备注（可选）',
            border: OutlineInputBorder(),
            isDense: true,
          ),
        ),
        const SizedBox(height: 16),
        SizedBox(
          width: double.infinity,
          child: FilledButton.icon(
            onPressed: canAct && confirmItems.isNotEmpty
                ? _confirmStockCheck
                : null,
            icon: widget.controller.loadingConfirm
                ? const _SmallSpinner()
                : const Icon(Icons.check_circle_rounded, size: 20),
            label: Text(
              widget.controller.loadingConfirm ? '提交中...' : '提交确认，完成盘点',
            ),
          ),
        ),
      ],
    );
  }

  // ══════════════════════════════════════════════════════════════════════════
  // Actions
  // ══════════════════════════════════════════════════════════════════════════

  void _goHistory() {
    final now = DateTime.now();
    final today =
        '${now.year.toString().padLeft(4, '0')}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}';
    Navigator.of(context).push(MaterialPageRoute<void>(
      builder: (_) => StockCheckLogsPage(
        controller: widget.logsController,
        initialStartDate: today,
        initialEndDate: today,
        initialPageSize: 10,
      ),
    ));
  }

  Future<void> _scanAndAppend() async {
    FocusScope.of(context).unfocus();
    final result = await widget.controller.scanAndAppendCreateItem(
      barcode: _scanBarcodeController.text,
      items: _collectItems(),
    );
    if (result == null) return;
    _replaceItems(result.items);
  }

  Future<void> _scanByCamera() async {
    if (!widget.controller.canOperate || widget.controller.busy) return;
    FocusScope.of(context).unfocus();
    final barcode = await BarcodeScannerSheet.scan(
      context,
      title: '盘点扫码',
      hint: '对准条码，识别后自动加入盘点列表',
    );
    if (!mounted || barcode == null || barcode.isEmpty) return;
    _scanBarcodeController.text = barcode;
    await _scanAndAppend();
  }

  void _clearScan() {
    _scanBarcodeController.clear();
    widget.controller.clearScanMessages();
  }

  void _addItem() {
    setState(() => _createItemEditors.add(_makeEditor()));
  }

  void _removeItem(int localId) {
    if (_createItemEditors.length <= 1) return;
    setState(() {
      final i =
          _createItemEditors.indexWhere((e) => e.localId == localId);
      if (i < 0) return;
      _createItemEditors[i].dispose();
      _createItemEditors.removeAt(i);
    });
  }

  Future<void> _createStockCheck() async {
    FocusScope.of(context).unfocus();
    final ok = await widget.controller.createStockCheck(
      remark: _createRemarkController.text,
      items: _collectItems(),
    );
    if (!ok) return;

    final check = widget.controller.result;
    if (check != null) {
      _queryIdController.text = check.id.toString();
    }
    _resetStep0();
    if (mounted) setState(() => _currentStep = 1);
  }

  Future<void> _queryStockCheck() async {
    FocusScope.of(context).unfocus();
    final ok = await widget.controller
        .queryStockCheck(id: _queryIdController.text);
    if (!ok || !mounted) return;

    final check = widget.controller.result;
    if (check == null) return;
    setState(() {
      if (check.status == 'COUNTING') {
        _currentStep = 2;
      } else {
        _currentStep = 1;
      }
    });
  }

  Future<void> _startStockCheck() async {
    FocusScope.of(context).unfocus();
    final ok = await widget.controller.startStockCheck(
      expectedVersion:
          widget.controller.suggestedStartExpectedVersion,
    );
    if (!ok || !mounted) return;
    setState(() => _currentStep = 2);
  }

  Future<void> _confirmStockCheck() async {
    FocusScope.of(context).unfocus();
    await widget.controller.confirmStockCheck(
      expectedVersion:
          widget.controller.suggestedStartExpectedVersion,
      remark: _confirmRemarkController.text,
    );
  }

  void _resetStep0() {
    _createRemarkController.clear();
    _replaceItems(<StockCheckCreateItemInput>[
      const StockCheckCreateItemInput(productId: ''),
    ]);
    _clearScan();
  }

  // ── Helpers ───────────────────────────────────────────────────────────────

  List<StockCheckCreateItemInput> _collectItems() {
    return _createItemEditors
        .map((e) =>
            StockCheckCreateItemInput(productId: e.productIdController.text))
        .toList();
  }

  void _replaceItems(List<StockCheckCreateItemInput> items) {
    final next = items.isEmpty
        ? <StockCheckCreateItemInput>[
            const StockCheckCreateItemInput(productId: '')
          ]
        : items;
    setState(() {
      for (final e in _createItemEditors) {
        e.dispose();
      }
      _createItemEditors
        ..clear()
        ..addAll(next.map(_makeEditorFromInput));
    });
  }

  _CreateItemEditor _makeEditor() =>
      _makeEditorFromInput(const StockCheckCreateItemInput(productId: ''));

  _CreateItemEditor _makeEditorFromInput(StockCheckCreateItemInput input) {
    return _CreateItemEditor(
      localId: _createItemSeed++,
      productIdController: TextEditingController(text: input.productId),
    );
  }

  void _syncConfirmEditors() {
    final items = widget.controller.confirmItems;
    final ids = items.map((e) => e.productId).toSet();

    // Remove stale controllers
    final stale =
        _confirmActualControllers.keys.where((id) => !ids.contains(id)).toList();
    for (final id in stale) {
      _confirmActualControllers[id]?.dispose();
      _confirmActualControllers.remove(id);
    }

    // Add new controllers
    for (final item in items) {
      if (_confirmActualControllers.containsKey(item.productId)) {
        // Sync value if changed externally
        final c = _confirmActualControllers[item.productId]!;
        if (c.text != item.actualStock) c.text = item.actualStock;
        continue;
      }
      final c = TextEditingController(text: item.actualStock);
      c.addListener(() {
        widget.controller.updateConfirmActualStock(
          productId: item.productId,
          actualStock: c.text,
        );
      });
      _confirmActualControllers[item.productId] = c;
    }
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Wizard Stepper
// ════════════════════════════════════════════════════════════════════════════

class _WizardStepper extends StatelessWidget {
  const _WizardStepper({
    required this.currentStep,
    required this.result,
    required this.onStepTap,
    required this.step0Content,
    required this.step1Content,
    required this.step2Content,
  });

  final int currentStep;
  final StockCheckData? result;
  final ValueChanged<int> onStepTap;
  final Widget step0Content;
  final Widget step1Content;
  final Widget step2Content;

  StepState _stateFor(int stepIndex) {
    if (stepIndex < currentStep) return StepState.complete;
    return StepState.indexed;
  }

  @override
  Widget build(BuildContext context) {
    return Stepper(
      currentStep: currentStep,
      controlsBuilder: (_, __) => const SizedBox.shrink(),
      onStepTapped: onStepTap,
      physics: const NeverScrollableScrollPhysics(),
      steps: <Step>[
        Step(
          title: const Text('第一步：选择要盘点的商品'),
          subtitle: const Text('扫码或手工添加商品，然后创建盘点单'),
          state: _stateFor(0),
          isActive: currentStep == 0,
          content: step0Content,
        ),
        Step(
          title: const Text('第二步：开始盘点'),
          subtitle: const Text('确认商品清单后，正式开始清点'),
          state: _stateFor(1),
          isActive: currentStep == 1,
          content: step1Content,
        ),
        Step(
          title: const Text('第三步：录入实盘数量'),
          subtitle: const Text('记录实际清点数量，提交完成盘点'),
          state: _stateFor(2),
          isActive: currentStep == 2,
          content: step2Content,
        ),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Active Check Card — 当前盘点单状态
// ════════════════════════════════════════════════════════════════════════════

class _ActiveCheckCard extends StatelessWidget {
  const _ActiveCheckCard({required this.result});
  final StockCheckData result;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final Color statusColor = switch (result.status) {
      'DRAFT' => const Color(0xFFF59E0B),
      'COUNTING' => const Color(0xFF3B82F6),
      'CONFIRMED' => const Color(0xFF10B981),
      _ => cs.onSurfaceVariant,
    };
    final String statusLabel = switch (result.status) {
      'DRAFT' => '草稿',
      'COUNTING' => '盘点中',
      'CONFIRMED' => '已确认',
      _ => result.status,
    };

    return Container(
      decoration: BoxDecoration(
        color: statusColor.withValues(alpha: 0.06),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: statusColor.withValues(alpha: 0.25)),
      ),
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      child: Row(
        children: <Widget>[
          Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(
              color: statusColor,
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              '盘点单 #${result.id}（${result.bizNo}）',
              style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 14),
            ),
          ),
          Container(
            padding:
                const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
            decoration: BoxDecoration(
              color: statusColor.withValues(alpha: 0.15),
              borderRadius: BorderRadius.circular(999),
            ),
            child: Text(
              statusLabel,
              style: TextStyle(
                color: statusColor,
                fontWeight: FontWeight.w700,
                fontSize: 12,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Confirmed Result Panel — 盘点完成后的差异展示
// ════════════════════════════════════════════════════════════════════════════

class _ConfirmedResultPanel extends StatelessWidget {
  const _ConfirmedResultPanel({required this.result});
  final StockCheckData result;

  @override
  Widget build(BuildContext context) {
    final int normalCount =
        result.items.where((i) => (i.deltaQty ?? 0) == 0).length;
    final int diffCount =
        result.items.where((i) => (i.deltaQty ?? 0) != 0).length;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        // 汇总
        Container(
          padding: const EdgeInsets.all(14),
          decoration: BoxDecoration(
            color: const Color(0xFF10B981).withValues(alpha: 0.07),
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
                color: const Color(0xFF10B981).withValues(alpha: 0.25)),
          ),
          child: Row(
            children: <Widget>[
              const Icon(Icons.check_circle_rounded,
                  color: Color(0xFF10B981), size: 20),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  '盘点完成：${result.items.length} 种商品，'
                  '$normalCount 种无差异，$diffCount 种有差异',
                  style: const TextStyle(
                      fontWeight: FontWeight.w600, fontSize: 13),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 12),

        // 明细列表
        ...result.items.map((item) {
          final int delta = item.deltaQty ?? 0;
          final Color c = delta < 0
              ? const Color(0xFFEF4444) // 亏库
              : delta > 0
                  ? const Color(0xFF3B82F6) // 盈库
                  : const Color(0xFF10B981); // 正常

          return Container(
            margin: const EdgeInsets.only(bottom: 8),
            padding:
                const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            decoration: BoxDecoration(
              color: c.withValues(alpha: 0.05),
              borderRadius: BorderRadius.circular(10),
              border: Border.all(color: c.withValues(alpha: 0.2)),
            ),
            child: Row(
              children: <Widget>[
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        '商品 #${item.productId}',
                        style: const TextStyle(
                            fontWeight: FontWeight.w600, fontSize: 13),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        '账面 ${item.bookStock}  →  实盘 ${item.actualStock ?? '-'}',
                        style: TextStyle(
                            fontSize: 12,
                            color: Theme.of(context)
                                .colorScheme
                                .onSurfaceVariant),
                      ),
                    ],
                  ),
                ),
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                  decoration: BoxDecoration(
                    color: c.withValues(alpha: 0.14),
                    borderRadius: BorderRadius.circular(999),
                  ),
                  child: Text(
                    delta == 0
                        ? '无差异'
                        : delta > 0
                            ? '+$delta'
                            : '$delta',
                    style: TextStyle(
                      color: c,
                      fontWeight: FontWeight.w800,
                      fontSize: 12,
                    ),
                  ),
                ),
              ],
            ),
          );
        }),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Scan Bar
// ════════════════════════════════════════════════════════════════════════════

class _ScanBar extends StatelessWidget {
  const _ScanBar({
    required this.controller,
    required this.loading,
    required this.canAct,
    required this.onScan,
    required this.onCamera,
    required this.onClear,
    required this.scanError,
    required this.scanSuccess,
  });

  final TextEditingController controller;
  final bool loading;
  final bool canAct;
  final VoidCallback onScan;
  final VoidCallback onCamera;
  final VoidCallback onClear;
  final String? scanError;
  final String? scanSuccess;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(
              child: TextField(
                controller: controller,
                decoration: InputDecoration(
                  labelText: '扫码添加商品',
                  hintText: '条码号，例如：6901234567890',
                  border: const OutlineInputBorder(),
                  isDense: true,
                  suffixIcon: IconButton(
                    tooltip: '摄像头扫码',
                    icon: const Icon(Icons.qr_code_scanner),
                    onPressed: canAct ? onCamera : null,
                  ),
                ),
              ),
            ),
            const SizedBox(width: 8),
            FilledButton(
              onPressed: canAct ? onScan : null,
              child: loading
                  ? const _SmallSpinner()
                  : const Icon(Icons.add, size: 20),
            ),
            const SizedBox(width: 4),
            IconButton(
              tooltip: '清空',
              onPressed: onClear,
              icon: const Icon(Icons.clear_rounded, size: 18),
            ),
          ],
        ),
        if (scanError != null)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: StatusNotice(message: scanError!, tone: NoticeTone.error),
          ),
        if (scanSuccess != null)
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child:
                StatusNotice(message: scanSuccess!, tone: NoticeTone.success),
          ),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Item Input Row — 创建明细行
// ════════════════════════════════════════════════════════════════════════════

class _ItemInputRow extends StatelessWidget {
  const _ItemInputRow({
    required this.editor,
    required this.canRemove,
    required this.onRemove,
  });

  final _CreateItemEditor editor;
  final bool canRemove;
  final VoidCallback onRemove;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: <Widget>[
        Expanded(
          child: TextField(
            controller: editor.productIdController,
            keyboardType: TextInputType.number,
            decoration: InputDecoration(
              labelText: '商品ID #${editor.localId}',
              hintText: '例如：101',
              border: const OutlineInputBorder(),
              isDense: true,
            ),
          ),
        ),
        if (canRemove) ...<Widget>[
          const SizedBox(width: 6),
          IconButton(
            tooltip: '移除',
            icon: const Icon(Icons.remove_circle_outline,
                size: 20, color: Colors.redAccent),
            onPressed: onRemove,
          ),
        ],
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Confirm Item Row — 录入实盘行
// ════════════════════════════════════════════════════════════════════════════

class _ConfirmItemRow extends StatelessWidget {
  const _ConfirmItemRow({
    required this.item,
    required this.controller,
    required this.enabled,
  });

  final StockCheckConfirmItemInput item;
  final TextEditingController controller;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    return Container(
      padding: const EdgeInsets.fromLTRB(14, 10, 14, 10),
      decoration: BoxDecoration(
        color: cs.surfaceContainerHighest.withValues(alpha: 0.4),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: cs.outlineVariant),
      ),
      child: Row(
        children: <Widget>[
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  '商品 #${item.productId}',
                  style: const TextStyle(
                      fontWeight: FontWeight.w600, fontSize: 13),
                ),
                Text(
                  '账面库存：${item.bookStock} 件',
                  style: TextStyle(
                      fontSize: 12, color: cs.onSurfaceVariant),
                ),
              ],
            ),
          ),
          const SizedBox(width: 12),
          SizedBox(
            width: 100,
            child: TextField(
              controller: controller,
              enabled: enabled,
              keyboardType: TextInputType.number,
              textAlign: TextAlign.center,
              decoration: const InputDecoration(
                labelText: '实盘数量',
                border: OutlineInputBorder(),
                isDense: true,
                contentPadding:
                    EdgeInsets.symmetric(horizontal: 8, vertical: 10),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Info Row
// ════════════════════════════════════════════════════════════════════════════

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.label, required this.value});
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 6),
      child: Row(
        children: <Widget>[
          Text(label,
              style: const TextStyle(color: Colors.grey, fontSize: 13)),
          const SizedBox(width: 8),
          Expanded(
            child: Text(value,
                style: const TextStyle(
                    fontWeight: FontWeight.w600, fontSize: 13)),
          ),
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Small Spinner
// ════════════════════════════════════════════════════════════════════════════

class _SmallSpinner extends StatelessWidget {
  const _SmallSpinner();

  @override
  Widget build(BuildContext context) {
    return const SizedBox(
      width: 16,
      height: 16,
      child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Create Item Editor (data class)
// ════════════════════════════════════════════════════════════════════════════

class _CreateItemEditor {
  _CreateItemEditor({
    required this.localId,
    required this.productIdController,
  });

  final int localId;
  final TextEditingController productIdController;

  void dispose() => productIdController.dispose();
}
