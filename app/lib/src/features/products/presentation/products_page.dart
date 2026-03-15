import 'dart:async';

import 'package:flutter/material.dart';

import '../../../core/widgets/barcode_scanner_sheet.dart';
import '../../../core/widgets/brand_ui.dart';
import '../application/batch_controller.dart';
import '../application/category_controller.dart';
import '../application/product_controller.dart';

import '../models/category_models.dart';
import '../models/product_models.dart';
import 'batch_management_page.dart';
import 'category_management_page.dart';
import 'create_product_sheet.dart';
import 'products_edit_page.dart';


// 顶层常量，避免重复创建 RegExp
const _kMoneyPattern = r'^\d+(\.\d{1,4})?$';
final RegExp _moneyPattern = RegExp(_kMoneyPattern);

// ════════════════════════════════════════════════════════════════════════════
// ProductsPage
// ════════════════════════════════════════════════════════════════════════════

class ProductsPage extends StatefulWidget {
  const ProductsPage({
    super.key,
    required this.controller,
    this.onStockCheck,
    this.categoryController,
    this.batchController,
  });

  final ProductController controller;

  /// 可选，传入则商品卡片显示「发起盘点」按钮
  final void Function(int productId)? onStockCheck;

  /// 可选，传入则启用分类管理和分类筛选
  final CategoryController? categoryController;

  /// 可选，传入则商品卡片显示「批次管理」入口
  final BatchController? batchController;



  @override
  State<ProductsPage> createState() => _ProductsPageState();
}

class _ProductsPageState extends State<ProductsPage> {
  final TextEditingController _keywordController = TextEditingController();
  final TextEditingController _barcodeController = TextEditingController();

  bool _filterExpanded = true;
  Timer? _debounce;

  /// 当前筛选用的分类节点
  CategoryNode? _filterCategory;

