import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../application/batch_controller.dart';
import '../application/category_controller.dart';
import '../application/product_controller.dart';
import '../models/product_models.dart';
import 'batch_management_page.dart';
import 'products_edit_page.dart';

const _kMoneyPattern = r'^\d+(\.\d{1,4})?$';
final RegExp _moneyPatternDetail = RegExp(_kMoneyPattern);

// ════════════════════════════════════════════════════════════════════════════
// ProductDetailPage
// ════════════════════════════════════════════════════════════════════════════

class ProductDetailPage extends StatefulWidget {
  const ProductDetailPage({
    super.key,
    required this.product,
    required this.controller,
    this.categoryController,
    this.batchController,
    this.onStockCheck,
  });

  final ProductData product;
  final ProductController controller;
  final CategoryController? categoryController;
  final BatchController? batchController;
  final void Function(int productId)? onStockCheck;

  @override
  State<ProductDetailPage> createState() => _ProductDetailPageState();
}

class _ProductDetailPageState extends State<ProductDetailPage> {
  late ProductData _product;

  @override
  void initState() {
    super.initState();
    _product = widget.product;
  }

  Future<void> _openEdit() async {
    final bool? changed = await Navigator.of(context).push<bool>(
      MaterialPageRoute<bool>(
        builder: (_) => ProductsEditPage(
          controller: widget.controller,
          product: _product,
          moneyPattern: _moneyPatternDetail,
          categoryController: widget.categoryController,
        ),
      ),
    );
    if (changed == true && mounted) {
      // 刷新当前显示的商品数据
      await widget.controller.loadProducts(
        page: widget.controller.page,
        pageSize: widget.controller.pageSize,
      );
      if (mounted) {
        // 从列表中找最新的数据
        final updated = widget.controller.list
            .where((p) => p.id == _product.id)
            .firstOrNull;
        if (updated != null) {
          setState(() => _product = updated);
        }
      }
    }
  }

