import 'package:flutter/material.dart';

import '../../../core/widgets/brand_ui.dart';
import '../application/stock_check_logs_controller.dart';
import '../models/inventory_models.dart';

// ── biz_type 中文映射 ────────────────────────────────────────────────────────

const Map<String, String> _bizTypeLabel = {
  'ADJ_CHECK': '盘点调整',
  'IN_PURCHASE': '采购入库',
  'IN_RETURN': '销售退货',
  'OUT_SALE': '销售出库',
  'OUT_ADJUST': '手动调减',
  'IN_ADJUST': '手动调增',
  'IN_INIT': '初始入库',
};

const Map<String, Color> _bizTypeColor = {
  'ADJ_CHECK': Color(0xFF8B5CF6),
  'IN_PURCHASE': Color(0xFF10B981),
  'IN_RETURN': Color(0xFFF59E0B),
  'OUT_SALE': Color(0xFF3B82F6),
  'OUT_ADJUST': Color(0xFFEF4444),
  'IN_ADJUST': Color(0xFF10B981),
  'IN_INIT': Color(0xFF6B7280),
};

String _labelOf(String raw) => _bizTypeLabel[raw] ?? raw;
Color _colorOf(String raw) => _bizTypeColor[raw] ?? const Color(0xFF6B7280);

// ── 时间格式化 ───────────────────────────────────────────────────────────────