  @override
  void initState() {
    super.initState();
    _keywordController.text = widget.controller.keyword;
    _barcodeController.text = widget.controller.barcode;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadProducts(page: 1);
      // 预加载分类树，确保卡片中显示分类名称
      widget.categoryController?.load();
    });
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _keywordController.dispose();
    _barcodeController.dispose();
    super.dispose();
  }

  // ── Load ─────────────────────────────────────────────────────────────────

  Future<void> _loadProducts({required int page}) async {
    await widget.controller.loadProducts(
      page: page,
      pageSize: widget.controller.pageSize,
      keyword: _keywordController.text.trim(),
      barcode: _barcodeController.text.trim(),
      categoryId: _filterCategory?.id,
    );
  }

  void _onKeywordChanged(String _) {
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 420), () {
      if (mounted) _loadProducts(page: 1);
    });
  }

  void _resetFilters() {
    _keywordController.clear();
    _barcodeController.clear();
    setState(() => _filterCategory = null);
    widget.controller.clearMessages();
    _loadProducts(page: 1);
  }

  Future<void> _fillFilterBarcodeByCamera() async {
    if (widget.controller.loading || widget.controller.submitting) return;
    FocusScope.of(context).unfocus();
    final String? barcode = await BarcodeScannerSheet.scan(
      context,
      title: '商品筛选扫码',
      hint: '识别成功后自动回填条码并执行筛选查询。',
    );
    if (!mounted || barcode == null || barcode.isEmpty) return;
    _barcodeController.text = barcode;
    await _loadProducts(page: 1);
  }

  // ── Edit / Create / Delete ────────────────────────────────────────────────

  Future<void> _showCreateProductSheet() async {
    final ProductData? created = await showModalBottomSheet<ProductData>(
      context: context,
      useSafeArea: true,
      isScrollControlled: true,
      builder: (BuildContext context) => CreateProductSheet(
        controller: widget.controller,
        moneyPattern: _moneyPattern,
        categoryController: widget.categoryController,
      ),
    );
    if (created != null) await _loadProducts(page: 1);
  }

  Future<void> _openEditPage(ProductData product) async {
    final bool? changed = await Navigator.of(context).push<bool>(
      MaterialPageRoute<bool>(
        builder: (BuildContext context) => ProductsEditPage(
          controller: widget.controller,
          product: product,
          moneyPattern: _moneyPattern,
          categoryController: widget.categoryController,
        ),
      ),
    );
    if (changed == true) await _loadProducts(page: widget.controller.page);
  }

  Future<void> _deleteProduct(ProductData product) async {
    final bool confirmed = await showDialog<bool>(
          context: context,
          builder: (BuildContext context) => AlertDialog(
            title: const Text('确认删除商品'),
            content: Text('即将永久删除「${product.name}」，此操作不可撤销。'),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(false),
                child: const Text('取消'),
              ),
              FilledButton(
                style: FilledButton.styleFrom(
                  backgroundColor: Theme.of(context).colorScheme.error,
                  foregroundColor: Theme.of(context).colorScheme.onError,
                ),
                onPressed: () => Navigator.of(context).pop(true),
                child: const Text('确认删除'),
              ),
            ],
          ),
        ) ??
        false;

    if (!confirmed) return;

    final DeleteProductResult? result = await widget.controller.deleteProduct(
      id: product.id,
      expectedVersion: product.version,
    );

    if (result != null && mounted) {
      final bool needFallback =
          widget.controller.list.length == 1 && widget.controller.page > 1;
      await _loadProducts(
          page: needFallback
              ? widget.controller.page - 1
              : widget.controller.page);
    }
  }

  // ── Category filter picker ────────────────────────────────────────────────

  Future<void> _pickFilterCategory() async {
    final cc = widget.categoryController;
    if (cc == null) return;
    if (cc.isEmpty) await cc.load();
    if (!mounted) return;

    // ignore: use_build_context_synchronously
    final CategoryNode? picked = await _CategoryFilterSheet.show(context, cc);
    if (!mounted) return;
    setState(() => _filterCategory = picked);
    await _loadProducts(page: 1);
  }

  // ── Build ─────────────────────────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.controller,
      builder: (BuildContext context, Widget? child) {
        final bool canWrite = widget.controller.canWrite;
        final bool busy =
            widget.controller.loading || widget.controller.submitting;
        final int total = widget.controller.total;
        final int page = widget.controller.page;
        final int pageSize = widget.controller.pageSize;
        final int totalPages =
            total == 0 ? 1 : ((total + pageSize - 1) / pageSize).floor();

        return Scaffold(
          appBar: AppBar(
            title: const Text('商品管理'),
            actions: <Widget>[
              // 搜索栏 开/关（图标切换）
              IconButton(
                tooltip: _filterExpanded ? '收起搜索' : '展开搜索',
                onPressed: () =>
                    setState(() => _filterExpanded = !_filterExpanded),
                icon: Icon(
                  _filterExpanded
                      ? Icons.search_off_rounded
                      : Icons.search_rounded,
                  color: _filterExpanded
                      ? Theme.of(context).colorScheme.primary
                      : null,
                ),
              ),
              // 新建商品（仅有写权限时显示）
              if (canWrite)
                IconButton(
                  tooltip: '新建商品',
                  onPressed: busy ? null : _showCreateProductSheet,
                  icon: const Icon(Icons.add_rounded),
                ),
              const SizedBox(width: 4),
            ],
          ),
          body: RefreshIndicator(
            onRefresh: () => _loadProducts(page: page),
            child: ListView(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 24),
              children: <Widget>[
                // ── 页头：紧凑型 ────────────────────────────────────────
                _PageHeader(
                  total: total,
                  page: page,
                  totalPages: totalPages,
                  filterCategory: _filterCategory,
                  categoryController: widget.categoryController,
                ),
                const SizedBox(height: 10),

                // ── 角色提示 ────────────────────────────────────────────
                if (!canWrite)
                  const Padding(
                    padding: EdgeInsets.only(bottom: 8),
                    child: StatusNotice(
                      message: '当前角色为只读，可浏览商品列表与详情，不可创建/编辑/删除。',
                      tone: NoticeTone.warning,
                    ),
                  ),

                // ── 错误 / 成功提示 ─────────────────────────────────────
                if (widget.controller.errorMessage != null)
                  StatusNotice(
                    message: widget.controller.errorMessage!,
                    tone: NoticeTone.error,
                  ),
                if (widget.controller.successMessage != null)
                  StatusNotice(
                    message: widget.controller.successMessage!,
                    tone: NoticeTone.success,
                  ),

                // ── 筛选区（可折叠）────────────────────────────────────
                AnimatedCrossFade(
                  duration: const Duration(milliseconds: 220),
                  crossFadeState: _filterExpanded
                      ? CrossFadeState.showFirst
                      : CrossFadeState.showSecond,
                  firstChild: _FilterCard(
                    keywordController: _keywordController,
                    barcodeController: _barcodeController,
                    busy: busy,
                    filterCategory: _filterCategory,
                    categoryController: widget.categoryController,
                    onKeywordChanged: _onKeywordChanged,
                    onSearch: () => _loadProducts(page: 1),
                    onReset: _resetFilters,
                    onScanBarcode: _fillFilterBarcodeByCamera,
                    onPickCategory: _pickFilterCategory,
                    onClearCategory: () {
                      setState(() => _filterCategory = null);
                      _loadProducts(page: 1);
                    },
                  ),
                  secondChild: const SizedBox.shrink(),
                ),
                const SizedBox(height: 10),

                // ── 商品列表 ────────────────────────────────────────────
                if (busy && widget.controller.list.isEmpty)
                  ..._buildSkeletons(4)
                else if (widget.controller.list.isEmpty)
                  _EmptyState(
                    hasFilter: _keywordController.text.isNotEmpty ||
                        _barcodeController.text.isNotEmpty ||
                        _filterCategory != null,
                    onClearFilter: _resetFilters,
                  )
                else
                  ...widget.controller.list.map(
                    (ProductData p) => Padding(
                      padding: const EdgeInsets.only(bottom: 10),
                      child: _ProductCard(
                        product: p,
                        canViewCostPrice: widget.controller.canViewCostPrice,
                        canWrite: canWrite,
                        busy: busy,
                        categoryController: widget.categoryController,
                        onEdit: () => _openEditPage(p),
                        onDelete: () => _deleteProduct(p),
                        onStockCheck: widget.onStockCheck != null
                            ? () => widget.onStockCheck!(p.id)
                            : null,
                        onBatchManage: widget.batchController != null
                            ? () => Navigator.of(context).push(
                                  MaterialPageRoute<void>(
                                    builder: (_) => BatchManagementPage(
                                      productId: p.id,
                                      productName: p.name,
                                      controller: widget.batchController!,
                                    ),
                                  ),
                                )
                            : null,
                      ),
                    ),
                  ),

                const SizedBox(height: 8),

                // ── 分页 ────────────────────────────────────────────────
                if (!busy || widget.controller.list.isNotEmpty)
                  PaginationBar(
                    page: page,
                    totalPages: totalPages,
                    loading: busy,
                    onPrev: page > 1 ? () => _loadProducts(page: page - 1) : null,
                    onNext: page < totalPages
                        ? () => _loadProducts(page: page + 1)
                        : null,
                  ),
                const SizedBox(height: 16),
              ],
            ),
          ),
        );
      },
    );
  }

  List<Widget> _buildSkeletons(int count) => List.generate(
        count,
        (_) => const Padding(
          padding: EdgeInsets.only(bottom: 10),
          child: _SkeletonCard(),
        ),
      );
}

