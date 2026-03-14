import 'package:flutter/material.dart';

import '../../../core/widgets/brand_ui.dart';
import '../application/top_sales_controller.dart';
import '../models/dashboard_data.dart';

class TopSalesPage extends StatefulWidget {
  const TopSalesPage({
    super.key,
    required this.controller,
    required this.initialDate,
  });

  final TopSalesController controller;

  /// YYYY-MM-DD, used as both start_date and end_date initially.
  final String initialDate;

  @override
  State<TopSalesPage> createState() => _TopSalesPageState();
}

class _TopSalesPageState extends State<TopSalesPage> {
  late DateTime _startDate;
  late DateTime _endDate;

  @override
  void initState() {
    super.initState();
    final parts = widget.initialDate.split('-');
    final dt = DateTime(
      int.tryParse(parts.elementAtOrNull(0) ?? '') ?? DateTime.now().year,
      int.tryParse(parts.elementAtOrNull(1) ?? '') ?? DateTime.now().month,
      int.tryParse(parts.elementAtOrNull(2) ?? '') ?? DateTime.now().day,
    );
    _startDate = DateUtils.dateOnly(dt);
    _endDate = DateUtils.dateOnly(dt);

    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  String _fmtDate(DateTime d) {
    final y = d.year.toString().padLeft(4, '0');
    final m = d.month.toString().padLeft(2, '0');
    final day = d.day.toString().padLeft(2, '0');
    return '$y-$m-$day';
  }

  String _friendlyRange() {
    final now = DateUtils.dateOnly(DateTime.now());
    final yesterday = now.subtract(const Duration(days: 1));
    if (DateUtils.isSameDay(_startDate, _endDate)) {
      if (DateUtils.isSameDay(_startDate, now)) return '今日';
      if (DateUtils.isSameDay(_startDate, yesterday)) return '昨日';
      return '${_startDate.month}月${_startDate.day}日';
    }
    return '${_startDate.month}/${_startDate.day} - ${_endDate.month}/${_endDate.day}';
  }

  Future<void> _load() async {
    await widget.controller.load(
      startDate: _fmtDate(_startDate),
      endDate: _fmtDate(_endDate),
    );
  }

  void _applyRange(DateTime start, DateTime end) {
    setState(() {
      _startDate = start;
      _endDate = end;
    });
    _load();
  }

  Future<void> _pickDateRange() async {
    final range = await showDateRangePicker(
      context: context,
      firstDate: DateTime(2000),
      lastDate: DateTime(2100),
      initialDateRange: DateTimeRange(start: _startDate, end: _endDate),
    );
    if (range == null || !mounted) return;
    _applyRange(
        DateUtils.dateOnly(range.start), DateUtils.dateOnly(range.end));
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text('热销排行 · ${_friendlyRange()}'),
      ),
      body: AnimatedBuilder(
        animation: widget.controller,
        builder: (BuildContext context, Widget? child) {
          return RefreshIndicator(
            onRefresh: _load,
            child: ListView(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 32),
              children: <Widget>[
                const SizedBox(height: 12),
                // ── 日期筛选 ─────────────────────────────────────────
                _DateFilterBar(
                  startDate: _startDate,
                  endDate: _endDate,
                  loading: widget.controller.loading,
                  onApplyRange: _applyRange,
                  onPickRange: _pickDateRange,
                ),
                const SizedBox(height: 12),

                // ── 排序切换 ─────────────────────────────────────────
                _SortModeBar(
                  current: widget.controller.sortMode,
                  onChanged: widget.controller.setSortMode,
                ),
                const SizedBox(height: 12),

                // ── 错误提示 ─────────────────────────────────────────
                if (widget.controller.errorMessage != null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 12),
                    child: StatusNotice(
                      message: widget.controller.errorMessage!,
                      tone: NoticeTone.error,
                    ),
                  ),

                // ── 汇总条 ──────────────────────────────────────────
                if (!widget.controller.loading &&
                    widget.controller.data != null)
                  _SummaryBar(summary: widget.controller.data!.summary),

                const SizedBox(height: 8),

                // ── 列表 ────────────────────────────────────────────
                if (widget.controller.loading)
                  const _RankingSkeleton()
                else if (widget.controller.data != null) ...<Widget>[
                  if (widget.controller.sortedItems.isEmpty)
                    const Padding(
                      padding: EdgeInsets.symmetric(vertical: 48),
                      child: Center(
                        child: Text('该日期范围内暂无销售数据',
                            style: TextStyle(
                                color: Color(0xFF94A3B8), fontSize: 14)),
                      ),
                    )
                  else
                    ...widget.controller.sortedItems
                        .asMap()
                        .entries
                        .map((entry) => _RankingItem(
                              rank: entry.key + 1,
                              item: entry.value,
                              sortMode: widget.controller.sortMode,
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

// ════════════════════════════════════════════════════════════════════════════
// Date Filter Bar
// ════════════════════════════════════════════════════════════════════════════

class _DateFilterBar extends StatelessWidget {
  const _DateFilterBar({
    required this.startDate,
    required this.endDate,
    required this.loading,
    required this.onApplyRange,
    required this.onPickRange,
  });

  final DateTime startDate;
  final DateTime endDate;
  final bool loading;
  final void Function(DateTime start, DateTime end) onApplyRange;
  final VoidCallback onPickRange;

  @override
  Widget build(BuildContext context) {
    final now = DateUtils.dateOnly(DateTime.now());
    final yesterday = now.subtract(const Duration(days: 1));
    final weekStart = now.subtract(Duration(days: now.weekday - 1));

    return Row(
      children: <Widget>[
        QuickDateChip(
          label: '今日',
          onTap: loading ? null : () => onApplyRange(now, now),
        ),
        QuickDateChip(
          label: '昨日',
          onTap: loading ? null : () => onApplyRange(yesterday, yesterday),
        ),
        QuickDateChip(
          label: '本周',
          onTap: loading ? null : () => onApplyRange(weekStart, now),
        ),
        const Spacer(),
        OutlinedButton.icon(
          onPressed: loading ? null : onPickRange,
          icon: Icon(Icons.date_range_rounded,
              size: 16,
              color: Theme.of(context).colorScheme.onSurfaceVariant),
          label: Text('自定义',
              style: TextStyle(
                  fontSize: 13,
                  color: Theme.of(context).colorScheme.onSurfaceVariant)),
          style: OutlinedButton.styleFrom(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            visualDensity: VisualDensity.compact,
          ),
        ),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Sort Mode Bar
// ════════════════════════════════════════════════════════════════════════════

class _SortModeBar extends StatelessWidget {
  const _SortModeBar({required this.current, required this.onChanged});

  final TopSalesSortMode current;
  final ValueChanged<TopSalesSortMode> onChanged;

  @override
  Widget build(BuildContext context) {
    Widget chip(TopSalesSortMode mode, String label, IconData icon) {
      final selected = current == mode;
      final cs = Theme.of(context).colorScheme;
      return Expanded(
        child: GestureDetector(
          onTap: () => onChanged(mode),
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 200),
            padding: const EdgeInsets.symmetric(vertical: 8),
            decoration: BoxDecoration(
              color: selected
                  ? cs.primary.withValues(alpha: 0.1)
                  : Colors.transparent,
              borderRadius: BorderRadius.circular(10),
              border: Border.all(
                color: selected
                    ? cs.primary.withValues(alpha: 0.4)
                    : cs.outlineVariant.withValues(alpha: 0.5),
              ),
            ),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: <Widget>[
                Icon(icon,
                    size: 15,
                    color: selected ? cs.primary : cs.onSurfaceVariant),
                const SizedBox(width: 4),
                Text(
                  label,
                  style: TextStyle(
                    fontSize: 13,
                    fontWeight: selected ? FontWeight.w700 : FontWeight.w500,
                    color: selected ? cs.primary : cs.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        ),
      );
    }

    return Row(
      children: <Widget>[
        chip(TopSalesSortMode.qty, '按销量', Icons.bar_chart_rounded),
        const SizedBox(width: 8),
        chip(TopSalesSortMode.sales, '按销售额', Icons.payments_rounded),
        const SizedBox(width: 8),
        chip(TopSalesSortMode.profit, '按毛利', Icons.trending_up_rounded),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Summary Bar
// ════════════════════════════════════════════════════════════════════════════

class _SummaryBar extends StatelessWidget {
  const _SummaryBar({required this.summary});
  final SalesReportSummaryData summary;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: BoxDecoration(
        color: cs.surfaceContainerHighest.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(14),
      ),
      child: Row(
        children: <Widget>[
          _SummaryCell(
              label: '总销量', value: '${summary.totalQty} 件'),
          _SummaryCell(
              label: '总销售额',
              value: '¥${summary.totalSalesDouble.toStringAsFixed(2)}'),
          _SummaryCell(
              label: '总毛利',
              value: '¥${summary.totalGrossProfitDouble.toStringAsFixed(2)}'),
        ],
      ),
    );
  }
}

class _SummaryCell extends StatelessWidget {
  const _SummaryCell({required this.label, required this.value});
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Expanded(
      child: Column(
        children: <Widget>[
          Text(label,
              style: TextStyle(
                  fontSize: 11,
                  color: Theme.of(context).colorScheme.onSurfaceVariant)),
          const SizedBox(height: 4),
          Text(value,
              style:
                  const TextStyle(fontWeight: FontWeight.w700, fontSize: 14)),
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Ranking Item
// ════════════════════════════════════════════════════════════════════════════

class _RankingItem extends StatelessWidget {
  const _RankingItem({
    required this.rank,
    required this.item,
    required this.sortMode,
  });

  final int rank;
  final SalesReportItemData item;
  final TopSalesSortMode sortMode;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    // Medal colors for top 3
    final Color? medalColor = switch (rank) {
      1 => const Color(0xFFFFD700),
      2 => const Color(0xFFC0C0C0),
      3 => const Color(0xFFCD7F32),
      _ => null,
    };

    final String medalEmoji = switch (rank) {
      1 => '🥇',
      2 => '🥈',
      3 => '🥉',
      _ => '',
    };

    // Highlight value based on sort mode
    final String highlightLabel = switch (sortMode) {
      TopSalesSortMode.qty => '${item.totalQty} 件',
      TopSalesSortMode.sales =>
        '¥${item.totalSalesDouble.toStringAsFixed(2)}',
      TopSalesSortMode.profit =>
        '¥${item.grossProfitDouble.toStringAsFixed(2)}',
    };

    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      decoration: BoxDecoration(
        color: medalColor != null
            ? medalColor.withValues(alpha: 0.06)
            : cs.surfaceContainerHighest.withValues(alpha: 0.3),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(
          color: medalColor != null
              ? medalColor.withValues(alpha: 0.25)
              : cs.outlineVariant.withValues(alpha: 0.4),
        ),
      ),
      child: Row(
        children: <Widget>[
          // Rank badge
          SizedBox(
            width: 36,
            child: rank <= 3
                ? Text(medalEmoji,
                    style: const TextStyle(fontSize: 22),
                    textAlign: TextAlign.center)
                : Container(
                    width: 28,
                    height: 28,
                    alignment: Alignment.center,
                    decoration: BoxDecoration(
                      color: cs.surfaceContainerHighest,
                      shape: BoxShape.circle,
                    ),
                    child: Text(
                      '$rank',
                      style: TextStyle(
                        fontWeight: FontWeight.w700,
                        fontSize: 12,
                        color: cs.onSurfaceVariant,
                      ),
                    ),
                  ),
          ),
          const SizedBox(width: 10),

          // Product info
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  item.productName,
                  style: const TextStyle(
                      fontWeight: FontWeight.w600, fontSize: 14),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                const SizedBox(height: 4),
                Row(
                  children: <Widget>[
                    _DetailChip(
                        label: '销量',
                        value: '${item.totalQty}',
                        highlighted:
                            sortMode == TopSalesSortMode.qty),
                    const SizedBox(width: 10),
                    _DetailChip(
                        label: '销售额',
                        value:
                            '¥${item.totalSalesDouble.toStringAsFixed(2)}',
                        highlighted:
                            sortMode == TopSalesSortMode.sales),
                    const SizedBox(width: 10),
                    _DetailChip(
                        label: '毛利',
                        value:
                            '¥${item.grossProfitDouble.toStringAsFixed(2)}',
                        highlighted:
                            sortMode == TopSalesSortMode.profit),
                  ],
                ),
              ],
            ),
          ),

          // Highlight value
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
            decoration: BoxDecoration(
              color: (medalColor ?? const Color(0xFF3B82F6))
                  .withValues(alpha: 0.12),
              borderRadius: BorderRadius.circular(999),
            ),
            child: Text(
              highlightLabel,
              style: TextStyle(
                fontWeight: FontWeight.w800,
                fontSize: 13,
                color: medalColor ?? const Color(0xFF3B82F6),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _DetailChip extends StatelessWidget {
  const _DetailChip({
    required this.label,
    required this.value,
    required this.highlighted,
  });

  final String label;
  final String value;
  final bool highlighted;

  @override
  Widget build(BuildContext context) {
    final color = highlighted
        ? Theme.of(context).colorScheme.primary
        : Theme.of(context).colorScheme.onSurfaceVariant;
    return Text(
      '$label $value',
      style: TextStyle(
        fontSize: 11,
        color: color,
        fontWeight: highlighted ? FontWeight.w600 : FontWeight.w400,
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Ranking Skeleton
// ════════════════════════════════════════════════════════════════════════════

class _RankingSkeleton extends StatelessWidget {
  const _RankingSkeleton();

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Column(
      children: List<Widget>.generate(
        5,
        (_) => Container(
          margin: const EdgeInsets.only(bottom: 8),
          height: 72,
          decoration: BoxDecoration(
            color: cs.surfaceContainerHighest.withValues(alpha: 0.4),
            borderRadius: BorderRadius.circular(14),
          ),
        ),
      ),
    );
  }
}
