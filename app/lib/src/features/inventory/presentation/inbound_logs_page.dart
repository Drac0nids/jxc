import 'package:flutter/material.dart';

import '../../../core/widgets/brand_ui.dart';
import '../application/inbound_logs_controller.dart';
import '../models/inventory_models.dart';

// ════════════════════════════════════════════════════════════════════════════
// Page
// ════════════════════════════════════════════════════════════════════════════

class InboundLogsPage extends StatefulWidget {
  const InboundLogsPage({
    super.key,
    required this.controller,
    required this.initialStartDate,
    required this.initialEndDate,
    required this.initialPageSize,
  });

  final InboundLogsController controller;
  final String initialStartDate;
  final String initialEndDate;
  final int initialPageSize;

  @override
  State<InboundLogsPage> createState() => _InboundLogsPageState();
}

class _InboundLogsPageState extends State<InboundLogsPage> {
  static final RegExp _datePattern = RegExp(r'^\d{4}-\d{2}-\d{2}$');
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

    WidgetsBinding.instance.addPostFrameCallback(
        (_) => _query(resetPage: true));
  }

  int get _pageSize => _pageSizeOptions[_selectedPageSizeIdx];

  // ── Queries ──────────────────────────────────────────────────────────────

  Future<void> _query({required bool resetPage}) async {
    final page = resetPage ? 1 : (widget.controller.data?.page ?? 1);
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
            title: const Text('入库记录'),
          ),
          body: RefreshIndicator(
            onRefresh: () => _query(resetPage: false),
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: <Widget>[
                const BrandHeroBanner(
                  title: '入库记录查询',
                  subtitle: '查看入库流水，支持日期范围筛选',
                  icon: Icons.inventory_2_rounded,
                  gradientSeedColor: Color(0xFF6366F1),
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
                SectionCard(
                  title: '筛选条件',
                  subtitle: '支持快捷日期和自定义范围',
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      // Quick date chips
                      QuickDateChipsRow(
                        disabled: loading,
                        onApply: _applyQuick,
                      ),
                      const SizedBox(height: 10),

                      // Date pickers
                      Row(
                        children: <Widget>[
                          Expanded(
                            child: DatePickerField(
                              label: '开始日期',
                              value: _fmtDate(_startDate),
                              onTap: loading ? null : _pickStart,
                            ),
                          ),
                          const Padding(
                            padding: EdgeInsets.symmetric(horizontal: 8),
                            child: Text('—',
                                style: TextStyle(color: Color(0xFF94A3B8))),
                          ),
                          Expanded(
                            child: DatePickerField(
                              label: '结束日期',
                              value: _fmtDate(_endDate),
                              onTap: loading ? null : _pickEnd,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 10),

                      // Page size + search
                      Row(
                        children: <Widget>[
                          const Text('每页：',
                              style: TextStyle(
                                  fontSize: 13, color: Color(0xFF64748B))),
                          const SizedBox(width: 4),
                          PageSizeSegmented(
                            options: _pageSizeOptions,
                            selected: _pageSize,
                            disabled: loading,
                            onChanged: (v) {
                              setState(() => _selectedPageSizeIdx =
                                  _pageSizeOptions.indexOf(v));
                              _query(resetPage: true);
                            },
                          ),
                          const Spacer(),
                          FilledButton.icon(
                            onPressed: loading
                                ? null
                                : () => _query(resetPage: true),
                            icon: const Icon(Icons.search, size: 16),
                            label: Text(loading ? '查询中…' : '查询'),
                          ),
                        ],
                      ),
                    ],
                  ),
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
                  ...data.list.map((log) => _buildCard(log, fmtTime: _fmtTime)),

                const SizedBox(height: 8),

                // ── Pagination ─────────────────────────────────────────────
                if (data != null)
                  PaginationBar(
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

  Widget _buildCard(StockLogData log,
      {required String Function(String) fmtTime}) {
    final ColorScheme cs = Theme.of(context).colorScheme;
    final bool isIn = log.deltaQty > 0;
    final Color deltaColor =
        isIn ? const Color(0xFF10B981) : const Color(0xFFEF4444);
    final String deltaStr =
        isIn ? '+${log.deltaQty}' : '${log.deltaQty}';

    // Product display
    final String productDisplay =
        (log.productName != null && log.productName!.isNotEmpty)
            ? log.productName!
            : 'ID: ${log.productId}';

    // Operator display
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
          color: deltaColor.withValues(alpha: 0.25),
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
            // Row 1: quantity badge
            Row(
              children: <Widget>[
                Container(
                  padding: const EdgeInsets.symmetric(
                      horizontal: 9, vertical: 4),
                  decoration: BoxDecoration(
                    color: deltaColor.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(999),
                    border: Border.all(
                        color: deltaColor.withValues(alpha: 0.35)),
                  ),
                  child: Text(
                    isIn ? '入库' : '出库',
                    style: TextStyle(
                      color: deltaColor,
                      fontSize: 11,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                const Spacer(),
                Text(
                  deltaStr,
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

            // Row 2: product + biz no
            Row(children: <Widget>[
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
            ]),
            const SizedBox(height: 4),
            Text(log.bizNo,
                style: TextStyle(
                    fontSize: 11, color: cs.onSurfaceVariant)),
            const SizedBox(height: 10),

            // Row 3: cost info
            Wrap(
              spacing: 16,
              runSpacing: 4,
              children: <Widget>[
                if (log.snapshotInboundUnitCost != null)
                  _MetaChip(
                      label: '本次进货价',
                      value: '¥${log.snapshotInboundUnitCost}'),
                _MetaChip(
                    label: '库存快照', value: '${log.snapshotStock}'),
                _MetaChip(label: '成本快照', value: '¥${log.snapshotCost}'),
              ],
            ),
            const SizedBox(height: 10),

            // Row 4: operator + time
            Row(children: <Widget>[
              Icon(Icons.person_outline,
                  size: 13, color: cs.onSurfaceVariant),
              const SizedBox(width: 4),
              Text(operatorDisplay,
                  style: TextStyle(
                      fontSize: 12, color: cs.onSurfaceVariant)),
              const Spacer(),
              Icon(Icons.access_time_rounded,
                  size: 13, color: cs.onSurfaceVariant),
              const SizedBox(width: 4),
              Text(fmtTime(log.createdAt),
                  style: TextStyle(
                      fontSize: 12, color: cs.onSurfaceVariant)),
            ]),
          ],
        ),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Meta Chip
// ════════════════════════════════════════════════════════════════════════════

class _MetaChip extends StatelessWidget {
  const _MetaChip({required this.label, required this.value});
  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Text('$label：',
            style:
                TextStyle(fontSize: 11, color: cs.onSurfaceVariant)),
        Text(value,
            style: const TextStyle(
                fontSize: 12, fontWeight: FontWeight.w600)),
      ],
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
              '$startDate ~ $endDate  共 $total 条入库记录',
              style:
                  TextStyle(fontSize: 12, color: cs.onSurfaceVariant),
            ),
          ),
          Text('$page / $totalPages 页',
              style: const TextStyle(
                  fontSize: 12, fontWeight: FontWeight.w600)),
        ],
      ),
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
          Icon(Icons.inbox_outlined, size: 64, color: Color(0xFF94A3B8)),
          SizedBox(height: 16),
          Text('该时间段内暂无入库记录',
              style:
                  TextStyle(fontSize: 15, color: Color(0xFF64748B))),
          SizedBox(height: 8),
          Text('请尝试调整日期范围后重新查询',
              style:
                  TextStyle(fontSize: 13, color: Color(0xFF94A3B8))),
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
    final base = Theme.of(context).colorScheme.surfaceContainerHighest;
    return Container(
      height: 140,
      decoration: BoxDecoration(
          color: base, borderRadius: BorderRadius.circular(16)),
      padding: const EdgeInsets.all(14),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(children: <Widget>[
            _Bone(width: 60, height: 22, base: base),
            const Spacer(),
            _Bone(width: 50, height: 22, base: base),
          ]),
          const SizedBox(height: 10),
          _Bone(width: 160, height: 14, base: base),
          const SizedBox(height: 6),
          _Bone(width: 110, height: 12, base: base),
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