// ════════════════════════════════════════════════════════════════════════════
// _PageHeader — 紧凑标题行（替换过重的 BrandHeroBanner）
// ════════════════════════════════════════════════════════════════════════════

class _PageHeader extends StatelessWidget {
  const _PageHeader({
    required this.total,
    required this.page,
    required this.totalPages,
    required this.filterCategory,
    required this.categoryController,
  });

  final int total;
  final int page;
  final int totalPages;
  final CategoryNode? filterCategory;
  final CategoryController? categoryController;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    final String? catPath = filterCategory != null && categoryController != null
        ? categoryController!.buildPath(filterCategory!.id)
        : null;

    return Row(
      crossAxisAlignment: CrossAxisAlignment.end,
      children: <Widget>[
        Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Container(
                  padding: const EdgeInsets.all(6),
                  decoration: BoxDecoration(
                    color: const Color(0xFF3B82F6).withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(10),
                  ),
                  child: const Icon(
                    Icons.inventory_2_rounded,
                    size: 20,
                    color: Color(0xFF3B82F6),
                  ),
                ),
                const SizedBox(width: 8),
                Text(
                  '商品中心',
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w800,
                      ),
                ),
              ],
            ),
            if (catPath != null) ...<Widget>[
              const SizedBox(height: 2),
              Text(
                '分类：$catPath',
                style: TextStyle(fontSize: 11, color: cs.primary),
              ),
            ],
          ],
        ),
        const Spacer(),
        Text(
          '共 $total 件 · $page/$totalPages 页',
          style: Theme.of(context)
              .textTheme
              .bodySmall
              ?.copyWith(color: cs.onSurfaceVariant),
        ),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _FilterCard
