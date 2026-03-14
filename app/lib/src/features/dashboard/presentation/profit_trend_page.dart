import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';

import '../../../core/widgets/brand_ui.dart';
import '../application/trend_controller.dart';
import '../models/dashboard_data.dart';

/// 毛利趋势分析页 — 从"今日毛利润"点击进入
class ProfitTrendPage extends StatefulWidget {
  const ProfitTrendPage({
    super.key,
    required this.controller,
    required this.initialDate,
  });

  final TrendController controller;
  final String initialDate;

  @override
  State<ProfitTrendPage> createState() => _ProfitTrendPageState();
}

class _ProfitTrendPageState extends State<ProfitTrendPage> {
  @override
  void initState() {
    super.initState();
    _load();
  }

  String _startDate(String endDate) {
    final end = DateTime.tryParse(endDate) ?? DateTime.now();
    final start = end.subtract(const Duration(days: 6));
    return '${start.year.toString().padLeft(4, '0')}-'
        '${start.month.toString().padLeft(2, '0')}-'
        '${start.day.toString().padLeft(2, '0')}';
  }

  String _todayString() {
    final now = DateTime.now();
    return '${now.year.toString().padLeft(4, '0')}-'
        '${now.month.toString().padLeft(2, '0')}-'
        '${now.day.toString().padLeft(2, '0')}';
  }

