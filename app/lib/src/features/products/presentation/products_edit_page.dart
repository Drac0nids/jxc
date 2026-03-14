import 'package:flutter/material.dart';

import '../../../core/widgets/barcode_scanner_sheet.dart';
import '../../../core/widgets/brand_ui.dart';
import '../application/category_controller.dart';
import '../application/product_controller.dart';
import '../models/product_models.dart';
import 'category_picker_sheet.dart';

class ProductsEditPage extends StatefulWidget {
  const ProductsEditPage({
    super.key,
    required this.controller,
    required this.product,
    required this.moneyPattern,
    this.categoryController,
  });

  final ProductController controller;
  final ProductData product;
  final RegExp moneyPattern;
  final CategoryController? categoryController;

  @override
  State<ProductsEditPage> createState() => _ProductsEditPageState();
}

class _ProductsEditPageState extends State<ProductsEditPage> {
  late final TextEditingController _skuController;
  late final TextEditingController _barcodeController;
  late final TextEditingController _nameController;
  late final TextEditingController _unitController;
  late final TextEditingController _retailPriceController;
  late final TextEditingController _minStockLimitController;
  late final TextEditingController _expectedVersionController;

  int? _selectedCategoryId;
  late bool _trackBatches;
  String? _localError;

  @override
  void initState() {
    super.initState();
    _skuController = TextEditingController(text: widget.product.sku);
    _barcodeController = TextEditingController(text: widget.product.barcode);
    _nameController = TextEditingController(text: widget.product.name);
    _unitController = TextEditingController(text: widget.product.unit);
    _retailPriceController =
        TextEditingController(text: widget.product.retailPrice);
    _minStockLimitController =
        TextEditingController(text: '${widget.product.minStockLimit}');
    _expectedVersionController =
        TextEditingController(text: '${widget.product.version}');
    _selectedCategoryId = widget.product.categoryId;
    _trackBatches = widget.product.trackBatches;
  }