// ════════════════════════════════════════════════════════════════════════════

class _FilterCard extends StatelessWidget {
  const _FilterCard({
    required this.keywordController,
    required this.barcodeController,
    required this.busy,
    required this.filterCategory,
    required this.categoryController,
    required this.onKeywordChanged,
    required this.onSearch,
    required this.onReset,
    required this.onScanBarcode,
    required this.onPickCategory,
    required this.onClearCategory,
  });

  final TextEditingController keywordController;
  final TextEditingController barcodeController;
  final bool busy;
  final CategoryNode? filterCategory;
  final CategoryController? categoryController;
  final ValueChanged<String> onKeywordChanged;
  final VoidCallback onSearch;
  final VoidCallback onReset;
  final VoidCallback onScanBarcode;
  final VoidCallback onPickCategory;
  final VoidCallback onClearCategory;

  @override
  Widget build(BuildContext context) {
    final String catLabel = filterCategory != null && categoryController != null
        ? (categoryController!.buildPath(filterCategory!.id) ??
            filterCategory!.name)
        : '全部分类';

    return SectionCard(
      title: '商品搜索',
      child: Column(
        children: <Widget>[
          const SizedBox(height: 4),
          TextField(
            controller: keywordController,
            enabled: !busy,
            onChanged: onKeywordChanged,
            onSubmitted: busy ? null : (_) => onSearch(),
            textInputAction: TextInputAction.search,
            decoration: const InputDecoration(
              labelText: '关键字',
              hintText: '名称 / SKU / 条码模糊匹配',
              border: OutlineInputBorder(),
              isDense: true,
              prefixIcon: Icon(Icons.search, size: 18),
            ),
          ),
          const SizedBox(height: 8),
          TextField(
            controller: barcodeController,
            enabled: !busy,
            textInputAction: TextInputAction.search,
            onSubmitted: busy ? null : (_) => onSearch(),
            decoration: InputDecoration(
              labelText: '条码',
              hintText: '精确匹配（支持扫码枪/摄像头）',
              border: const OutlineInputBorder(),
              isDense: true,
              prefixIcon: const Icon(Icons.qr_code, size: 18),
              suffixIcon: IconButton(
                tooltip: '扫码填充条码',
                onPressed: busy ? null : onScanBarcode,
                icon: const Icon(Icons.qr_code_scanner),
              ),
            ),
          ),

          // 分类筛选
          if (categoryController != null) ...<Widget>[
            const SizedBox(height: 8),
            InkWell(
              onTap: busy ? null : onPickCategory,
              borderRadius: BorderRadius.circular(8),
              child: InputDecorator(
                decoration: InputDecoration(
                  labelText: '分类筛选',
                  border: const OutlineInputBorder(),
                  isDense: true,
                  prefixIcon:
                      const Icon(Icons.category_outlined, size: 18),
                  suffixIcon: filterCategory != null
                      ? IconButton(
                          icon: const Icon(Icons.close, size: 18),
                          onPressed: busy ? null : onClearCategory,
                          tooltip: '清除分类筛选',
                        )
                      : const Icon(Icons.expand_more, size: 18),
                ),
                child: Text(
                  catLabel,
                  style: TextStyle(
                    fontSize: 14,
                    color: filterCategory != null
                        ? Theme.of(context).colorScheme.onSurface
                        : Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
              ),
            ),
          ],

          const SizedBox(height: 10),
          Row(
            children: <Widget>[
              FilledButton.icon(
                onPressed: busy ? null : onSearch,
                icon: const Icon(Icons.search, size: 16),
                label: const Text('查询'),
              ),
              const SizedBox(width: 8),
              OutlinedButton.icon(
                onPressed: busy ? null : onReset,
                icon: const Icon(Icons.clear, size: 16),
                label: const Text('重置'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _ProductCard
// ════════════════════════════════════════════════════════════════════════════

class _ProductCard extends StatelessWidget {
  const _ProductCard({
    required this.product,
    required this.canViewCostPrice,
    required this.canWrite,
    required this.busy,
    required this.categoryController,
    required this.onEdit,
    required this.onDelete,
    this.onStockCheck,
    this.onBatchManage,
  });

  final ProductData product;
  final bool canViewCostPrice;
  final bool canWrite;
  final bool busy;
  final CategoryController? categoryController;
  final VoidCallback onEdit;
  final VoidCallback onDelete;
  final VoidCallback? onStockCheck;
  final VoidCallback? onBatchManage;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;

    // 使用 extension 计算属性
    final bool isLow = product.isLowStock;
    final double progress = product.stockProgress;
    final String? margin = canViewCostPrice ? product.grossMarginStr : null;

    final Color stockBarColor = progress < 0.3
        ? cs.error
        : progress < 0.6
            ? const Color(0xFFF59E0B)
            : const Color(0xFF10B981);

    // 分类路径
    final String? catPath = categoryController != null && product.categoryId != null
        ? categoryController!.buildPath(product.categoryId)
        : null;

    return Container(
      decoration: BoxDecoration(
        color: cs.surface,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: isLow
              ? cs.error.withValues(alpha: 0.35)
              : cs.outlineVariant,
        ),
        boxShadow: <BoxShadow>[
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.04),
            blurRadius: 10,
            offset: const Offset(0, 3),
          ),
        ],
      ),
      child: InkWell(
        borderRadius: BorderRadius.circular(16),
        onTap: canWrite ? null : onEdit,
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[

              // ── Row 1: 商品名 + 库存徽章 ──────────────────────────────
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          product.name,
                          style: const TextStyle(
                            fontWeight: FontWeight.w800,
                            fontSize: 15,
                          ),
                          overflow: TextOverflow.ellipsis,
                          maxLines: 2,
                        ),
                        const SizedBox(height: 3),
                        Text(
                          'SKU：${product.sku.isEmpty ? '-' : product.sku}',
                          style: TextStyle(
                              fontSize: 11, color: cs.onSurfaceVariant),
                        ),
                        // 分类路径
                        if (catPath != null) ...<Widget>[
                          const SizedBox(height: 2),
                          Row(
                            children: <Widget>[
                              Icon(Icons.category_outlined,
                                  size: 11, color: cs.primary),
                              const SizedBox(width: 3),
                              Expanded(
                                child: Text(
                                  catPath,
                                  style: TextStyle(
                                    fontSize: 11,
                                    color: cs.primary,
                                    fontWeight: FontWeight.w500,
                                  ),
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                            ],
                          ),
                        ],
                      ],
                    ),
                  ),
                  const SizedBox(width: 10),
                  // 库存徽章
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.end,
                    children: <Widget>[
                      Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 10, vertical: 4),
                        decoration: BoxDecoration(
                          color: isLow
                              ? cs.error.withValues(alpha: 0.12)
                              : const Color(0xFF10B981).withValues(alpha: 0.12),
                          borderRadius: BorderRadius.circular(999),
                          border: Border.all(
                            color: isLow
                                ? cs.error.withValues(alpha: 0.4)
                                : const Color(0xFF10B981)
                                    .withValues(alpha: 0.4),
                          ),
                        ),
                        child: Text(
                          isLow
                              ? '库存 ${product.currentStock} / 需 ${product.minStockLimit}'
                              : '库存 ${product.currentStock}',
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w700,
                            color: isLow
                                ? cs.error
                                : const Color(0xFF059669),
                          ),
                        ),
                      ),
                      if (isLow) ...<Widget>[
                        const SizedBox(height: 3),
                        Text(
                          '差 ${product.shortage} 件',
                          style: TextStyle(
                              fontSize: 10,
                              color: cs.error,
                              fontWeight: FontWeight.w600),
                        ),
                      ],
                      const SizedBox(height: 4),
                      SizedBox(
                        width: 72,
                        child: ClipRRect(
                          borderRadius: BorderRadius.circular(3),
                          child: LinearProgressIndicator(
                            value: progress,
                            minHeight: 5,
                            backgroundColor: cs.surfaceContainerHighest,
                            valueColor: AlwaysStoppedAnimation<Color>(
                                stockBarColor),
                          ),
                        ),
                      ),
                    ],
                  ),
                ],
              ),

              const SizedBox(height: 10),

              // ── Row 2: 条码 + 单位 ────────────────────────────────────
              Row(
                children: <Widget>[
                  Icon(Icons.qr_code, size: 13, color: cs.onSurfaceVariant),
                  const SizedBox(width: 4),
                  Expanded(
                    child: Text(
                      product.barcode.isEmpty ? '-' : product.barcode,
                      style:
                          TextStyle(fontSize: 12, color: cs.onSurfaceVariant),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Icon(Icons.straighten,
                      size: 13, color: cs.onSurfaceVariant),
                  const SizedBox(width: 4),
                  Text(
                    '单位：${product.unit}',
                    style:
                        TextStyle(fontSize: 12, color: cs.onSurfaceVariant),
                  ),
                ],
              ),
              const SizedBox(height: 10),

              // ── Row 3: 价格区 ─────────────────────────────────────────
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                decoration: BoxDecoration(
                  color: cs.surfaceContainerHighest.withValues(alpha: 0.5),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Row(
                  children: <Widget>[
                    // 零售价
                    Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text('零售价',
                            style: TextStyle(
                                fontSize: 10, color: cs.onSurfaceVariant)),
                        Text(
                          '¥${product.retailPrice}',
                          style: const TextStyle(
                            fontSize: 16,
                            fontWeight: FontWeight.w800,
                            color: Color(0xFF3B82F6),
                            letterSpacing: -0.3,
                          ),
                        ),
                      ],
                    ),
                    if (canViewCostPrice && product.costPrice != null) ...[
                      const SizedBox(width: 12),
                      Container(
                          width: 1,
                          height: 28,
                          color: cs.outlineVariant.withValues(alpha: 0.5)),
                      const SizedBox(width: 12),
                      Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          Text('均摊成本',
                              style: TextStyle(
                                  fontSize: 10, color: cs.onSurfaceVariant)),
                          Text(
                            '¥${product.costPrice}',
                            style: TextStyle(
                              fontSize: 14,
                              fontWeight: FontWeight.w700,
                              color: cs.onSurface,
                            ),
                          ),
                        ],
                      ),
                      if (product.lastInboundUnitCost != null) ...[
                        const SizedBox(width: 12),
                        Container(
                            width: 1,
                            height: 28,
                            color: cs.outlineVariant.withValues(alpha: 0.5)),
                        const SizedBox(width: 12),
                        Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            Text('最近进货',
                                style: TextStyle(
                                    fontSize: 10, color: cs.onSurfaceVariant)),
                            Text(
                              '¥${product.lastInboundUnitCost}',
                              style: TextStyle(
                                fontSize: 13,
                                fontWeight: FontWeight.w600,
                                color: cs.onSurfaceVariant,
                              ),
                            ),
                          ],
                        ),
                      ],
                    ],
                    if (margin != null) ...<Widget>[
                      const Spacer(),
                      Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 8, vertical: 3),
                        decoration: BoxDecoration(
                          color: const Color(0xFF10B981).withValues(alpha: 0.12),
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Text(
                          '📈 毛利 $margin',
                          style: const TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w700,
                            color: Color(0xFF059669),
                          ),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
              const SizedBox(height: 10),

              // ── Row 4: 元数据 ─────────────────────────────────────────
              Row(
                children: <Widget>[
                  Icon(Icons.warning_amber_rounded,
                      size: 12, color: cs.onSurfaceVariant),
                  const SizedBox(width: 3),
                  Text(
                    '预警 ${product.minStockLimit} 件',
                    style:
                        TextStyle(fontSize: 11, color: cs.onSurfaceVariant),
                  ),
                ],
              ),
              const SizedBox(height: 10),

              // ── Row 5: 操作按钮 ───────────────────────────────────────
              if (canWrite)
                Row(
                  children: <Widget>[
                    OutlinedButton.icon(
                      onPressed: busy ? null : onEdit,
                      icon: const Icon(Icons.edit_outlined, size: 16),
                      label: const Text('编辑'),
                    ),
                    const SizedBox(width: 8),
                    if (onStockCheck != null) ...<Widget>[
                      OutlinedButton.icon(
                        onPressed: busy ? null : onStockCheck,
                        icon: const Icon(
                            Icons.playlist_add_check_rounded,
                            size: 16),
                        label: const Text('盘点'),
                      ),
                      const SizedBox(width: 8),
                    ],
                    if (onBatchManage != null) ...<Widget>[
                      OutlinedButton.icon(
                        onPressed: busy ? null : onBatchManage,
                        icon: const Icon(Icons.inventory_2_outlined, size: 16),
                        label: const Text('批次'),
                      ),
                      const SizedBox(width: 8),
                    ],
                    TextButton.icon(
                      onPressed: busy ? null : onDelete,
                      style: TextButton.styleFrom(
                        foregroundColor: cs.error,
                      ),
                      icon: const Icon(Icons.delete_outline, size: 16),
                      label: const Text('删除'),
                    ),
                  ],
                )
              else
                Row(
                  children: <Widget>[
                    OutlinedButton.icon(
                      onPressed: onEdit,
                      icon: const Icon(Icons.visibility_outlined, size: 16),
                      label: const Text('查看详情'),
                    ),
                    if (onBatchManage != null) ...<Widget>[
                      const SizedBox(width: 8),
                      OutlinedButton.icon(
                        onPressed: onBatchManage,
                        icon: const Icon(Icons.inventory_2_outlined, size: 16),
                        label: const Text('批次'),
                      ),
                    ],
                  ],
                ),
            ],
          ),
        ),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _CategoryFilterSheet — 分类筛选弹窗（三级平铺）
// ════════════════════════════════════════════════════════════════════════════

class _CategoryFilterSheet extends StatefulWidget {
  const _CategoryFilterSheet({required this.controller});
  final CategoryController controller;

