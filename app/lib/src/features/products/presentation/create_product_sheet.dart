import 'package:flutter/material.dart';

import '../../../core/widgets/barcode_scanner_sheet.dart';
import '../../../core/widgets/brand_ui.dart';
import '../application/category_controller.dart';
import '../application/product_controller.dart';
import '../models/category_models.dart';
import '../models/product_models.dart';
import 'category_picker_sheet.dart';

// ════════════════════════════════════════════════════════════════════════════
// CreateProductSheet
// 新建商品底部弹窗，支持分类选择（categoryController 可选）
// ════════════════════════════════════════════════════════════════════════════

class CreateProductSheet extends StatefulWidget {
  const CreateProductSheet({
    super.key,
    required this.controller,
    required this.moneyPattern,
    this.presetBarcode,
    this.categoryController,
  });

  final ProductController controller;
  final RegExp moneyPattern;
  final String? presetBarcode;
  final CategoryController? categoryController;

  @override
  State<CreateProductSheet> createState() => _CreateProductSheetState();
}

class _CreateProductSheetState extends State<CreateProductSheet> {
  final TextEditingController _skuController = TextEditingController();
  final TextEditingController _barcodeController = TextEditingController();
  final TextEditingController _nameController = TextEditingController();
  final TextEditingController _unitController = TextEditingController();
  final TextEditingController _retailPriceController = TextEditingController();
  final TextEditingController _initStockController =
      TextEditingController(text: '0');
  final TextEditingController _minStockLimitController =
      TextEditingController(text: '0');
  final TextEditingController _costPriceController = TextEditingController();

  /// 当前选中的分类叶子节点
  CategoryNode? _selectedCategory;

  bool _trackBatches = false;

  String? _localError;

  @override
  void initState() {
    super.initState();
    final String preset = widget.presetBarcode?.trim() ?? '';
    if (preset.isNotEmpty) _barcodeController.text = preset;
  }

  @override
  void dispose() {
    _skuController.dispose();
    _barcodeController.dispose();
    _nameController.dispose();
    _unitController.dispose();
    _retailPriceController.dispose();
    _initStockController.dispose();
    _minStockLimitController.dispose();
    _costPriceController.dispose();
    super.dispose();
  }

  // ── Category picker ──────────────────────────────────────────────────────

  Future<void> _pickCategory() async {
    final cc = widget.categoryController;
    if (cc == null) return;

    // 确保分类已加载
    if (cc.isEmpty) await cc.load();

    if (!mounted) return;
    // ignore: use_build_context_synchronously
    final int? pickedId = await CategoryPickerSheet.show(
      context,
      tree: cc.tree,
      initialId: _selectedCategory?.id,
    );
    if (pickedId == null) {
      setState(() => _selectedCategory = null);
    } else {
      final CategoryNode? node = cc.findById(pickedId);
      if (node != null) setState(() => _selectedCategory = node);
    }
  }

  // ── Barcode camera ───────────────────────────────────────────────────────

  Future<void> _fillBarcodeByCamera() async {
    FocusScope.of(context).unfocus();
    final String? barcode = await BarcodeScannerSheet.scan(
      context,
      title: '新建商品扫码',
      hint: '识别成功后会自动回填到条码输入框。',
    );
    if (!mounted || barcode == null || barcode.isEmpty) return;
    _barcodeController.text = barcode;
  }

  // ── Submit ───────────────────────────────────────────────────────────────

  Future<void> _submit() async {
    final bool submitting = widget.controller.submitting;
    if (submitting) return;

    final String? err = _validateForm();
    if (err != null) {
      setState(() => _localError = err);
      return;
    }
    setState(() => _localError = null);

    final ProductData? created = await widget.controller.createProduct(
      CreateProductRequest(
        sku: _skuController.text.trim().isEmpty
            ? null
            : _skuController.text.trim(),
        barcode: _barcodeController.text.trim(),
        name: _nameController.text.trim(),
        unit: _unitController.text.trim(),
        retailPrice: _retailPriceController.text.trim(),
        initStock: _parseInt(_initStockController.text),
        minStockLimit: _parseInt(_minStockLimitController.text),
        costPrice: _costPriceController.text.trim(),
        categoryId: _selectedCategory?.id,
        trackBatches: _trackBatches,
      ),
    );

    if (created != null && mounted) Navigator.of(context).pop(created);
  }

