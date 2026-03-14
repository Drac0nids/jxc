import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';

import '../../../core/widgets/brand_ui.dart';
import '../../../core/models/user_role.dart';
import '../application/trend_controller.dart';
import '../models/dashboard_data.dart';

/// 销售趋势分析页 — 从"今日销售额"点击进入
class SalesTrendPage extends StatefulWidget {
  const SalesTrendPage({
    super.key,
    required this.controller,
    required this.role,
    required this.initialDate,
  });

  final TrendController controller;
  final UserRole role;
  final String initialDate; // 看板当天日期，用作 end_date

  @override
  State<SalesTrendPage> createState() => _SalesTrendPageState();
}

class _SalesTrendPageState extends State<SalesTrendPage> {


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

  Future<void> _load() async {
    final end =
        widget.initialDate.isNotEmpty ? widget.initialDate : _todayString();
    await widget.controller.load(
      startDate: _startDate(end),
      endDate: end,
    );
  }

  String _todayString() {
    final now = DateTime.now();
    return '${now.year.toString().padLeft(4, '0')}-'
        '${now.month.toString().padLeft(2, '0')}-'
        '${now.day.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('销售趋势分析'),
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
                const BrandHeroBanner(
                  title: '销售趋势',
                  subtitle: '近 7 天销售额走势与当日商品构成',
                  icon: Icons.show_chart_rounded,
                ),
                const SizedBox(height: 16),
                if (loading) ...<Widget>[
                  const _SkeletonCard(height: 80),
                  const SizedBox(height: 12),
                  const _SkeletonCard(height: 220),
                ] else if (error != null) ...<Widget>[
                  StatusNotice(message: error, tone: NoticeTone.error),
                ] else if (trend != null) ...<Widget>[
                  _SummaryCard(trend: trend),
                  const SizedBox(height: 16),
                  _BarLineChartCard(days: trend.days),
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
// Sub-widgets for SalesTrendPage
// ────────────────────────────────────────────────────────────────────────────

class _SummaryCard extends StatelessWidget {
  const _SummaryCard({required this.trend});
  final TrendData trend;

  @override
  Widget build(BuildContext context) {
    final today = trend.days.isNotEmpty ? trend.days.last : null;
    final sales = today?.totalSales ?? '0';
    final orders = today?.totalOrders ?? 0;
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
                  Text('今日销售额',
                      style: Theme.of(context)
                          .textTheme
                          .bodySmall
                          ?.copyWith(color: colorScheme.onSurfaceVariant)),
                  const SizedBox(height: 4),
                  Text(
                    '¥$sales',
                    style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                          fontWeight: FontWeight.w800,
                          color: const Color(0xFF10B981),
                        ),
                  ),
                ],
              ),
            ),
            Column(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: <Widget>[
                Text('共 $orders 笔订单',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        )),
                const SizedBox(height: 4),
                const Icon(Icons.receipt_long_rounded,
                    color: Color(0xFF10B981), size: 32),
              ],
            ),
          ],
        ),
      ),
    );
  }
}


class _BarLineChartCard extends StatelessWidget {
  const _BarLineChartCard({required this.days});
  final List<TrendDayData> days;

  static const double _leftReserved = 52.0;
  static const double _rightReserved = 40.0;
  static const double _bottomReserved = 28.0;
  static const double _topReserved = 8.0;

  static String _dateLabel(String date) => date.length >= 10
      ? '${date.substring(5, 7)}/${date.substring(8, 10)}'
      : date;

  static String _salesLabel(double v) {
    if (v >= 10000) return '${(v / 10000).toStringAsFixed(1)}w';
    if (v >= 1000) return '${(v / 1000).toStringAsFixed(1)}k';
    return v.toStringAsFixed(0);
  }

