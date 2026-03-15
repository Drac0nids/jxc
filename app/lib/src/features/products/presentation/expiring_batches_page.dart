import 'package:flutter/material.dart';
import '../application/batch_controller.dart';
import '../models/batch_models.dart';
import 'batch_management_page.dart';

/// 临期批次汇总页 — 按商品分组，展示所有 30 天内到期的批次
class ExpiringBatchesPage extends StatelessWidget {
  const ExpiringBatchesPage({super.key, required this.controller});

  final BatchController controller;

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: controller,
      builder: (BuildContext ctx, _) {
        final expiring = controller.expiring;

        // 按 productId 分组
        final Map<int, _ProductGroup> groups = {};
        for (final item in expiring) {
          final pid = item.batch.productId;
          groups.putIfAbsent(
            pid,
            () => _ProductGroup(
              productId: pid,
              productName: item.productName,
              productSku: item.productSku,
              items: [],
            ),
          ).items.add(item);
        }
        // 按最早到期日排序（组内最紧的批次决定组的顺序）
        final sorted = groups.values.toList()
          ..sort((a, b) {
            final aMin = a.items
                .map((e) => e.batch.daysUntilExpiry ?? 9999)
                .reduce((x, y) => x < y ? x : y);
            final bMin = b.items
                .map((e) => e.batch.daysUntilExpiry ?? 9999)
                .reduce((x, y) => x < y ? x : y);
            return aMin.compareTo(bMin);
          });

        return Scaffold(
          appBar: AppBar(
            title: Text('临期批次提醒（${expiring.length}）'),
            actions: <Widget>[
              IconButton(
                icon: const Icon(Icons.refresh),
                tooltip: '刷新',
                onPressed: () => controller.loadExpiring(),
              ),
            ],
          ),
          body: expiring.isEmpty
              ? const _EmptyState()
              : ListView.builder(
                  padding: const EdgeInsets.all(16),
                  itemCount: sorted.length,
                  itemBuilder: (BuildContext ctx2, int i) {
                    return Padding(
                      padding: const EdgeInsets.only(bottom: 12),
                      child: _ProductGroupCard(
                        group: sorted[i],
                        controller: controller,
                      ),
                    );
                  },
                ),
        );
      },
    );
  }
}

// ── 数据结构 ──────────────────────────────────────────────────────────────────

class _ProductGroup {
  _ProductGroup({
    required this.productId,
    required this.productName,
    required this.productSku,
    required this.items,
  });

  final int productId;
  final String productName;
  final String productSku;
  final List<ExpiringBatchData> items;
}

// ── 商品分组卡片 ───────────────────────────────────────────────────────────────

class _ProductGroupCard extends StatelessWidget {
  const _ProductGroupCard({
    required this.group,
    required this.controller,
  });

  final _ProductGroup group;
  final BatchController controller;

  @override
  Widget build(BuildContext context) {
    final ColorScheme scheme = Theme.of(context).colorScheme;
    final int minDays = group.items
        .map((e) => e.batch.daysUntilExpiry ?? 9999)
        .reduce((a, b) => a < b ? a : b);

    final Color urgencyColor = _urgencyColor(minDays);

    return Container(
      decoration: BoxDecoration(
        color: scheme.surface,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: urgencyColor.withValues(alpha: 0.4)),
        boxShadow: <BoxShadow>[
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.04),
            blurRadius: 8,
            offset: const Offset(0, 3),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          // ── 商品标题 ─────────────────────────────────────────────────
          InkWell(
            borderRadius: const BorderRadius.vertical(top: Radius.circular(16)),
            onTap: () => Navigator.of(context).push(
              MaterialPageRoute<void>(
                builder: (_) => BatchManagementPage(
                  productId: group.productId,
                  productName: group.productName,
                  controller: controller,
                ),
              ),
            ),
            child: Padding(
              padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
              child: Row(
                children: <Widget>[
                  Container(
                    width: 8,
                    height: 8,
                    decoration: BoxDecoration(
                      color: urgencyColor,
                      shape: BoxShape.circle,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          group.productName,
                          style: const TextStyle(
                            fontSize: 15,
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        if (group.productSku.isNotEmpty)
                          Text(
                            group.productSku,
                            style: TextStyle(
                              fontSize: 11,
                              color: scheme.onSurfaceVariant,
                            ),
                          ),
                      ],
                    ),
                  ),
                  Text(
                    '${group.items.length} 个批次',
                    style: TextStyle(
                      fontSize: 12,
                      color: scheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(width: 4),
                  Icon(Icons.chevron_right_rounded,
                      size: 18, color: scheme.onSurfaceVariant),
                ],
              ),
            ),
          ),
          // ── 分割线 ───────────────────────────────────────────────────
          Divider(height: 1, color: scheme.outlineVariant),
          // ── 批次列表 ─────────────────────────────────────────────────
          ...group.items.map(
            (item) => _BatchRow(item: item, scheme: scheme),
          ),
          const SizedBox(height: 4),
        ],
      ),
    );
  }