  String? _validateForm() {
    if (_barcodeController.text.trim().isEmpty) return '条码不能为空';
    if (_nameController.text.trim().isEmpty) return '商品名称不能为空';
    if (_unitController.text.trim().isEmpty) return '单位不能为空';
    if (!widget.moneyPattern.hasMatch(_retailPriceController.text.trim())) {
      return '零售价格式错误（示例：3.50）';
    }
    final String initStockRaw = _initStockController.text.trim();
    if (initStockRaw.isNotEmpty) {
      final int? v = _parseInt(initStockRaw);
      if (v == null || v < 0) return '初始库存必须为大于等于 0 的整数';
    }
    final String minStockRaw = _minStockLimitController.text.trim();
    if (minStockRaw.isNotEmpty) {
      final int? v = _parseInt(minStockRaw);
      if (v == null || v < 0) return '预警阈值必须为大于等于 0 的整数';
    }
    final String costRaw = _costPriceController.text.trim();
    if (costRaw.isEmpty) return '进货价格不能为空';
    if (!widget.moneyPattern.hasMatch(costRaw)) {
      return '进货价格式错误（示例：2.10）';
    }
    return null;
  }

  int? _parseInt(String raw) {
    final String v = raw.trim();
    if (v.isEmpty) return null;
    return int.tryParse(v);
  }