  Future<void> _load() async {
    final end =
        widget.initialDate.isNotEmpty ? widget.initialDate : _todayString();
    await widget.controller.load(
      startDate: _startDate(end),
      endDate: end,
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('毛利趋势分析'),
        actions: <Widget>[
          IconButton(
            icon: const Icon(Icons.refresh),
            tooltip: '刷新',
            onPressed: _load,
          ),
        ],
      ),
      body: AnimatedBuilder(
        animation: widget.controller,
        builder: (BuildContext context, Widget? child) {
          final trend = widget.controller.data;
          final error = widget.controller.errorMessage;
          final loading = widget.controller.loading;

          return RefreshIndicator(
            onRefresh: _load,
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: <Widget>[
                BrandHeroBanner(
                  title: '毛利趋势',
                  subtitle: '近 7 天毛利润与销售额对比',
                  icon: Icons.trending_up_rounded,
                ),
                const SizedBox(height: 16),
                if (loading) ...<Widget>[
                  _Skeleton(height: 96),
                  const SizedBox(height: 12),
                  _Skeleton(height: 240),
                  const SizedBox(height: 12),
                  _Skeleton(height: 72),
                ] else if (error != null) ...<Widget>[
                  StatusNotice(message: error, tone: NoticeTone.error),
                ] else if (trend != null) ...<Widget>[
                  _ProfitSummaryCard(trend: trend),
                  const SizedBox(height: 16),
                  _BarAndLineChartCard(days: trend.days),
                  const SizedBox(height: 16),
                  _GrossProfitNote(),
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
// Sub-widgets for ProfitTrendPage
// ────────────────────────────────────────────────────────────────────────────

class _ProfitSummaryCard extends StatelessWidget {
  const _ProfitSummaryCard({required this.trend});
  final TrendData trend;

  @override
  Widget build(BuildContext context) {
    final today = trend.days.isNotEmpty ? trend.days.last : null;
    final profit = today?.totalGrossProfit ?? '0';
    final sales = today?.totalSalesDouble ?? 0;
    final profitDouble = today?.totalGrossProfitDouble ?? 0;
    final rate =
        sales > 0 ? (profitDouble / sales * 100).toStringAsFixed(1) : '0.0';
    final colorScheme = Theme.of(context).colorScheme;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Row(
          children: <Widget>[
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text('今日毛利润',
                      style: Theme.of(context)
                          .textTheme
                          .bodySmall
                          ?.copyWith(color: colorScheme.onSurfaceVariant)),
                  const SizedBox(height: 4),
                  Text(
                    '¥$profit',
                    style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                          fontWeight: FontWeight.w800,
                          color: const Color(0xFF3B82F6),
                        ),
                  ),
                ],
              ),
            ),
            Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: <Widget>[
                Text('毛利率',
                    style: Theme.of(context)
                        .textTheme
                        .bodySmall
                        ?.copyWith(color: colorScheme.onSurfaceVariant)),
                const SizedBox(height: 4),
                Text(
                  '$rate%',
                  style: Theme.of(context)
                      .textTheme
                      .titleLarge
                      ?.copyWith(fontWeight: FontWeight.w700),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _BarAndLineChartCard extends StatelessWidget {
  const _BarAndLineChartCard({required this.days});
  final List<TrendDayData> days;

  @override
  Widget build(BuildContext context) {
    if (days.isEmpty) return const SizedBox.shrink();

    final maxY =
        days.map((d) => d.totalSalesDouble).reduce((a, b) => a > b ? a : b) *
            1.25;

    final barGroups = <BarChartGroupData>[];
    for (int i = 0; i < days.length; i++) {
      barGroups.add(BarChartGroupData(
        x: i,
        barRods: <BarChartRodData>[
          BarChartRodData(
            toY: days[i].totalGrossProfitDouble,
            color: const Color(0xFF3B82F6),
            width: 14,
            borderRadius: BorderRadius.circular(4),
          ),
        ],
      ));
    }

    final salesSpots = <FlSpot>[];
    for (int i = 0; i < days.length; i++) {
      salesSpots.add(FlSpot(i.toDouble(), days[i].totalSalesDouble));
    }

    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 20, 20, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text('近 7 天毛利 & 销售额',
                style: Theme.of(context)
                    .textTheme
                    .titleSmall
                    ?.copyWith(fontWeight: FontWeight.w700)),
            const SizedBox(height: 8),
            Row(
              children: <Widget>[
                _LegendDot(color: const Color(0xFF3B82F6), label: '毛利润'),
                const SizedBox(width: 16),
                _LegendDot(color: const Color(0xFF10B981), label: '销售额'),
              ],
            ),
            const SizedBox(height: 16),
            SizedBox(
              height: 220,
              child: BarChart(
                BarChartData(
                  maxY: maxY > 0 ? maxY : 100,
                  gridData: FlGridData(
                    show: true,
                    drawVerticalLine: false,
                    getDrawingHorizontalLine: (value) => FlLine(
                      color: Theme.of(context)
                          .colorScheme
                          .outlineVariant
                          .withValues(alpha: 0.5),
                      strokeWidth: 1,
                    ),
                  ),
                  borderData: FlBorderData(show: false),
                  titlesData: FlTitlesData(
                    leftTitles: const AxisTitles(
                        sideTitles: SideTitles(showTitles: false)),
                    rightTitles: const AxisTitles(
                        sideTitles: SideTitles(showTitles: false)),
                    topTitles: const AxisTitles(
                        sideTitles: SideTitles(showTitles: false)),
                    bottomTitles: AxisTitles(
                      sideTitles: SideTitles(
                        showTitles: true,
                        reservedSize: 28,
                        getTitlesWidget: (value, meta) {
                          final idx = value.toInt();
                          if (idx < 0 || idx >= days.length) {
                            return const SizedBox.shrink();
                          }
                          final date = days[idx].date;
                          final label = date.length >= 10
                              ? '${date.substring(5, 7)}/${date.substring(8, 10)}'
                              : date;
                          return Padding(
                            padding: const EdgeInsets.only(top: 6),
                            child: Text(label,
                                style: const TextStyle(fontSize: 10)),
                          );
                        },
                      ),
                    ),
                  ),
                  barGroups: barGroups,
                  // Overlay sales line via extraLinesData is not available in BarChart
                  // Use barTouchData for tooltip
                  barTouchData: BarTouchData(
                    touchTooltipData: BarTouchTooltipData(
                      getTooltipItem: (group, groupIndex, rod, rodIndex) =>
                          BarTooltipItem(
                        '毛利 ¥${rod.toY.toStringAsFixed(2)}\n'
                        '销售 ¥${days[group.x].totalSalesDouble.toStringAsFixed(2)}',
                        const TextStyle(
                            color: Colors.white,
                            fontSize: 12,
                            fontWeight: FontWeight.w600),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _LegendDot extends StatelessWidget {
  const _LegendDot({required this.color, required this.label});
  final Color color;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Container(
          width: 10,
          height: 10,
          decoration: BoxDecoration(color: color, shape: BoxShape.circle),
        ),
        const SizedBox(width: 4),
        Text(label, style: const TextStyle(fontSize: 12)),
      ],
    );
  }
}

class _GrossProfitNote extends StatelessWidget {
  const _GrossProfitNote();

  @override
  Widget build(BuildContext context) {
    return Card(
      color: Theme.of(context).colorScheme.surfaceContainerLowest,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: <Widget>[
            Icon(Icons.info_outline_rounded,
                size: 18,
                color: Theme.of(context).colorScheme.onSurfaceVariant),
            const SizedBox(width: 10),
            Expanded(
              child: Text(
                '口径说明：毛利润 = 销售额 − 销售成本（移动加权平均成本法）',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _Skeleton extends StatelessWidget {
  const _Skeleton({required this.height});
  final double height;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: height,
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(12),
      ),
    );
  }
}