String _fmtTime(String raw) {
  try {
    final dt = DateTime.parse(raw).toLocal();
    final y = dt.year;
    final mo = dt.month.toString().padLeft(2, '0');
    final d = dt.day.toString().padLeft(2, '0');
    final h = dt.hour.toString().padLeft(2, '0');
    final mi = dt.minute.toString().padLeft(2, '0');
    return '$y/$mo/$d $h:$mi';
  } catch (_) {
    return raw;
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Page
// ════════════════════════════════════════════════════════════════════════════

class StockCheckLogsPage extends StatefulWidget {
  const StockCheckLogsPage({
    super.key,
    required this.controller,
    required this.initialStartDate,
    required this.initialEndDate,
    required this.initialPageSize,
  });

  final StockCheckLogsController controller;
  final String initialStartDate;
  final String initialEndDate;
  final int initialPageSize;

  @override
  State<StockCheckLogsPage> createState() => _StockCheckLogsPageState();
}

class _StockCheckLogsPageState extends State<StockCheckLogsPage> {
  static final RegExp _datePattern = RegExp(r'^\d{4}-\d{2}-\d{2}$');

  // Selected page size index: 0=10, 1=20, 2=50
  static const List<int> _pageSizeOptions = <int>[10, 20, 50];
  late int _selectedPageSizeIdx;

  DateTime _startDate = DateUtils.dateOnly(DateTime.now());
  DateTime _endDate = DateUtils.dateOnly(DateTime.now());

  @override
  void initState() {
    super.initState();
    _startDate = _parseDateOrToday(widget.initialStartDate);
    _endDate = _parseDateOrToday(widget.initialEndDate);

    final initSize = widget.initialPageSize;
    _selectedPageSizeIdx = _pageSizeOptions.indexOf(initSize);
    if (_selectedPageSizeIdx < 0) _selectedPageSizeIdx = 0;

    WidgetsBinding.instance.addPostFrameCallback((_) => _query(resetPage: true));
  }

  int get _pageSize => _pageSizeOptions[_selectedPageSizeIdx];

  // ── Queries ──────────────────────────────────────────────────────────────

  Future<void> _query({required bool resetPage}) async {
    final data = widget.controller.data;
    final page = resetPage ? 1 : (data?.page ?? 1);
    await widget.controller.load(
      startDate: _fmtDate(_startDate),
      endDate: _fmtDate(_endDate),
      page: page,
      pageSize: _pageSize,
    );
  }

  Future<void> _gotoPage(int page) async {
    await widget.controller.load(
      startDate: _fmtDate(_startDate),
      endDate: _fmtDate(_endDate),
      page: page,
      pageSize: _pageSize,
    );
  }

  // ── Date helpers ─────────────────────────────────────────────────────────

  DateTime _parseDateOrToday(String raw) {
    final t = raw.trim();
    if (!_datePattern.hasMatch(t)) return DateUtils.dateOnly(DateTime.now());
    final parsed = DateTime.tryParse(t);
    if (parsed == null) return DateUtils.dateOnly(DateTime.now());
    return DateUtils.dateOnly(parsed);
  }

  String _fmtDate(DateTime d) {
    final y = d.year.toString().padLeft(4, '0');
    final m = d.month.toString().padLeft(2, '0');
    final day = d.day.toString().padLeft(2, '0');
    return '$y-$m-$day';
  }

  void _applyQuick(DateTime start, DateTime end) {
    setState(() {
      _startDate = start;
      _endDate = end;
    });
    _query(resetPage: true);
  }

  Future<void> _pickStart() async {
    final picked = await showDatePicker(
      context: context,
      initialDate: _startDate,
      firstDate: DateTime(2000),
      lastDate: DateTime(2100),
    );
    if (picked == null || !mounted) return;
    setState(() {
      _startDate = DateUtils.dateOnly(picked);
      if (_endDate.isBefore(_startDate)) _endDate = _startDate;
    });
  }

  Future<void> _pickEnd() async {
    final picked = await showDatePicker(
      context: context,
      initialDate: _endDate,
      firstDate: DateTime(2000),
      lastDate: DateTime(2100),
    );
    if (picked == null || !mounted) return;
    setState(() {
      _endDate = DateUtils.dateOnly(picked);
      if (_startDate.isAfter(_endDate)) _startDate = _endDate;
    });
  }

  // ── Build ─────────────────────────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.controller,
      builder: (BuildContext context, Widget? child) {
        final data = widget.controller.data;
        final currentPage = data?.page ?? 1;
        final total = data?.total ?? 0;
        final pSize = data?.pageSize ?? _pageSize;
        final totalPages = total == 0 ? 1 : ((total + pSize - 1) ~/ pSize);
        final loading = widget.controller.loading;

        return Scaffold(
          appBar: AppBar(
            title: const Text('盘点历史流水'),
            actions: <Widget>[
              IconButton(
                tooltip: '刷新',
                onPressed: loading ? null : () => _query(resetPage: false),
                icon: const Icon(Icons.refresh),
              ),
            ],
          ),
          body: RefreshIndicator(
            onRefresh: () => _query(resetPage: false),
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: <Widget>[
                const BrandHeroBanner(
                  title: '盘点记录查询',
                  subtitle: '查看库存盘点差异流水，支持日期范围筛选',
                  icon: Icons.history_rounded,
                  gradientSeedColor: Color(0xFF8B5CF6),
                ),
                const SizedBox(height: 12),

                // ── Error ──────────────────────────────────────────────────
                if (widget.controller.errorMessage != null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: StatusNotice(
                      message: widget.controller.errorMessage!,
                      tone: NoticeTone.error,
                    ),
                  ),

                // ── Filter Card ────────────────────────────────────────────
                _FilterCard(
                  startDate: _startDate,
                  endDate: _endDate,
                  pageSizeOptions: _pageSizeOptions,
                  selectedIdx: _selectedPageSizeIdx,
                  loading: loading,
                  onPickStart: _pickStart,
                  onPickEnd: _pickEnd,
                  onApplyQuick: _applyQuick,
                  onPageSizeChanged: (idx) {
                    setState(() => _selectedPageSizeIdx = idx);
                    _query(resetPage: true);
                  },
                  onSearch: () => _query(resetPage: true),
                  fmtDate: _fmtDate,
                ),
                const SizedBox(height: 12),

                // ── Summary bar ────────────────────────────────────────────
                if (data != null)
                  _SummaryBar(
                    startDate: _fmtDate(_startDate),
                    endDate: _fmtDate(_endDate),
                    total: total,
                    page: currentPage,
                    totalPages: totalPages,
                  ),
                const SizedBox(height: 8),

                // ── List ───────────────────────────────────────────────────
                if (loading && (data == null || data.list.isEmpty))
                  ..._skeletons(4)
                else if (data == null || data.list.isEmpty)
                  _EmptyState(loading: loading)
                else
                  ...data.list.map(_buildCard),

                const SizedBox(height: 8),

                // ── Pagination ─────────────────────────────────────────────
                if (data != null)
                  _PaginationBar(
                    page: currentPage,
                    totalPages: totalPages,
                    loading: loading,
                    onPrev: currentPage > 1
                        ? () => _gotoPage(currentPage - 1)
                        : null,
                    onNext: currentPage < totalPages
                        ? () => _gotoPage(currentPage + 1)
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

  List<Widget> _skeletons(int n) => List.generate(
        n,
        (_) => const Padding(
          padding: EdgeInsets.only(bottom: 12),
          child: _SkeletonCard(),
        ),
      );

  Widget _buildCard(StockLogData log) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    final bool isGain = log.deltaQty > 0;
    final Color deltaColor =
        isGain ? const Color(0xFF10B981) : const Color(0xFFEF4444);
    final String deltaLabel = isGain ? '盘盈' : '盘亏';
    final String deltaStr =
        isGain ? '+${log.deltaQty}' : '${log.deltaQty}';

    final int stockBefore = log.snapshotStock;
    final int stockAfter = stockBefore + log.deltaQty;

    final Color typeColor = _colorOf(log.bizType);
    final String typeLabel = _labelOf(log.bizType);

    // Product display: name preferred, fallback to ID
    final String productDisplay =
        (log.productName != null && log.productName!.isNotEmpty)
            ? log.productName!
            : 'ID: ${log.productId}';

    // Operator display: name preferred, fallback to short UUID
    final String operatorDisplay =
        (log.operatorName != null && log.operatorName!.isNotEmpty)
            ? log.operatorName!
            : log.operatorId.length > 8
                ? '${log.operatorId.substring(0, 8)}…'
                : log.operatorId;

    return Container(
      margin: const EdgeInsets.only(bottom: 12),
      decoration: BoxDecoration(
        color: cs.surface,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: typeColor.withValues(alpha: 0.25),
        ),
        boxShadow: <BoxShadow>[
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.04),
            blurRadius: 10,
            offset: const Offset(0, 3),
          ),
        ],
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            // ── Row 1: type badge + delta ──────────────────────────────────
            Row(
              children: <Widget>[
                // Type badge
                Container(
                  padding: const EdgeInsets.symmetric(
                      horizontal: 9, vertical: 4),
                  decoration: BoxDecoration(
                    color: typeColor.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(999),
                    border: Border.all(
                        color: typeColor.withValues(alpha: 0.35)),
                  ),
                  child: Text(
                    typeLabel,
                    style: TextStyle(
                      color: typeColor,
                      fontSize: 11,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                const Spacer(),
                // Delta qty — main hero number
                Text(
                  '$deltaLabel $deltaStr',
                  style: TextStyle(
                    fontSize: 20,
                    fontWeight: FontWeight.w900,
                    color: deltaColor,
                    letterSpacing: -0.5,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 10),

            // ── Row 2: product name + biz no ──────────────────────────────
            Row(
              children: <Widget>[
                Icon(Icons.inventory_2_outlined,
                    size: 14, color: cs.onSurfaceVariant),
                const SizedBox(width: 4),
                Expanded(
                  child: Text(
                    productDisplay,
                    style: const TextStyle(
                        fontSize: 14, fontWeight: FontWeight.w700),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              log.bizNo,
              style:
                  TextStyle(fontSize: 11, color: cs.onSurfaceVariant),
            ),
            const SizedBox(height: 10),

            // ── Row 3: stock before → after ───────────────────────────────
            Container(
              padding: const EdgeInsets.symmetric(
                  horizontal: 12, vertical: 8),
              decoration: BoxDecoration(
                color:
                    cs.surfaceContainerHighest.withValues(alpha: 0.5),
                borderRadius: BorderRadius.circular(10),
              ),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: <Widget>[
                  _StockColumn(label: '账面库存', value: stockBefore),
                  Icon(Icons.arrow_forward_rounded,
                      size: 18, color: cs.onSurfaceVariant),
                  _StockColumn(
                    label: '盘后实际',
                    value: stockAfter,
                    highlight: true,
                    color: deltaColor,
                  ),
                ],
              ),
            ),
            const SizedBox(height: 10),

            // ── Row 4: operator + time ─────────────────────────────────────
            Row(
              children: <Widget>[
                Icon(Icons.person_outline,
                    size: 13, color: cs.onSurfaceVariant),
                const SizedBox(width: 4),
                Text(
                  operatorDisplay,
                  style: TextStyle(
                      fontSize: 12, color: cs.onSurfaceVariant),
                ),
                const Spacer(),
                Icon(Icons.access_time_rounded,
                    size: 13, color: cs.onSurfaceVariant),
                const SizedBox(width: 4),
                Text(
                  _fmtTime(log.createdAt),
                  style: TextStyle(
                      fontSize: 12, color: cs.onSurfaceVariant),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Filter Card
// ════════════════════════════════════════════════════════════════════════════

class _FilterCard extends StatelessWidget {
  const _FilterCard({
    required this.startDate,
    required this.endDate,
    required this.pageSizeOptions,
    required this.selectedIdx,
    required this.loading,
    required this.onPickStart,
    required this.onPickEnd,
    required this.onApplyQuick,
    required this.onPageSizeChanged,
    required this.onSearch,
    required this.fmtDate,
  });

  final DateTime startDate;
  final DateTime endDate;
  final List<int> pageSizeOptions;
  final int selectedIdx;
  final bool loading;
  final VoidCallback onPickStart;
  final VoidCallback onPickEnd;
  final void Function(DateTime, DateTime) onApplyQuick;
  final void Function(int) onPageSizeChanged;
  final VoidCallback onSearch;
  final String Function(DateTime) fmtDate;

  @override
  Widget build(BuildContext context) {
    final now = DateUtils.dateOnly(DateTime.now());

    return SectionCard(
      title: '筛选条件',
      subtitle: '支持快捷日期和自定义范围',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          // Quick date chips
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: Row(
              children: <Widget>[
                _QuickChip(
                    label: '今日',
                    onTap: loading
                        ? null
                        : () => onApplyQuick(now, now)),
                _QuickChip(
                    label: '本周',
                    onTap: loading
                        ? null
                        : () {
                            final weekStart = now.subtract(
                                Duration(days: now.weekday - 1));
                            onApplyQuick(weekStart, now);
                          }),
                _QuickChip(
                    label: '本月',
                    onTap: loading
                        ? null
                        : () {
                            final monthStart =
                                DateTime(now.year, now.month, 1);
                            onApplyQuick(monthStart, now);
                          }),
                _QuickChip(
                    label: '近30天',
                    onTap: loading
                        ? null
                        : () => onApplyQuick(
                            now.subtract(const Duration(days: 29)),
                            now)),
              ],
            ),
          ),
          const SizedBox(height: 10),

          // Date pickers
          Row(
            children: <Widget>[
              Expanded(
                child: _DateField(
                  label: '开始日期',
                  value: fmtDate(startDate),
                  onTap: loading ? null : onPickStart,
                ),
              ),
              const Padding(
                padding: EdgeInsets.symmetric(horizontal: 8),
                child: Text('—',
                    style: TextStyle(color: Color(0xFF94A3B8))),
              ),
              Expanded(
                child: _DateField(
                  label: '结束日期',
                  value: fmtDate(endDate),
                  onTap: loading ? null : onPickEnd,
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),

          // Page size + search button
          Row(
            children: <Widget>[
              const Text('每页：',
                  style: TextStyle(fontSize: 13, color: Color(0xFF64748B))),
              const SizedBox(width: 4),
              SegmentedButton<int>(
                segments: pageSizeOptions
                    .asMap()
                    .entries
                    .map((e) => ButtonSegment<int>(
                          value: e.key,
                          label: Text('${e.value}'),
                        ))
                    .toList(),
                selected: <int>{selectedIdx},
                onSelectionChanged: loading
                    ? null
                    : (s) => onPageSizeChanged(s.first),
                style: const ButtonStyle(
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  visualDensity: VisualDensity.compact,
                ),
              ),
              const Spacer(),
              FilledButton.icon(
                onPressed: loading ? null : onSearch,
                icon: const Icon(Icons.search, size: 16),
                label: Text(loading ? '查询中…' : '查询'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _QuickChip extends StatelessWidget {
  const _QuickChip({required this.label, required this.onTap});
  final String label;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(right: 6),
      child: ActionChip(
        label: Text(label,
            style: const TextStyle(fontSize: 12)),
        onPressed: onTap,
        visualDensity: VisualDensity.compact,
        padding: const EdgeInsets.symmetric(horizontal: 4),
      ),
    );
  }
}

class _DateField extends StatelessWidget {
  const _DateField(
      {required this.label, required this.value, required this.onTap});
  final String label;
  final String value;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: InputDecorator(
        decoration: InputDecoration(
          labelText: label,
          border: const OutlineInputBorder(),
          isDense: true,
          suffixIcon: const Icon(Icons.calendar_month_outlined, size: 18),
        ),
        child: Text(value,
            style: const TextStyle(fontSize: 14)),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Summary Bar
// ════════════════════════════════════════════════════════════════════════════

class _SummaryBar extends StatelessWidget {
  const _SummaryBar({
    required this.startDate,
    required this.endDate,
    required this.total,
    required this.page,
    required this.totalPages,
  });

  final String startDate;
  final String endDate;
  final int total;
  final int page;
  final int totalPages;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: cs.surfaceContainerHighest.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(10),
      ),
      child: Row(
        children: <Widget>[
          Icon(Icons.summarize_outlined,
              size: 14, color: cs.onSurfaceVariant),
          const SizedBox(width: 6),
          Expanded(
            child: Text(
              '$startDate ~ $endDate  共 $total 条盘点流水',
              style:
                  TextStyle(fontSize: 12, color: cs.onSurfaceVariant),
            ),
          ),
          Text(
            '$page / $totalPages 页',
            style: const TextStyle(
                fontSize: 12, fontWeight: FontWeight.w600),
          ),
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Stock Column (before → after)
// ════════════════════════════════════════════════════════════════════════════

class _StockColumn extends StatelessWidget {
  const _StockColumn({
    required this.label,
    required this.value,
    this.highlight = false,
    this.color,
  });

  final String label;
  final int value;
  final bool highlight;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: <Widget>[
        Text(
          label,
          style: const TextStyle(
              fontSize: 10, color: Color(0xFF94A3B8)),
        ),
        const SizedBox(height: 2),
        Text(
          '$value',
          style: TextStyle(
            fontSize: 20,
            fontWeight: FontWeight.w800,
            color: highlight
                ? (color ?? Theme.of(context).colorScheme.primary)
                : Theme.of(context).colorScheme.onSurface,
          ),
        ),
      ],
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
  const _EmptyState({required this.loading});
  final bool loading;

  @override
  Widget build(BuildContext context) {
    if (loading) return const SizedBox.shrink();
    return const Padding(
      padding: EdgeInsets.symmetric(vertical: 48),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(Icons.content_paste_off_outlined,
              size: 64, color: Color(0xFF94A3B8)),
          SizedBox(height: 16),
          Text('该时间段内暂无盘点流水',
              style: TextStyle(fontSize: 15, color: Color(0xFF64748B))),
          SizedBox(height: 8),
          Text('请尝试调整日期范围后重新查询',
              style: TextStyle(fontSize: 13, color: Color(0xFF94A3B8))),
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
    final base =
        Theme.of(context).colorScheme.surfaceContainerHighest;
    return Container(
      height: 160,
      decoration: BoxDecoration(
          color: base, borderRadius: BorderRadius.circular(16)),
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(children: <Widget>[
            _Bone(width: 80, height: 22, base: base),
            const Spacer(),
            _Bone(width: 70, height: 22, base: base),
          ]),
          const SizedBox(height: 10),
          _Bone(width: 180, height: 14, base: base),
          const SizedBox(height: 6),
          _Bone(width: 120, height: 12, base: base),
          const SizedBox(height: 10),
          _Bone(width: double.infinity, height: 44, base: base),
        ],
      ),
    );
  }
}

class _Bone extends StatelessWidget {
  const _Bone(
      {required this.width, required this.height, required this.base});
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
