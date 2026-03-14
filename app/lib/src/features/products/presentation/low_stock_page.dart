import 'package:flutter/material.dart';

import '../../../core/widgets/brand_ui.dart';
import '../application/low_stock_controller.dart';
import '../models/product_models.dart';

/// 低库存商品下钻页 — 从首页看板"低库存商品数"点击进入
class LowStockPage extends StatefulWidget {
  const LowStockPage({
    super.key,
    required this.controller,
    this.onGoInbound,
  });

  final LowStockController controller;
  final VoidCallback? onGoInbound;

  @override
  State<LowStockPage> createState() => _LowStockPageState();
}

class _LowStockPageState extends State<LowStockPage> {
  @override
  void initState() {
    super.initState();
    widget.controller.load();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('低库存商品'),
      ),
      body: AnimatedBuilder(
        animation: widget.controller,
        builder: (BuildContext context, Widget? child) {
          final loading = widget.controller.loading;
          final error = widget.controller.errorMessage;
          final items = widget.controller.items;
          final total = widget.controller.total;

          return RefreshIndicator(
            onRefresh: widget.controller.load,
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: <Widget>[
                const BrandHeroBanner(
                  title: '低库存预警',
                  subtitle: '以下商品库存低于预警阈值，请及时补货',
                  icon: Icons.warning_amber_rounded,
                ),
                const SizedBox(height: 16),
                if (loading) ...<Widget>[
                  for (int i = 0; i < 5; i++) ...<Widget>[
                    const _SkeletonCard(),
                    const SizedBox(height: 10),
                  ],
                ] else if (error != null) ...<Widget>[
                  StatusNotice(message: error, tone: NoticeTone.error),
                ] else if (items.isEmpty) ...<Widget>[
                  const _EmptyState(),
                ] else ...<Widget>[
                  Row(
                    children: <Widget>[
                      Expanded(child: _SummaryBanner(total: total)),
                      if (widget.onGoInbound != null) ...<Widget>[
                        const SizedBox(width: 10),
                        FilledButton.icon(
                          onPressed: widget.onGoInbound,
                          icon: const Icon(Icons.move_to_inbox_rounded, size: 18),
                          label: const Text('去采购入库'),
                          style: FilledButton.styleFrom(
                            backgroundColor: const Color(0xFF10B981),
                            padding: const EdgeInsets.symmetric(
                                horizontal: 12, vertical: 12),
                          ),
                        ),
                      ],
                    ],
                  ),
                  const SizedBox(height: 12),
                  ...items.map((p) => Padding(
                        padding: const EdgeInsets.only(bottom: 10),
                        child: _LowStockCard(product: p),
                      )),
                ],
              ],
            ),
          );
        },
      ),
    );
  }
}

// ────────────────────────────────────────────────────────────────────────────

class _SummaryBanner extends StatelessWidget {
  const _SummaryBanner({required this.total});
  final int total;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: BoxDecoration(
        color: const Color(0xFFEF4444).withValues(alpha: 0.08),
        borderRadius: BorderRadius.circular(12),
        border:
            Border.all(color: const Color(0xFFEF4444).withValues(alpha: 0.2)),
      ),
      child: Row(
        children: <Widget>[
          const Icon(Icons.inventory_2_outlined,
              color: Color(0xFFEF4444), size: 20),
          const SizedBox(width: 10),
          Text(
            '当前低库存商品共 $total 件',
            style: const TextStyle(
                fontWeight: FontWeight.w600, color: Color(0xFFEF4444)),
          ),
        ],
      ),
    );
  }
}

class _LowStockCard extends StatelessWidget {
  const _LowStockCard({required this.product});
  final ProductData product;

  Color _progressColor(double ratio) {
    if (ratio < 0.3) return const Color(0xFFEF4444); // coral red
    if (ratio < 0.6) return const Color(0xFFF59E0B); // amber orange
    return const Color(0xFF10B981); // emerald green
  }

  @override
  Widget build(BuildContext context) {
    final gap = product.minStockLimit - product.currentStock;
    final ratio = product.minStockLimit > 0
        ? (product.currentStock / product.minStockLimit).clamp(0.0, 1.0)
        : 0.0;
    final progressColor = _progressColor(ratio);
    final colorScheme = Theme.of(context).colorScheme;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            // Title row
            Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    product.name,
                    style: Theme.of(context).textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                  ),
                ),
                const SizedBox(width: 8),
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                  decoration: BoxDecoration(
                    color: progressColor.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    '差 $gap ${product.unit}',
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                      color: progressColor,
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 4),
            // Barcode
            if (product.barcode.isNotEmpty)
              Text(
                product.barcode,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
              ),
            const SizedBox(height: 12),
            // Progress bar
            ClipRRect(
              borderRadius: BorderRadius.circular(4),
              child: LinearProgressIndicator(
                value: ratio,
                minHeight: 8,
                backgroundColor: progressColor.withValues(alpha: 0.15),
                valueColor: AlwaysStoppedAnimation<Color>(progressColor),
              ),
            ),
            const SizedBox(height: 8),
            // Stock vs threshold
            Row(
              children: <Widget>[
                _StockLabel(
                  label: '当前库存',
                  value: '${product.currentStock} ${product.unit}',
                  valueColor: progressColor,
                ),
                const Spacer(),
                _StockLabel(
                  label: '预警阈值',
                  value: '${product.minStockLimit} ${product.unit}',
                  valueColor: colorScheme.onSurface,
                  alignRight: true,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _StockLabel extends StatelessWidget {
  const _StockLabel({
    required this.label,
    required this.value,
    required this.valueColor,
    this.alignRight = false,
  });

  final String label;
  final String value;
  final Color valueColor;
  final bool alignRight;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment:
          alignRight ? CrossAxisAlignment.end : CrossAxisAlignment.start,
      children: <Widget>[
        Text(
          label,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
        ),
        Text(
          value,
          style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                fontWeight: FontWeight.w700,
                color: valueColor,
              ),
        ),
      ],
    );
  }
}

class _EmptyState extends StatelessWidget {
  const _EmptyState();

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 48),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(Icons.check_circle_outline_rounded,
              size: 64, color: const Color(0xFF10B981).withValues(alpha: 0.7)),
          const SizedBox(height: 16),
          Text(
            '当前没有低库存商品，库存充足 🎉',
            style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }
}

class _SkeletonCard extends StatelessWidget {
  const _SkeletonCard();

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 110,
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(12),
      ),
    );
  }
}
