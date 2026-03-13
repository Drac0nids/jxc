import 'dart:async';

import 'package:flutter/material.dart';

import '../../../core/widgets/barcode_scanner_sheet.dart';
import '../../../core/widgets/brand_ui.dart';
import '../application/product_controller.dart';
import '../models/product_models.dart';
import 'products_edit_page.dart';

class ProductsPage extends StatefulWidget {
  const ProductsPage({super.key, required this.controller});

  final ProductController controller;

  @override
  State<ProductsPage> createState() => _ProductsPageState();
}

class _ProductsPageState extends State<ProductsPage> {
  static final RegExp _moneyPattern = RegExp(r'^\d+(\.\d{1,4})?$');

  final TextEditingController _keywordController = TextEditingController();
  final TextEditingController _barcodeController = TextEditingController();

  bool _filterExpanded = true;
  Timer? _debounce;

  @override
  void initState() {
    super.initState();
    _keywordController.text = widget.controller.keyword;
    _barcodeController.text = widget.controller.barcode;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadProducts(page: 1);
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
              // Filter toggle
              IconButton(
                tooltip: _filterExpanded ? '收起筛选' : '展开筛选',
                onPressed: () =>
                    setState(() => _filterExpanded = !_filterExpanded),
                icon: Icon(_filterExpanded
                    ? Icons.filter_list_off
                    : Icons.filter_list),
              ),
              IconButton(
                tooltip: '刷新',
                onPressed:
                    busy ? null : () => _loadProducts(page: page),
                icon: const Icon(Icons.refresh),
              ),
            ],
          ),
          floatingActionButton: canWrite
              ? FloatingActionButton.extended(
                  onPressed: busy ? null : _showCreateProductSheet,
                  icon: const Icon(Icons.add),
                  label: const Text('新建商品'),
                )
              : null,
          body: RefreshIndicator(
            onRefresh: () => _loadProducts(page: page),
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: <Widget>[
                // Hero Banner
                const BrandHeroBanner(
                  title: '商品中心',
                  subtitle: '统一管理商品信息、价格体系与库存状态',
                  icon: Icons.inventory_2_rounded,
                  gradientSeedColor: Color(0xFF3B82F6),
                ),
                const SizedBox(height: 12),

                // Role notice
                if (!canWrite)
                  const Padding(
                    padding: EdgeInsets.only(bottom: 8),
                    child: StatusNotice(
                      message: '当前角色为只读，可浏览商品列表与详情，不可创建/编辑/删除。',
                      tone: NoticeTone.warning,
                    ),
                  ),

                // Error / success messages
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

                // Collapsible filter
                AnimatedCrossFade(
                  duration: const Duration(milliseconds: 220),
                  crossFadeState: _filterExpanded
                      ? CrossFadeState.showFirst
                      : CrossFadeState.showSecond,
                  firstChild: _FilterCard(
                    keywordController: _keywordController,
                    barcodeController: _barcodeController,
                    busy: busy,
                    onKeywordChanged: _onKeywordChanged,
                    onSearch: () => _loadProducts(page: 1),
                    onReset: _resetFilters,
                    onScanBarcode: _fillFilterBarcodeByCamera,
                  ),
                  secondChild: const SizedBox.shrink(),
                ),
                const SizedBox(height: 12),

                // List header
                _ListHeader(total: total, page: page, totalPages: totalPages),
                const SizedBox(height: 8),

                // List
                if (busy && widget.controller.list.isEmpty)
                  ..._buildSkeletons(4)
                else if (widget.controller.list.isEmpty)
                  _EmptyState(
                    hasFilter: _keywordController.text.isNotEmpty ||
                        _barcodeController.text.isNotEmpty,
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
                        onEdit: () => _openEditPage(p),
                        onDelete: () => _deleteProduct(p),
                      ),
                    ),
                  ),

                const SizedBox(height: 8),

                // Pagination
                if (!busy || widget.controller.list.isNotEmpty)
                  _PaginationBar(
                    page: page,
                    totalPages: totalPages,
                    loading: busy,
                    onPrev: page > 1
                        ? () => _loadProducts(page: page - 1)
                        : null,
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
// Filter Card
// ════════════════════════════════════════════════════════════════════════════

class _FilterCard extends StatelessWidget {
  const _FilterCard({
    required this.keywordController,
    required this.barcodeController,
    required this.busy,
    required this.onKeywordChanged,
    required this.onSearch,
    required this.onReset,
    required this.onScanBarcode,
  });

  final TextEditingController keywordController;
  final TextEditingController barcodeController;
  final bool busy;
  final ValueChanged<String> onKeywordChanged;
  final VoidCallback onSearch;
  final VoidCallback onReset;
  final VoidCallback onScanBarcode;

  @override
  Widget build(BuildContext context) {
    return SectionCard(
      title: '商品搜索',
      subtitle: '关键字 400ms 防抖自动搜索，条码支持扫码枪',
      child: Column(
        children: <Widget>[
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
// List Header
// ════════════════════════════════════════════════════════════════════════════

class _ListHeader extends StatelessWidget {
  const _ListHeader({
    required this.total,
    required this.page,
    required this.totalPages,
  });

  final int total;
  final int page;
  final int totalPages;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: <Widget>[
        Text(
          '商品列表',
          style: Theme.of(context)
              .textTheme
              .titleMedium
              ?.copyWith(fontWeight: FontWeight.w700),
        ),
        const Spacer(),
        Text(
          '共 $total 件 · $page/$totalPages 页',
          style: Theme.of(context).textTheme.bodySmall,
        ),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Product Card
// ════════════════════════════════════════════════════════════════════════════

class _ProductCard extends StatelessWidget {
  const _ProductCard({
    required this.product,
    required this.canViewCostPrice,
    required this.canWrite,
    required this.busy,
    required this.onEdit,
    required this.onDelete,
  });

  final ProductData product;
  final bool canViewCostPrice;
  final bool canWrite;
  final bool busy;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;

    // Stock metrics
    final int stock = product.currentStock;
    final int limit = product.minStockLimit;
    final bool isLowStock = limit > 0 && stock < limit;
    final int shortage = isLowStock ? limit - stock : 0;

    // Progress bar - relative to minStockLimit (or fallback to stock+20)
    final double progressValue = limit > 0
        ? (stock / limit).clamp(0.0, 1.0)
        : stock <= 0
            ? 0.0
            : (stock / (stock + 20)).clamp(0.0, 1.0);

    final Color stockBarColor = progressValue < 0.3
        ? cs.error
        : progressValue < 0.6
            ? const Color(0xFFF59E0B)
            : const Color(0xFF10B981);

    // Price & profit — 均摊成本用于毛利率计算
    final double? retailVal =
        double.tryParse(product.retailPrice.replaceAll('¥', ''));
    final double? avgCostVal = canViewCostPrice && product.costPrice != null
        ? double.tryParse(product.costPrice!.replaceAll('¥', ''))
        : null;
    final double? lastInboundVal =
        canViewCostPrice && product.lastInboundUnitCost != null
            ? double.tryParse(
                product.lastInboundUnitCost!.replaceAll('¥', ''))
            : null;
    // 毛利率基于加权均摊成本计算
    final String? profitRateStr =
        (retailVal != null && avgCostVal != null && retailVal > 0)
            ? '${((retailVal - avgCostVal) / retailVal * 100).toStringAsFixed(1)}%'
            : null;

    return Container(
      decoration: BoxDecoration(
        color: cs.surface,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: isLowStock
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
        onTap: canWrite ? null : onEdit, // readonly: tap to view
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              // ── Row 1: name + stock badge ──────────────────────────────
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
                              fontSize: 11,
                              color: cs.onSurfaceVariant),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(width: 10),
                  // Stock badge
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.end,
                    children: <Widget>[
                      Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 10, vertical: 4),
                        decoration: BoxDecoration(
                          color: isLowStock
                              ? cs.error.withValues(alpha: 0.12)
                              : const Color(0xFF10B981).withValues(alpha: 0.12),
                          borderRadius: BorderRadius.circular(999),
                          border: Border.all(
                            color: isLowStock
                                ? cs.error.withValues(alpha: 0.4)
                                : const Color(0xFF10B981)
                                    .withValues(alpha: 0.4),
                          ),
                        ),
                        child: Text(
                          isLowStock
                              ? '库存 $stock / 需 $limit'
                              : '库存 $stock',
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w700,
                            color: isLowStock
                                ? cs.error
                                : const Color(0xFF059669),
                          ),
                        ),
                      ),
                      if (isLowStock) ...<Widget>[
                        const SizedBox(height: 3),
                        Text(
                          '差 $shortage 件',
                          style: TextStyle(
                              fontSize: 10,
                              color: cs.error,
                              fontWeight: FontWeight.w600),
                        ),
                      ],
                      const SizedBox(height: 4),
                      // Progress bar
                      SizedBox(
                        width: 72,
                        child: ClipRRect(
                          borderRadius: BorderRadius.circular(3),
                          child: LinearProgressIndicator(
                            value: progressValue,
                            minHeight: 5,
                            backgroundColor:
                                cs.surfaceContainerHighest,
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

              // ── Row 2: barcode + unit ──────────────────────────────────
              Row(
                children: <Widget>[
                  Icon(Icons.qr_code, size: 13, color: cs.onSurfaceVariant),
                  const SizedBox(width: 4),
                  Expanded(
                    child: Text(
                      product.barcode.isEmpty ? '-' : product.barcode,
                      style: TextStyle(
                          fontSize: 12, color: cs.onSurfaceVariant),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Icon(Icons.straighten, size: 13, color: cs.onSurfaceVariant),
                  const SizedBox(width: 4),
                  Text(
                    '单位：${product.unit}',
                    style:
                        TextStyle(fontSize: 12, color: cs.onSurfaceVariant),
                  ),
                ],
              ),
              const SizedBox(height: 10),

              // ── Row 3: prices ──────────────────────────────────────────
              Container(
                padding: const EdgeInsets.symmetric(
                    horizontal: 12, vertical: 8),
                decoration: BoxDecoration(
                  color: cs.surfaceContainerHighest.withValues(alpha: 0.5),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Row(
                  children: <Widget>[
                    // Retail price
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
                                  fontSize: 10,
                                  color: cs.onSurfaceVariant)),
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
                      if (lastInboundVal != null) ...[
                        const SizedBox(width: 12),
                        Container(
                            width: 1,
                            height: 28,
                            color:
                                cs.outlineVariant.withValues(alpha: 0.5)),
                        const SizedBox(width: 12),
                        Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            Text('最近进货',
                                style: TextStyle(
                                    fontSize: 10,
                                    color: cs.onSurfaceVariant)),
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
                    if (profitRateStr != null) ...<Widget>[
                      const Spacer(),
                      Container(
                        padding: const EdgeInsets.symmetric(
                            horizontal: 8, vertical: 3),
                        decoration: BoxDecoration(
                          color: const Color(0xFF10B981)
                              .withValues(alpha: 0.12),
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Text(
                          '📈 毛利 $profitRateStr',
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

              // ── Row 4: secondary info ─────────────────────────────────
              Wrap(
                spacing: 12,
                runSpacing: 4,
                children: <Widget>[
                  _MetaChip(
                      icon: Icons.warning_amber_rounded,
                      label: '预警 ${product.minStockLimit}件'),
                ],
              ),
              const SizedBox(height: 10),

              // ── Row 5: actions ────────────────────────────────────────
              if (canWrite)
                Row(
                  children: <Widget>[
                    OutlinedButton.icon(
                      onPressed: busy ? null : onEdit,
                      icon: const Icon(Icons.edit_outlined, size: 16),
                      label: const Text('编辑'),
                    ),
                    const SizedBox(width: 8),
                    TextButton.icon(
                      onPressed: busy ? null : onDelete,
                      style: TextButton.styleFrom(
                        foregroundColor:
                            Theme.of(context).colorScheme.error,
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
// Pagination Bar
// ════════════════════════════════════════════════════════════════════════════

class _PaginationBar extends StatelessWidget {
  const _PaginationBar({
    required this.page,
    required this.totalPages,
    required this.loading,
    required this.onPrev,
    required this.onNext,
  });

  final int page;
  final int totalPages;
  final bool loading;
  final VoidCallback? onPrev;
  final VoidCallback? onNext;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: <Widget>[
        OutlinedButton.icon(
          onPressed: loading ? null : onPrev,
          icon: const Icon(Icons.chevron_left, size: 18),
          label: const Text('上一页'),
        ),
        const Spacer(),
        Container(
          padding:
              const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
          decoration: BoxDecoration(
            color: Theme.of(context)
                .colorScheme
                .surfaceContainerHighest
                .withValues(alpha: 0.6),
            borderRadius: BorderRadius.circular(20),
          ),
          child: Text(
            '第 $page / $totalPages 页',
            style: const TextStyle(
                fontSize: 13, fontWeight: FontWeight.w600),
          ),
        ),
        const Spacer(),
        OutlinedButton.icon(
          onPressed: loading ? null : onNext,
          icon: const Icon(Icons.chevron_right, size: 18),
          label: const Text('下一页'),
        ),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Empty State
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
            style: const TextStyle(
                fontSize: 15, color: Color(0xFF64748B)),
          ),
          const SizedBox(height: 8),
          Text(
            hasFilter ? '请尝试调整关键字或条码后重新查询' : '点击右下角「新建商品」按钮添加第一个商品',
            style: const TextStyle(
                fontSize: 13, color: Color(0xFF94A3B8)),
          ),
          if (hasFilter) ...<Widget>[
            const SizedBox(height: 16),
            OutlinedButton.icon(
              onPressed: onClearFilter,
              icon: const Icon(Icons.clear, size: 16),
              label: const Text('清除筛选条件'),
            ),
          ],
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Skeleton Card
// ════════════════════════════════════════════════════════════════════════════

class _SkeletonCard extends StatelessWidget {
  const _SkeletonCard();

  @override
  Widget build(BuildContext context) {
    final Color base =
        Theme.of(context).colorScheme.surfaceContainerHighest;
    return Container(
      height: 180,
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
          const SizedBox(height: 10),
          _Bone(width: double.infinity, height: 52, base: base),
          const SizedBox(height: 10),
          _Bone(width: 200, height: 12, base: base),
          const SizedBox(height: 8),
          _Bone(width: 120, height: 12, base: base),
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

// ════════════════════════════════════════════════════════════════════════════
// Meta Chip
// ════════════════════════════════════════════════════════════════════════════

class _MetaChip extends StatelessWidget {
  const _MetaChip({required this.icon, required this.label});
  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Icon(icon, size: 12, color: cs.onSurfaceVariant),
        const SizedBox(width: 3),
        Text(label,
            style:
                TextStyle(fontSize: 11, color: cs.onSurfaceVariant)),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Create Product Sheet
// ════════════════════════════════════════════════════════════════════════════

class CreateProductSheet extends StatefulWidget {
  const CreateProductSheet({
    super.key,
    required this.controller,
    required this.moneyPattern,
    this.presetBarcode,
  });

  final ProductController controller;
  final RegExp moneyPattern;
  final String? presetBarcode;

  @override
  State<CreateProductSheet> createState() => _CreateProductSheetState();
}

class _CreateProductSheetState extends State<CreateProductSheet> {
  final TextEditingController _skuController = TextEditingController();
  final TextEditingController _barcodeController = TextEditingController();
  final TextEditingController _nameController = TextEditingController();
  final TextEditingController _unitController = TextEditingController();
  final TextEditingController _retailPriceController =
      TextEditingController();
  final TextEditingController _initStockController =
      TextEditingController(text: '0');
  final TextEditingController _minStockLimitController =
      TextEditingController(text: '0');
  final TextEditingController _costPriceController =
      TextEditingController();

  String? _localError;

  @override
  void initState() {
    super.initState();
    final String presetBarcode = widget.presetBarcode?.trim() ?? '';
    if (presetBarcode.isNotEmpty) {
      _barcodeController.text = presetBarcode;
    }
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
                  style: TextStyle(
                      fontSize: 18, fontWeight: FontWeight.w800),
                ),
                const SizedBox(height: 16),

                // ── Section 1: 基本信息 ───────────────────────────────
                const _SectionLabel(label: '基本信息'),
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
                const SizedBox(height: 16),

                // ── Section 2: 价格设定 ───────────────────────────────
                const _SectionLabel(label: '价格设定'),
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

                // ── Section 3: 库存设置 ───────────────────────────────
                const _SectionLabel(label: '库存设置'),
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
                        label: '低库存预警阈值',
                        hint: '默认 0（不预警）',
                        enabled: !submitting,
                        keyboardType: TextInputType.number,
                      ),
                    ),
                  ],
                ),

                // Error
                if (_localError != null) ...<Widget>[
                  const SizedBox(height: 12),
                  Container(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 12, vertical: 8),
                    decoration: BoxDecoration(
                      color: Theme.of(context)
                          .colorScheme
                          .errorContainer,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Row(
                      children: <Widget>[
                        Icon(Icons.error_outline,
                            size: 16,
                            color: Theme.of(context)
                                .colorScheme
                                .onErrorContainer),
                        const SizedBox(width: 6),
                        Expanded(
                          child: Text(
                            _localError!,
                            style: TextStyle(
                              color: Theme.of(context)
                                  .colorScheme
                                  .onErrorContainer,
                              fontSize: 13,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],

                const SizedBox(height: 16),
                Row(
                  children: <Widget>[
                    Expanded(
                      child: FilledButton(
                        onPressed: submitting ? null : _submit,
                        child:
                            Text(submitting ? '创建中…' : '创建商品'),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: OutlinedButton(
                        onPressed: submitting
                            ? null
                            : () => Navigator.of(context).pop(),
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

  Future<void> _submit() async {
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
      ),
    );

    if (created != null && mounted) Navigator.of(context).pop(created);
  }

  String? _validateForm() {
    if (_barcodeController.text.trim().isEmpty) return '条码不能为空';
    if (_nameController.text.trim().isEmpty) return '商品名称不能为空';
    if (_unitController.text.trim().isEmpty) return '单位不能为空';
    if (!widget.moneyPattern
        .hasMatch(_retailPriceController.text.trim())) {
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
}

// ════════════════════════════════════════════════════════════════════════════
// Section Label
// ════════════════════════════════════════════════════════════════════════════

class _SectionLabel extends StatelessWidget {
  const _SectionLabel({required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: <Widget>[
        Container(
          width: 3,
          height: 14,
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.primary,
            borderRadius: BorderRadius.circular(2),
          ),
        ),
        const SizedBox(width: 6),
        Text(
          label,
          style: TextStyle(
            fontWeight: FontWeight.w700,
            fontSize: 13,
            color: Theme.of(context).colorScheme.primary,
          ),
        ),
      ],
    );
  }
}