  Future<void> _confirmDelete() async {
    final bool confirmed = await showDialog<bool>(
          context: context,
          builder: (_) => AlertDialog(
            title: const Text('确认删除商品'),
            content: Text('即将永久删除「${_product.name}」，此操作不可撤销。'),
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
    if (!confirmed || !mounted) return;
    try {
      await widget.controller.deleteProduct(
        id: _product.id,
        expectedVersion: _product.version,
      );
      if (mounted) Navigator.of(context).pop(true); // 通知列表页刷新
    } catch (e) {
      if (!mounted) return;
      final msg = e.toString().replaceFirst('Exception: ', '');
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('删除失败：$msg'),
          backgroundColor: Theme.of(context).colorScheme.error,
          duration: const Duration(seconds: 4),
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    final bool canWrite = widget.controller.canWrite;
    final bool canViewCost = widget.controller.canViewCostPrice;
    final ProductData p = _product;

    final bool isLow = p.isLowStock;
    final double progress = p.stockProgress;
    final Color stockColor = progress < 0.3
        ? cs.error
        : progress < 0.6
            ? const Color(0xFFF59E0B)
            : const Color(0xFF10B981);

    final String? catPath = widget.categoryController != null &&
            p.categoryId != null
        ? widget.categoryController!.buildPath(p.categoryId)
        : null;

    return Scaffold(
      backgroundColor: cs.surfaceContainerLowest,
      appBar: AppBar(
        title: const Text('商品详情'),
        actions: <Widget>[
          if (canWrite)
            IconButton(
              icon: const Icon(Icons.edit_rounded),
              tooltip: '编辑',
              onPressed: _openEdit,
            ),
          if (canWrite)
            IconButton(
              icon: Icon(Icons.delete_outline_rounded, color: cs.error),
              tooltip: '删除',
              onPressed: _confirmDelete,
            ),
          const SizedBox(width: 4),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: <Widget>[
          // ── Hero 名称卡 ───────────────────────────────────────────────
          _HeroCard(product: p, catPath: catPath, cs: cs),
          const SizedBox(height: 12),

          // ── 库存状态卡 ────────────────────────────────────────────────
          _SectionCard(
            title: '库存状态',
            icon: Icons.inventory_2_rounded,
            iconColor: isLow ? cs.error : const Color(0xFF10B981),
            child: Column(
              children: <Widget>[
                const SizedBox(height: 4),
                // 进度条
                ClipRRect(
                  borderRadius: BorderRadius.circular(4),
                  child: LinearProgressIndicator(
                    value: progress,
                    minHeight: 8,
                    backgroundColor: cs.surfaceContainerHighest,
                    valueColor: AlwaysStoppedAnimation<Color>(stockColor),
                  ),
                ),
                const SizedBox(height: 12),
                Row(
                  children: <Widget>[
                    _StatBox(
                      label: '当前库存',
                      value: '${p.currentStock}',
                      unit: p.unit,
                      color: stockColor,
                    ),
                    const SizedBox(width: 10),
                    _StatBox(
                      label: '预警阈值',
                      value: '${p.minStockLimit}',
                      unit: p.unit,
                      color: cs.onSurfaceVariant,
                    ),
                    if (isLow) ...<Widget>[
                      const SizedBox(width: 10),
                      _StatBox(
                        label: '缺货数量',
                        value: '${p.shortage}',
                        unit: p.unit,
                        color: cs.error,
                        highlight: true,
                      ),
                    ],
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),

          // ── 价格信息卡 ────────────────────────────────────────────────
          if (canViewCost)
            _SectionCard(
              title: '价格信息',
              icon: Icons.payments_rounded,
              iconColor: const Color(0xFF8B5CF6),
              child: Column(
                children: <Widget>[
                  const SizedBox(height: 4),
                  Row(
                    children: <Widget>[
                      _StatBox(
                        label: '零售价',
                        value: '¥${p.retailPrice}',
                        color: const Color(0xFF10B981),
                      ),
                      const SizedBox(width: 10),
                      _StatBox(
                        label: '成本价',
                        value: p.costPrice != null ? '¥${p.costPrice}' : '-',
                        color: cs.onSurfaceVariant,
                      ),
                      const SizedBox(width: 10),
                      _StatBox(
                        label: '毛利率',
                        value: p.grossMarginStr ?? '-',
                        color: const Color(0xFF8B5CF6),
                      ),
                    ],
                  ),
                  if (p.lastInboundUnitCost != null) ...<Widget>[
                    const SizedBox(height: 10),
                    Row(
                      children: <Widget>[
                        Icon(Icons.local_shipping_outlined,
                            size: 13, color: cs.onSurfaceVariant),
                        const SizedBox(width: 5),
                        Text(
                          '最近入库单价：¥${p.lastInboundUnitCost}',
                          style: TextStyle(
                              fontSize: 12, color: cs.onSurfaceVariant),
                        ),
                      ],
                    ),
                  ],
                ],
              ),
            ),
          if (canViewCost) const SizedBox(height: 12),

          // ── 基本信息卡 ────────────────────────────────────────────────
          _SectionCard(
            title: '基本信息',
            icon: Icons.info_outline_rounded,
            iconColor: const Color(0xFF3B82F6),
            child: Column(
              children: <Widget>[
                const SizedBox(height: 4),
                _InfoRow(label: 'SKU', value: p.sku.isEmpty ? '-' : p.sku),
                _InfoRow(
                    label: '条码',
                    value: p.barcode.isEmpty ? '-' : p.barcode,
                    copyable: p.barcode.isNotEmpty),
                _InfoRow(label: '单位', value: p.unit),
                if (catPath != null)
                  _InfoRow(label: '分类', value: catPath),
                _InfoRow(label: '商品 ID', value: '#${p.id}'),
                // 功能标签
                const SizedBox(height: 8),
                Wrap(
                  spacing: 6,
                  runSpacing: 6,
                  children: <Widget>[
                    _FeatureChip(
                      label: '批次追踪',
                      icon: Icons.layers_outlined,
                      active: p.trackBatches,
                    ),
                    _FeatureChip(
                      label: '流水码追踪',
                      icon: Icons.qr_code_2_rounded,
                      active: p.trackSerials,
                    ),
                  ],
                ),
              ],
            ),
          ),
          const SizedBox(height: 12),

          // ── 操作区 ────────────────────────────────────────────────────
          _SectionCard(
            title: '操作',
            icon: Icons.settings_rounded,
            iconColor: const Color(0xFF64748B),
            child: Column(
              children: <Widget>[
                const SizedBox(height: 4),
                if (canWrite)
                  _ActionTile(
                    icon: Icons.edit_outlined,
                    label: '编辑商品信息',
                    subtitle: '修改名称、价格、库存预警等基本信息',
                    color: const Color(0xFF3B82F6),
                    onTap: _openEdit,
                  ),
                if (widget.batchController != null && p.trackBatches)
                  _ActionTile(
                    icon: Icons.layers_outlined,
                    label: '批次管理',
                    subtitle: '查看、创建和管理商品批次',
                    color: const Color(0xFFF59E0B),
                    onTap: () => Navigator.of(context).push(
                      MaterialPageRoute<void>(
                        builder: (_) => BatchManagementPage(
                          productId: p.id,
                          productName: p.name,
                          controller: widget.batchController!,
                        ),
                      ),
                    ),
                  ),
                if (widget.onStockCheck != null)
                  _ActionTile(
                    icon: Icons.fact_check_outlined,
                    label: '发起盘点',
                    subtitle: '对该商品当前库存进行盘点',
                    color: const Color(0xFF10B981),
                    onTap: () {
                      widget.onStockCheck!(p.id);
                      Navigator.of(context).pop();
                    },
                  ),
                if (canWrite)
                  _ActionTile(
                    icon: Icons.delete_outline_rounded,
                    label: '删除商品',
                    subtitle: '永久删除，操作不可撤销',
                    color: cs.error,
                    onTap: _confirmDelete,
                    isDestructive: true,
                  ),
              ],
            ),
          ),
          const SizedBox(height: 24),
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Sub-widgets
// ════════════════════════════════════════════════════════════════════════════

class _HeroCard extends StatelessWidget {
  const _HeroCard({
    required this.product,
    required this.catPath,
    required this.cs,
  });

  final ProductData product;
  final String? catPath;
  final ColorScheme cs;

  @override
  Widget build(BuildContext context) {
    final bool isLow = product.isLowStock;
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        gradient: LinearGradient(
          colors: <Color>[
            isLow
                ? cs.error.withValues(alpha: 0.10)
                : const Color(0xFF3B82F6).withValues(alpha: 0.08),
            isLow
                ? cs.error.withValues(alpha: 0.04)
                : const Color(0xFF8B5CF6).withValues(alpha: 0.06),
          ],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: isLow
              ? cs.error.withValues(alpha: 0.3)
              : const Color(0xFF3B82F6).withValues(alpha: 0.2),
        ),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          // 商品图标
          Container(
            width: 52,
            height: 52,
            decoration: BoxDecoration(
              color: isLow
                  ? cs.error.withValues(alpha: 0.1)
                  : const Color(0xFF3B82F6).withValues(alpha: 0.1),
              borderRadius: BorderRadius.circular(14),
            ),
            child: Icon(
              Icons.inventory_2_outlined,
              size: 28,
              color:
                  isLow ? cs.error : const Color(0xFF3B82F6),
            ),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  product.name,
                  style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w800,
                    height: 1.2,
                  ),
                ),
                const SizedBox(height: 4),
                if (product.sku.isNotEmpty)
                  Text(
                    'SKU：${product.sku}',
                    style: TextStyle(
                        fontSize: 12, color: cs.onSurfaceVariant),
                  ),
                if (catPath != null) ...<Widget>[
                  const SizedBox(height: 3),
                  Row(
                    children: <Widget>[
                      Icon(Icons.category_outlined,
                          size: 11, color: cs.primary),
                      const SizedBox(width: 3),
                      Expanded(
                        child: Text(
                          catPath!,
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
                if (isLow) ...<Widget>[
                  const SizedBox(height: 6),
                  Container(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 8, vertical: 3),
                    decoration: BoxDecoration(
                      color: cs.error.withValues(alpha: 0.12),
                      borderRadius: BorderRadius.circular(999),
                    ),
                    child: Text(
                      '⚠ 库存预警：差 ${product.shortage} ${product.unit}',
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.w700,
                        color: cs.error,
                      ),
                    ),
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────

class _SectionCard extends StatelessWidget {
  const _SectionCard({
    required this.title,
    required this.icon,
    required this.iconColor,
    required this.child,
  });

  final String title;
  final IconData icon;
  final Color iconColor;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.fromLTRB(16, 14, 16, 16),
      decoration: BoxDecoration(
        color: cs.surface,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: cs.outlineVariant),
        boxShadow: <BoxShadow>[
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.03),
            blurRadius: 6,
            offset: const Offset(0, 2),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            children: <Widget>[
              Icon(icon, size: 16, color: iconColor),
              const SizedBox(width: 6),
              Text(
                title,
                style: TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w700,
                  color: cs.onSurfaceVariant,
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),
          Divider(height: 1, color: cs.outlineVariant.withValues(alpha: 0.6)),
          child,
        ],
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────

class _StatBox extends StatelessWidget {
  const _StatBox({
    required this.label,
    required this.value,
    this.unit,
    required this.color,
    this.highlight = false,
  });

  final String label;
  final String value;
  final String? unit;
  final Color color;
  final bool highlight;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    return Expanded(
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        decoration: BoxDecoration(
          color: highlight
              ? color.withValues(alpha: 0.08)
              : cs.surfaceContainerLowest,
          borderRadius: BorderRadius.circular(10),
          border: highlight
              ? Border.all(color: color.withValues(alpha: 0.3))
              : null,
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(label,
                style: TextStyle(fontSize: 10, color: cs.onSurfaceVariant)),
            const SizedBox(height: 4),
            Text(
              value,
              style: TextStyle(
                fontSize: 18,
                fontWeight: FontWeight.w800,
                color: color,
                letterSpacing: -0.5,
              ),
            ),
            if (unit != null)
              Text(unit!,
                  style:
                      TextStyle(fontSize: 10, color: cs.onSurfaceVariant)),
          ],
        ),
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────

class _InfoRow extends StatelessWidget {
  const _InfoRow({
    required this.label,
    required this.value,
    this.copyable = false,
  });

  final String label;
  final String value;
  final bool copyable;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 5),
      child: Row(
        children: <Widget>[
          SizedBox(
            width: 60,
            child: Text(
              label,
              style:
                  TextStyle(fontSize: 12, color: cs.onSurfaceVariant),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: const TextStyle(
                  fontSize: 13, fontWeight: FontWeight.w500),
              overflow: TextOverflow.ellipsis,
            ),
          ),
          if (copyable)
            GestureDetector(
              onTap: () {
                Clipboard.setData(ClipboardData(text: value));
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(
                    content: Text('已复制到剪贴板'),
                    duration: Duration(seconds: 1),
                  ),
                );
              },
              child: Padding(
                padding: const EdgeInsets.only(left: 6),
                child: Icon(Icons.copy_all_rounded,
                    size: 14, color: cs.onSurfaceVariant),
              ),
            ),
        ],
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────

class _FeatureChip extends StatelessWidget {
  const _FeatureChip({
    required this.label,
    required this.icon,
    required this.active,
  });

  final String label;
  final IconData icon;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    final Color color =
        active ? cs.primary : cs.onSurfaceVariant.withValues(alpha: 0.4);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: active
            ? cs.primaryContainer
            : cs.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(999),
        border: Border.all(
          color: active
              ? cs.primary.withValues(alpha: 0.4)
              : cs.outlineVariant,
        ),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(icon, size: 12, color: color),
          const SizedBox(width: 4),
          Text(
            label,
            style: TextStyle(
              fontSize: 11,
              fontWeight: FontWeight.w600,
              color: color,
            ),
          ),
          const SizedBox(width: 4),
          Icon(
            active ? Icons.check_circle_rounded : Icons.cancel_rounded,
            size: 11,
            color: color,
          ),
        ],
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────

class _ActionTile extends StatelessWidget {
  const _ActionTile({
    required this.icon,
    required this.label,
    required this.subtitle,
    required this.color,
    required this.onTap,
    this.isDestructive = false,
  });

  final IconData icon;
  final String label;
  final String subtitle;
  final Color color;
  final VoidCallback onTap;
  final bool isDestructive;

  @override
  Widget build(BuildContext context) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(10),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          decoration: BoxDecoration(
            color: isDestructive
                ? color.withValues(alpha: 0.06)
                : cs.surfaceContainerLowest,
            borderRadius: BorderRadius.circular(10),
            border: Border.all(
              color: isDestructive
                  ? color.withValues(alpha: 0.25)
                  : cs.outlineVariant,
            ),
          ),
          child: Row(
            children: <Widget>[
              Container(
                width: 36,
                height: 36,
                decoration: BoxDecoration(
                  color: color.withValues(alpha: 0.12),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(icon, size: 18, color: color),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      label,
                      style: TextStyle(
                        fontSize: 14,
                        fontWeight: FontWeight.w600,
                        color: isDestructive ? color : cs.onSurface,
                      ),
                    ),
                    Text(
                      subtitle,
                      style: TextStyle(
                          fontSize: 11, color: cs.onSurfaceVariant),
                    ),
                  ],
                ),
              ),
              Icon(Icons.chevron_right_rounded,
                  size: 18, color: cs.onSurfaceVariant),
            ],
          ),
        ),
      ),
    );
  }
}