  // ── Build ────────────────────────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.controller,
      builder: (BuildContext context, Widget? child) {
        final bool submitting = widget.controller.submitting;

        return Padding(
          padding: EdgeInsets.only(
            left: 16,
            right: 16,
            top: 8,
            bottom: MediaQuery.of(context).viewInsets.bottom + 16,
          ),
          child: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                // Handle bar
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
                const SizedBox(height: 14),
                const Text(
                  '新建商品',
                  style: TextStyle(fontSize: 18, fontWeight: FontWeight.w800),
                ),
                const SizedBox(height: 16),

                // ── Section 1: 基本信息 ──────────────────────────────────
                const SectionLabel(label: '基本信息'),
                const SizedBox(height: 8),
                _buildField(
                  controller: _skuController,
                  label: 'SKU（可选）',
                  hint: '留空由服务端自动生成',
                  enabled: !submitting,
                ),
                const SizedBox(height: 8),
                _buildField(
                  controller: _barcodeController,
                  label: '条码 *',
                  hint: '支持手工/扫码枪/摄像头',
                  enabled: !submitting,
                  suffix: IconButton(
                    tooltip: '扫码填入条码',
                    onPressed: submitting ? null : _fillBarcodeByCamera,
                    icon: const Icon(Icons.qr_code_scanner),
                  ),
                ),
                const SizedBox(height: 8),
                _buildField(
                  controller: _nameController,
                  label: '商品名称 *',
                  hint: '例如：百事可乐 550ml',
                  enabled: !submitting,
                ),
                const SizedBox(height: 8),
                _buildField(
                  controller: _unitController,
                  label: '单位 *',
                  hint: '例如：瓶、盒、个',
                  enabled: !submitting,
                ),

                // ── 分类选择 ──────────────────────────────────────────────
                if (widget.categoryController != null) ...<Widget>[
                  const SizedBox(height: 8),
                  _CategoryTile(
                    selectedCategory: _selectedCategory,
                    categoryController: widget.categoryController!,
                    onTap: submitting ? null : _pickCategory,
                    onClear: submitting
                        ? null
                        : () => setState(() => _selectedCategory = null),
                  ),
                ],

                const SizedBox(height: 16),

                // ── Section 2: 价格设定 ──────────────────────────────────
                const SectionLabel(label: '价格设定'),
                const SizedBox(height: 8),
                Row(
                  children: <Widget>[
                    Expanded(
                      child: _buildField(
                        controller: _retailPriceController,
                        label: '零售价 *',
                        hint: '3.50',
                        enabled: !submitting,
                        keyboardType: const TextInputType.numberWithOptions(
                            decimal: true),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: _buildField(
                        controller: _costPriceController,
                        label: '进货价格 *',
                        hint: '2.10',
                        enabled: !submitting,
                        keyboardType: const TextInputType.numberWithOptions(
                            decimal: true),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 16),

                // ── Section 3: 库存设置 ──────────────────────────────────
                const SectionLabel(label: '库存设置'),
                const SizedBox(height: 8),
                Row(
                  children: <Widget>[
                    Expanded(
                      child: _buildField(
                        controller: _initStockController,
                        label: '初始库存',
                        hint: '默认 0',
                        enabled: !submitting,
                        keyboardType: TextInputType.number,
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: _buildField(
                        controller: _minStockLimitController,
                        label: '低库存预警',
                        hint: '默认 0（不预警）',
                        enabled: !submitting,
                        keyboardType: TextInputType.number,
                      ),
                    ),
                  ],
                ),

                // ── Section 4: 批次追踪 ──────────────────────────────────
                const SizedBox(height: 8),
                SwitchListTile(
                  value: _trackBatches,
                  onChanged: submitting
                      ? null
                      : (bool v) => setState(() => _trackBatches = v),
                  title: const Text(
                    '启用批次追踪',
                    style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600),
                  ),
                  subtitle: const Text(
                    '开启后，每次采购入库时会提示关联批次（适用于有保质期的商品）',
                    style: TextStyle(fontSize: 12),
                  ),
                  contentPadding: EdgeInsets.zero,
                  dense: true,
                ),

                // Error
                if (_localError != null) ...<Widget>[
                  const SizedBox(height: 12),
                  _ErrorBanner(message: _localError!),
                ],

                const SizedBox(height: 16),
                Row(
                  children: <Widget>[
                    Expanded(
                      child: FilledButton(
                        onPressed: submitting ? null : _submit,
                        child: Text(submitting ? '创建中…' : '创建商品'),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: OutlinedButton(
                        onPressed:
                            submitting ? null : () => Navigator.of(context).pop(),
                        child: const Text('取消'),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  Widget _buildField({
    required TextEditingController controller,
    required String label,
    required String hint,
    required bool enabled,
    TextInputType? keyboardType,
    Widget? suffix,
  }) {
    return TextField(
      controller: controller,
      enabled: enabled,
      keyboardType: keyboardType,
      decoration: InputDecoration(
        labelText: label,
        hintText: hint,
        border: const OutlineInputBorder(),
        isDense: true,
        suffixIcon: suffix,
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _CategoryTile — 分类选择行
// ════════════════════════════════════════════════════════════════════════════

class _CategoryTile extends StatelessWidget {
  const _CategoryTile({
    required this.selectedCategory,
    required this.categoryController,
    required this.onTap,
    required this.onClear,
  });

  final CategoryNode? selectedCategory;
  final CategoryController categoryController;
  final VoidCallback? onTap;
  final VoidCallback? onClear;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    final String path = selectedCategory == null
        ? '点击选择分类（可选）'
        : (categoryController.buildPath(selectedCategory!.id) ??
            selectedCategory!.name);
    final bool hasValue = selectedCategory != null;

    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: InputDecorator(
        decoration: InputDecoration(
          labelText: '商品分类',
          border: const OutlineInputBorder(),
          isDense: true,
          prefixIcon: const Icon(Icons.category_outlined, size: 18),
          suffixIcon: hasValue
              ? IconButton(
                  icon: const Icon(Icons.close, size: 18),
                  onPressed: onClear,
                  tooltip: '清除分类',
                )
              : const Icon(Icons.chevron_right, size: 18),
        ),
        child: Text(
          path,
          style: TextStyle(
            fontSize: 14,
            color: hasValue ? cs.onSurface : cs.onSurfaceVariant,
          ),
          overflow: TextOverflow.ellipsis,
        ),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _ErrorBanner — 错误提示条
// ════════════════════════════════════════════════════════════════════════════

class _ErrorBanner extends StatelessWidget {
  const _ErrorBanner({required this.message});
  final String message;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: cs.errorContainer,
        borderRadius: BorderRadius.circular(8),
      ),
      child: Row(
        children: <Widget>[
          Icon(Icons.error_outline, size: 16, color: cs.onErrorContainer),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              message,
              style: TextStyle(color: cs.onErrorContainer, fontSize: 13),
            ),
          ),
        ],
      ),
    );
  }
}