  Color _urgencyColor(int days) {
    if (days <= 0) return const Color(0xFF9333EA); // 已过期 - 紫
    if (days <= 7) return const Color(0xFFEF4444);  // 危急 - 红
    if (days <= 15) return const Color(0xFFF97316); // 警告 - 橙
    return const Color(0xFFF59E0B);                  // 提醒 - 黄
  }
}

// ── 批次行 ─────────────────────────────────────────────────────────────────────

class _BatchRow extends StatelessWidget {
  const _BatchRow({required this.item, required this.scheme});

  final ExpiringBatchData item;
  final ColorScheme scheme;

  @override
  Widget build(BuildContext context) {
    final BatchData batch = item.batch;
    final int days = batch.daysUntilExpiry ?? 0;
    final Color dayColor = _dayColor(days);
    final String dayLabel = days <= 0
        ? '已过期'
        : days == 1
            ? '明天到期'
            : '还剩 $days 天';

    final String? expiresStr = batch.expiresAt != null
        ? '${batch.expiresAt!.year}-'
              '${batch.expiresAt!.month.toString().padLeft(2, '0')}-'
              '${batch.expiresAt!.day.toString().padLeft(2, '0')}'
        : null;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
      child: Row(
        children: <Widget>[
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  batch.lotNumber,
                  style: const TextStyle(
                      fontSize: 13, fontWeight: FontWeight.w600),
                ),
                if (expiresStr != null)
                  Text(
                    '到期日：$expiresStr',
                    style: TextStyle(
                        fontSize: 11, color: scheme.onSurfaceVariant),
                  ),
                if (batch.supplier != null && batch.supplier!.isNotEmpty)
                  Text(
                    '供应商：${batch.supplier}',
                    style: TextStyle(
                        fontSize: 11, color: scheme.onSurfaceVariant),
                  ),
              ],
            ),
          ),
          Container(
            padding:
                const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
            decoration: BoxDecoration(
              color: dayColor.withValues(alpha: 0.12),
              borderRadius: BorderRadius.circular(8),
              border: Border.all(color: dayColor.withValues(alpha: 0.4)),
            ),
            child: Text(
              dayLabel,
              style: TextStyle(
                fontSize: 11,
                fontWeight: FontWeight.w700,
                color: dayColor,
              ),
            ),
          ),
        ],
      ),
    );
  }

  Color _dayColor(int days) {
    if (days <= 0) return const Color(0xFF9333EA);
    if (days <= 7) return const Color(0xFFEF4444);
    if (days <= 15) return const Color(0xFFF97316);
    return const Color(0xFFF59E0B);
  }
}

// ── 空状态 ─────────────────────────────────────────────────────────────────────

class _EmptyState extends StatelessWidget {
  const _EmptyState();

  @override
  Widget build(BuildContext context) {
    return const Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(Icons.check_circle_outline_rounded,
              size: 64, color: Color(0xFF10B981)),
          SizedBox(height: 16),
          Text('30 天内无临期批次',
              style: TextStyle(fontSize: 16, color: Color(0xFF64748B))),
        ],
      ),
    );
  }
}