  static Future<CategoryNode?> show(
      BuildContext context, CategoryController controller) {
    return showModalBottomSheet<CategoryNode>(
      context: context,
      useSafeArea: true,
      isScrollControlled: true,
      builder: (_) => _CategoryFilterSheet(controller: controller),
    );
  }

  @override
  State<_CategoryFilterSheet> createState() => _CategoryFilterSheetState();
}

class _CategoryFilterSheetState extends State<_CategoryFilterSheet> {
  CategoryNode? _l1;
  CategoryNode? _l2;

  @override
  Widget build(BuildContext context) {
    final List<CategoryNode> l1s = widget.controller.tree;
    final List<CategoryNode> l2s = _l1?.children ?? <CategoryNode>[];
    final List<CategoryNode> l3s = _l2?.children ?? <CategoryNode>[];

    return DraggableScrollableSheet(
      expand: false,
      initialChildSize: 0.55,
      maxChildSize: 0.85,
      builder: (_, controller) => Padding(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            // Handle
            Center(
              child: Container(
                width: 36, height: 4,
                decoration: BoxDecoration(
                  color: Theme.of(context).dividerColor,
                  borderRadius: BorderRadius.circular(2),
                ),
              ),
            ),
            const SizedBox(height: 12),
            Row(
              children: <Widget>[
                const Text('按分类筛选',
                    style: TextStyle(
                        fontSize: 16, fontWeight: FontWeight.w800)),
                const Spacer(),
                TextButton(
                  onPressed: () async {
                    Navigator.of(context).pop(null);
                    await Future<void>.delayed(Duration.zero);
                    if (context.mounted) {
                      await Navigator.of(context).push(
                        MaterialPageRoute<void>(
                          builder: (_) => CategoryManagementPage(
                            controller: widget.controller,
                          ),
                        ),
                      );
                    }
                  },
                  child: const Text('管理分类'),
                ),
                TextButton(
                  onPressed: () => Navigator.of(context).pop(null),
                  child: const Text('不限'),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Expanded(
              child: ListView(
                controller: controller,
                children: <Widget>[
                  // 大类
                  if (l1s.isNotEmpty) ...<Widget>[
                    const SectionLabel(label: '大类'),
                    const SizedBox(height: 6),
                    Wrap(
                      spacing: 8,
                      runSpacing: 6,
                      children: l1s.map((CategoryNode n) {
                        final bool sel = _l1?.id == n.id;
                        return _CatChip(
                          label: n.name,
                          selected: sel,
                          onTap: () {
                            setState(() {
                              _l1 = sel ? null : n;
                              _l2 = null;
                            });
                          },
                        );
                      }).toList(),
                    ),
                  ],
                  // 中类
                  if (l2s.isNotEmpty) ...<Widget>[
                    const SizedBox(height: 12),
                    const SectionLabel(label: '中类'),
                    const SizedBox(height: 6),
                    Wrap(
                      spacing: 8,
                      runSpacing: 6,
                      children: l2s.map((CategoryNode n) {
                        final bool sel = _l2?.id == n.id;
                        return _CatChip(
                          label: n.name,
                          selected: sel,
                          onTap: () =>
                              setState(() => _l2 = sel ? null : n),
                        );
                      }).toList(),
                    ),
                  ],
                  // 小类
                  if (l3s.isNotEmpty) ...<Widget>[
                    const SizedBox(height: 12),
                    const SectionLabel(label: '小类'),
                    const SizedBox(height: 6),
                    Wrap(
                      spacing: 8,
                      runSpacing: 6,
                      children: l3s.map((CategoryNode n) => _CatChip(
                            label: n.name,
                            selected: false,
                            onTap: () => Navigator.of(context).pop(n),
                          )).toList(),
                    ),
                  ],
                  // 若选中了大类/中类，直接可以用该层级筛选
                  if (_l1 != null) ...<Widget>[
                    const SizedBox(height: 20),
                    SizedBox(
                      width: double.infinity,
                      child: FilledButton.icon(
                        icon: const Icon(Icons.done, size: 16),
                        label: Text(
                          _l2 != null
                              ? '筛选：${_l2!.name}'
                              : '筛选：${_l1!.name}（全部子分类）',
                        ),
                        onPressed: () =>
                            Navigator.of(context).pop(_l2 ?? _l1),
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _CatChip extends StatelessWidget {
  const _CatChip({
    required this.label,
    required this.selected,
    required this.onTap,
  });
  final String label;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    return GestureDetector(
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 160),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        decoration: BoxDecoration(
          color: selected
              ? cs.primary.withValues(alpha: 0.15)
              : cs.surfaceContainerHighest,
          borderRadius: BorderRadius.circular(20),
          border: Border.all(
            color: selected ? cs.primary : cs.outlineVariant,
          ),
        ),
        child: Text(
          label,
          style: TextStyle(
            fontSize: 13,
            fontWeight: selected ? FontWeight.w700 : FontWeight.w400,
            color: selected ? cs.primary : cs.onSurface,
          ),
        ),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _EmptyState
// ════════════════════════════════════════════════════════════════════════════

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.hasFilter, required this.onClearFilter});

  final bool hasFilter;
  final VoidCallback onClearFilter;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 48),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(
            hasFilter
                ? Icons.manage_search_rounded
                : Icons.inventory_2_outlined,
            size: 64,
            color: const Color(0xFF94A3B8),
          ),
          const SizedBox(height: 16),
          Text(
            hasFilter ? '未找到匹配商品' : '暂无商品数据',
            style: const TextStyle(fontSize: 15, color: Color(0xFF64748B)),
          ),
          const SizedBox(height: 8),
          Text(
            hasFilter
                ? '请尝试调整关键字、条码或分类后重新查询'
                : '点击右下角「新建商品」按钮添加第一个商品',
            style: const TextStyle(fontSize: 13, color: Color(0xFF94A3B8)),
            textAlign: TextAlign.center,
          ),
          if (hasFilter) ...<Widget>[
            const SizedBox(height: 16),
            OutlinedButton.icon(
              onPressed: onClearFilter,
              icon: const Icon(Icons.clear, size: 16),
              label: const Text('清除所有筛选条件'),
            ),
          ],
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _SkeletonCard / _Bone
// ════════════════════════════════════════════════════════════════════════════

class _SkeletonCard extends StatelessWidget {
  const _SkeletonCard();

  @override
  Widget build(BuildContext context) {
    final Color base = Theme.of(context).colorScheme.surfaceContainerHighest;
    return Container(
      height: 200,
      decoration: BoxDecoration(
        color: base,
        borderRadius: BorderRadius.circular(16),
      ),
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            children: <Widget>[
              _Bone(width: 160, height: 16, base: base),
              const Spacer(),
              _Bone(width: 72, height: 24, base: base),
            ],
          ),
          const SizedBox(height: 8),
          _Bone(width: 120, height: 11, base: base),
          const SizedBox(height: 6),
          _Bone(width: 180, height: 11, base: base),
          const SizedBox(height: 10),
          _Bone(width: double.infinity, height: 52, base: base),
          const SizedBox(height: 10),
          _Bone(width: 200, height: 12, base: base),
        ],
      ),
    );
  }
}

class _Bone extends StatelessWidget {
  const _Bone({required this.width, required this.height, required this.base});
  final double width;
  final double height;
  final Color base;

  @override
  Widget build(BuildContext context) => Container(
        width: width,
        height: height,
        decoration: BoxDecoration(
          color: base.withValues(alpha: 0.5),
          borderRadius: BorderRadius.circular(6),
        ),
      );
}