  @override
  Widget build(BuildContext context) {
    if (days.isEmpty) return const SizedBox.shrink();
    final colorScheme = Theme.of(context).colorScheme;

    final maxSales = days
        .map((d) => d.totalSalesDouble)
        .reduce((a, b) => a > b ? a : b);
    final maxSalesY = (maxSales * 1.3).ceilToDouble().clamp(1.0, double.infinity);

    final maxOrders = days
        .map((d) => d.totalOrders.toDouble())
        .reduce((a, b) => a > b ? a : b);
    final maxOrdersY =
        (maxOrders * 1.3).ceilToDouble().clamp(1.0, double.infinity);

    final barGroups = <BarChartGroupData>[
      for (int i = 0; i < days.length; i++)
        BarChartGroupData(
          x: i,
          barRods: [
            BarChartRodData(
              toY: days[i].totalSalesDouble,
              width: 22,
              borderRadius:
                  const BorderRadius.vertical(top: Radius.circular(6)),
              gradient: const LinearGradient(
                colors: [Color(0xFF34D399), Color(0xFF10B981)],
                begin: Alignment.topCenter,
                end: Alignment.bottomCenter,
              ),
              backDrawRodData: BackgroundBarChartRodData(
                show: true,
                toY: maxSalesY,
                color: colorScheme.surfaceContainerHighest
                    .withValues(alpha: 0.45),
              ),
            ),
          ],
        ),
    ];

    final lineSpots = <FlSpot>[
      for (int i = 0; i < days.length; i++)
        FlSpot(i.toDouble(), days[i].totalOrders.toDouble()),
    ];

    return Card(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 20, 16, 12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            // ── 标题 + 图例 ──────────────────────────────────────
            Row(
              children: <Widget>[
                Expanded(
                  child: Text(
                    '销售额 & 订单量对比',
                    style: Theme.of(context)
                        .textTheme
                        .titleSmall
                        ?.copyWith(fontWeight: FontWeight.w700),
                  ),
                ),
                const _LegendBar(
                    color: Color(0xFF10B981), label: '销售额'),
                const SizedBox(width: 14),
                const _LegendLine(
                    color: Color(0xFF3B82F6), label: '订单数'),
              ],
            ),
            const SizedBox(height: 20),
            // ── 图表区域 ─────────────────────────────────────────
            SizedBox(
              height: 220,
              child: Stack(
                children: <Widget>[
                  // 底层：柱状图（销售额，左轴）
                  BarChart(
                    BarChartData(
                      maxY: maxSalesY,
                      barGroups: barGroups,
                      gridData: FlGridData(
                        show: true,
                        drawVerticalLine: false,
                        getDrawingHorizontalLine: (v) => FlLine(
                          color: colorScheme.outlineVariant
                              .withValues(alpha: 0.4),
                          strokeWidth: 1,
                        ),
                      ),
                      borderData: FlBorderData(show: false),
                      titlesData: FlTitlesData(
                        topTitles: const AxisTitles(
                          sideTitles: SideTitles(
                              showTitles: false,
                              reservedSize: _topReserved),
                        ),
                        leftTitles: AxisTitles(
                          sideTitles: SideTitles(
                            showTitles: true,
                            reservedSize: _leftReserved,
                            interval: maxSalesY / 4,
                            getTitlesWidget: (v, meta) {
                              if (v == 0 || v == maxSalesY) {
                                return const SizedBox.shrink();
                              }
                              return Text(
                                _salesLabel(v),
                                style: TextStyle(
                                  fontSize: 10,
                                  color: colorScheme.onSurfaceVariant,
                                ),
                              );
                            },
                          ),
                        ),
                        rightTitles: const AxisTitles(
                          sideTitles: SideTitles(
                              showTitles: false,
                              reservedSize: _rightReserved),
                        ),
                        bottomTitles: AxisTitles(
                          sideTitles: SideTitles(
                            showTitles: true,
                            reservedSize: _bottomReserved,
                            getTitlesWidget: (v, meta) {
                              final idx = v.toInt();
                              if (idx < 0 || idx >= days.length) {
                                return const SizedBox.shrink();
                              }
                              return Padding(
                                padding: const EdgeInsets.only(top: 6),
                                child: Text(
                                  _dateLabel(days[idx].date),
                                  style: TextStyle(
                                      fontSize: 10,
                                      color: colorScheme.onSurfaceVariant),
                                ),
                              );
                            },
                          ),
                        ),
                      ),
                      barTouchData: BarTouchData(
                        touchTooltipData: BarTouchTooltipData(
                          getTooltipColor: (_) =>
                              colorScheme.inverseSurface,
                          getTooltipItem: (group, groupIndex, rod, rodIndex) {
                            final day = days[group.x];
                            return BarTooltipItem(
                              '${_dateLabel(day.date)}\n'
                              '¥${rod.toY.toStringAsFixed(2)}\n'
                              '${day.totalOrders} 笔订单',
                              TextStyle(
                                color: colorScheme.onInverseSurface,
                                fontSize: 12,
                                fontWeight: FontWeight.w600,
                                height: 1.6,
                              ),
                            );
                          },
                        ),
                      ),
                    ),
                  ),
                  // 顶层：折线图（订单数，右轴），禁用触摸事件
                  IgnorePointer(
                    child: LineChart(
                      LineChartData(
                        minX: 0,
                        maxX:
                            (days.length - 1).toDouble().clamp(0.0, 6.0),
                        minY: 0,
                        maxY: maxOrdersY,
                        gridData: const FlGridData(show: false),
                        borderData: FlBorderData(show: false),
                        titlesData: FlTitlesData(
                          topTitles: const AxisTitles(
                            sideTitles: SideTitles(
                                showTitles: false,
                                reservedSize: _topReserved),
                          ),
                          leftTitles: const AxisTitles(
                            sideTitles: SideTitles(
                                showTitles: false,
                                reservedSize: _leftReserved),
                          ),
                          rightTitles: AxisTitles(
                            sideTitles: SideTitles(
                              showTitles: true,
                              reservedSize: _rightReserved,
                              interval: maxOrdersY / 4,
                              getTitlesWidget: (v, meta) {
                                if (v == 0 || v == maxOrdersY) {
                                  return const SizedBox.shrink();
                                }
                                return Padding(
                                  padding: const EdgeInsets.only(left: 6),
                                  child: Text(
                                    v.toInt().toString(),
                                    style: const TextStyle(
                                      fontSize: 10,
                                      color: Color(0xFF3B82F6),
                                    ),
                                  ),
                                );
                              },
                            ),
                          ),
                          bottomTitles: const AxisTitles(
                            sideTitles: SideTitles(
                                showTitles: false,
                                reservedSize: _bottomReserved),
                          ),
                        ),
                        lineBarsData: <LineChartBarData>[
                          LineChartBarData(
                            spots: lineSpots,
                            isCurved: true,
                            curveSmoothness: 0.3,
                            color: const Color(0xFF3B82F6),
                            barWidth: 2.5,
                            dotData: FlDotData(
                              show: true,
                              getDotPainter:
                                  (spot, percent, barData, index) =>
                                      FlDotCirclePainter(
                                radius: 4,
                                color: const Color(0xFF3B82F6),
                                strokeWidth: 2,
                                strokeColor: Colors.white,
                              ),
                            ),
                            belowBarData: BarAreaData(
                              show: true,
                              color: const Color(0xFF3B82F6)
                                  .withValues(alpha: 0.08),
                            ),
                          ),
                        ],
                        lineTouchData:
                            const LineTouchData(enabled: false),
                      ),
                    ),
                  ),
                ],
              ),
            ),
            // ── 轴说明 ───────────────────────────────────────────
            const SizedBox(height: 8),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: <Widget>[
                Text(
                  '← 左轴：销售额（¥）',
                  style: TextStyle(
                      fontSize: 10,
                      color: const Color(0xFF10B981)
                          .withValues(alpha: 0.8)),
                ),
                Text(
                  '右轴：订单数（笔）→',
                  style: TextStyle(
                      fontSize: 10,
                      color: const Color(0xFF3B82F6)
                          .withValues(alpha: 0.8)),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _LegendBar extends StatelessWidget {
  const _LegendBar({required this.color, required this.label});
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
          decoration:
              BoxDecoration(color: color, borderRadius: BorderRadius.circular(3)),
        ),
        const SizedBox(width: 4),
        Text(label, style: const TextStyle(fontSize: 11)),
      ],
    );
  }
}

class _LegendLine extends StatelessWidget {
  const _LegendLine({required this.color, required this.label});
  final Color color;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Container(
          width: 18,
          height: 2.5,
          decoration: BoxDecoration(
            color: color,
            borderRadius: BorderRadius.circular(2),
          ),
        ),
        const SizedBox(width: 4),
        Text(label, style: const TextStyle(fontSize: 11)),
      ],
    );
  }
}

class _SkeletonCard extends StatelessWidget {
  const _SkeletonCard({required this.height});
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