  @override
  void dispose() {
    _skuController.dispose();
    _barcodeController.dispose();
    _nameController.dispose();
    _unitController.dispose();
    _retailPriceController.dispose();
    _minStockLimitController.dispose();
    _expectedVersionController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.controller,
      builder: (BuildContext context, Widget? child) {
        final bool submitting = widget.controller.submitting;

        return Scaffold(
          appBar: AppBar(
            title: const Text('编辑商品'),
          ),
          body: ListView(
            padding: const EdgeInsets.all(16),
            children: <Widget>[
              SectionCard(
                title: '当前商品状态',
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text('商品ID：${widget.product.id}',
                        style:
                            const TextStyle(fontSize: 13, color: Colors.grey)),
                    const SizedBox(height: 4),
                    Text('当前库存：${widget.product.currentStock}',
                        style:
                            const TextStyle(fontSize: 13, color: Colors.grey)),
                    const SizedBox(height: 4),
                    Text('当前版本：${widget.product.version}',
                        style:
                            const TextStyle(fontSize: 13, color: Colors.grey)),
                  ],
                ),
              ),
              const SizedBox(height: 12),
              _buildTextField(
                controller: _skuController,
                label: 'SKU *',
                hint: '例如：KO-330',
                enabled: !submitting,
              ),
              const SizedBox(height: 8),
              _buildTextField(
                controller: _barcodeController,
                label: '条码 *',
                hint: '支持手工/扫码枪/手机摄像头',
                enabled: !submitting,
                suffixIcon: IconButton(
                  tooltip: '扫码填入条码',
                  onPressed: submitting ? null : _fillBarcodeByCamera,
                  icon: const Icon(Icons.qr_code_scanner),
                ),
              ),
              const SizedBox(height: 8),
              _buildTextField(
                controller: _nameController,
                label: '商品名称 *',
                hint: '例如：可口可乐 330ml（新包装）',
                enabled: !submitting,
              ),
              const SizedBox(height: 8),
              _buildTextField(
                controller: _unitController,
                label: '单位 *',
                hint: '例如：罐',
                enabled: !submitting,
              ),
              const SizedBox(height: 8),
              Row(
                children: <Widget>[
                  Expanded(
                    child: _buildTextField(
                      controller: _retailPriceController,
                      label: '零售价 *',
                      hint: '3.80',
                      enabled: !submitting,
                      keyboardType:
                          const TextInputType.numberWithOptions(decimal: true),
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 8),
              Row(
                children: <Widget>[
                  Expanded(
                    child: _buildTextField(
                      controller: _minStockLimitController,
                      label: '预警阈值 *',
                      hint: '大于等于 0 的整数',
                      enabled: !submitting,
                      keyboardType: TextInputType.number,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: _buildTextField(
                      controller: _expectedVersionController,
                      label: 'expected_version *',
                      hint: '建议保持最新版本号',
                      enabled: !submitting,
                      keyboardType: TextInputType.number,
                    ),
                  ),
                ],
              ),
              // 分类选择（可选，有 categoryController 时展示）
              if (widget.categoryController != null) ...<Widget>[
                const SizedBox(height: 8),
                _buildCategoryTile(context),
              ],
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
              if (_localError != null) ...<Widget>[
                const SizedBox(height: 10),
                Text(
                  _localError!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
              const SizedBox(height: 12),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: <Widget>[
                  FilledButton.icon(
                    onPressed: submitting ? null : _submitUpdate,
                    icon: const Icon(Icons.save_outlined),
                    label: Text(submitting ? '保存中...' : '保存修改'),
                  ),
                  OutlinedButton.icon(
                    onPressed: submitting ? null : _resetToInitial,
                    icon: const Icon(Icons.restart_alt),
                    label: const Text('还原初始值'),
                  ),
                  TextButton.icon(
                    onPressed: submitting ? null : _deleteCurrentProduct,
                    icon: const Icon(Icons.delete_outline),
                    label: const Text('删除商品'),
                  ),
                ],
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _buildCategoryTile(BuildContext context) {
    final catCtrl = widget.categoryController!;
    final String? path = catCtrl.buildPath(_selectedCategoryId);
    final colorScheme = Theme.of(context).colorScheme;

    return InkWell(
      borderRadius: BorderRadius.circular(8),
      onTap: () async {
        // 确保分类已加载
        if (catCtrl.tree.isEmpty && !catCtrl.loading) {
          await catCtrl.load();
        }
        if (!mounted) return;
        // ignore: use_build_context_synchronously
        final int? picked = await CategoryPickerSheet.show(
          context,
          tree: catCtrl.tree,
          initialId: _selectedCategoryId,
        );
        if (!mounted) return;
        // picked==null 表示用户按 "清除分类"
        // 若用户直接关闭底部弹窗则保持不变
        setState(() => _selectedCategoryId = picked);
      },
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 14),
        decoration: BoxDecoration(
          border: Border.all(color: Colors.grey.shade400),
          borderRadius: BorderRadius.circular(8),
        ),
        child: Row(
          children: <Widget>[
            Icon(Icons.category_outlined,
                size: 20, color: colorScheme.primary),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    '商品分类',
                    style: TextStyle(
                        fontSize: 12, color: Colors.grey.shade600),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    path ?? '未选择分类（可选）',
                    style: TextStyle(
                      fontSize: 14,
                      color: path != null ? null : Colors.grey.shade400,
                    ),
                  ),
                ],
              ),
            ),
            Icon(Icons.chevron_right, color: Colors.grey.shade500),
          ],
        ),
      ),
    );
  }

  Widget _buildTextField({
    required TextEditingController controller,
    required String label,
    required String hint,
    required bool enabled,
    TextInputType? keyboardType,
    Widget? suffixIcon,
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
        suffixIcon: suffixIcon,
      ),
    );
  }


  void _resetToInitial() {
    _skuController.text = widget.product.sku;
    _barcodeController.text = widget.product.barcode;
    _nameController.text = widget.product.name;
    _unitController.text = widget.product.unit;
    _retailPriceController.text = widget.product.retailPrice;
    _minStockLimitController.text = '${widget.product.minStockLimit}';
    _expectedVersionController.text = '${widget.product.version}';
    setState(() {
      _selectedCategoryId = widget.product.categoryId;
      _trackBatches = widget.product.trackBatches;
      _localError = null;
    });
  }

  Future<void> _fillBarcodeByCamera() async {
    FocusScope.of(context).unfocus();
    final String? barcode = await BarcodeScannerSheet.scan(
      context,
      title: '编辑商品扫码',
      hint: '识别成功后会自动回填到编辑表单条码字段。',
    );

    if (!mounted || barcode == null || barcode.isEmpty) {
      return;
    }

    _barcodeController.text = barcode;
  }

  Future<void> _submitUpdate() async {
    final String? validationError = _validate();
    if (validationError != null) {
      setState(() {
        _localError = validationError;
      });
      return;
    }

    setState(() {
      _localError = null;
    });

    final ProductData? updated = await widget.controller.updateProduct(
      id: widget.product.id,
      request: UpdateProductRequest(
        sku: _skuController.text.trim(),
        barcode: _barcodeController.text.trim(),
        name: _nameController.text.trim(),
        unit: _unitController.text.trim(),
        retailPrice: _retailPriceController.text.trim(),
        minStockLimit: int.tryParse(_minStockLimitController.text.trim()),
        expectedVersion: int.tryParse(_expectedVersionController.text.trim()),
        categoryId: _selectedCategoryId,
        trackBatches: _trackBatches,
      ),
    );

    if (updated != null && mounted) {
      Navigator.of(context).pop(true);
    }
  }

  Future<void> _deleteCurrentProduct() async {
    final bool confirmed = await showDialog<bool>(
          context: context,
          builder: (BuildContext context) {
            return AlertDialog(
              title: const Text('确认删除商品'),
              content:
                  Text('确定删除 #${widget.product.id} ${widget.product.name} 吗？'),
              actions: <Widget>[
                TextButton(
                  onPressed: () => Navigator.of(context).pop(false),
                  child: const Text('取消'),
                ),
                FilledButton(
                  onPressed: () => Navigator.of(context).pop(true),
                  child: const Text('删除'),
                ),
              ],
            );
          },
        ) ??
        false;

    if (!confirmed) {
      return;
    }

    final int? expectedVersion =
        int.tryParse(_expectedVersionController.text.trim());
    final DeleteProductResult? deleted = await widget.controller.deleteProduct(
      id: widget.product.id,
      expectedVersion: expectedVersion ?? widget.product.version,
    );

    if (deleted != null && mounted) {
      Navigator.of(context).pop(true);
    }
  }

  String? _validate() {
    if (_skuController.text.trim().isEmpty) {
      return 'SKU 不能为空';
    }
    if (_barcodeController.text.trim().isEmpty) {
      return '条码不能为空';
    }
    if (_nameController.text.trim().isEmpty) {
      return '商品名称不能为空';
    }
    if (_unitController.text.trim().isEmpty) {
      return '单位不能为空';
    }

    if (!widget.moneyPattern.hasMatch(_retailPriceController.text.trim())) {
      return '零售价格式错误（示例：3.80）';
    }

    final int? minStockLimit =
        int.tryParse(_minStockLimitController.text.trim());
    if (minStockLimit == null || minStockLimit < 0) {
      return '预警阈值必须为大于等于 0 的整数';
    }

    final int? expectedVersion =
        int.tryParse(_expectedVersionController.text.trim());
    if (expectedVersion == null || expectedVersion < 0) {
      return 'expected_version 必须为大于等于 0 的整数';
    }

    return null;
  }
}
