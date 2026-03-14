import 'package:flutter/material.dart';

import '../../../core/widgets/brand_ui.dart';
import '../application/dashboard_orders_controller.dart';
import '../models/dashboard_data.dart';

class DashboardOrdersPage extends StatefulWidget {
  const DashboardOrdersPage({
    super.key,
    required this.controller,
    required this.initialStartDate,
    required this.initialEndDate,
    required this.initialPageSize,
  });

  final DashboardOrdersController controller;
  final String initialStartDate;
  final String initialEndDate;
  final int initialPageSize;

  @override
  State<DashboardOrdersPage> createState() => _DashboardOrdersPageState();
}

class _DashboardOrdersPageState extends State<DashboardOrdersPage> {
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

  @override
  void dispose() {
    super.dispose();
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

  void _applyQuick(DateTime start, DateTime end) {
    setState(() {
      _startDate = start;
      _endDate = end;
    });
    _query(resetPage: true);
  }

  Future<void> _pickStartDate() async {
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

  Future<void> _pickEndDate() async {
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

  // ── Formatters ────────────────────────────────────────────────────────────

  /// ISO datetime → "MM-DD HH:mm"
  String _formatDateTime(String? raw) {
    if (raw == null || raw.isEmpty) return '-';
    try {
      final dt = DateTime.parse(raw).toLocal();
      return '${dt.month.toString().padLeft(2, '0')}-'
          '${dt.day.toString().padLeft(2, '0')} '
          '${dt.hour.toString().padLeft(2, '0')}:'
          '${dt.minute.toString().padLeft(2, '0')}';
    } catch (_) {
      return raw;
    }
  }

  /// Status code → chinese label
  String _statusLabel(String status) {
    const Map<String, String> labels = <String, String>{
      'CONFIRMED': '已确认',
      'RETURNED': '已退货',
      'OUTBOUND_ONLY': '流水聚合',
      'VOID': '已作废',
      'PENDING': '待确认',
    };
    return labels[status] ?? status;
  }

  Color _statusBg(String status) {
    switch (status) {
      case 'CONFIRMED':
        return const Color(0xFFDCFCE7);
      case 'RETURNED':
        return const Color(0xFFE0F2FE);
      case 'VOID':
        return const Color(0xFFFFE4E6);
      case 'OUTBOUND_ONLY':
        return const Color(0xFFFEF9C3);
      default:
        return const Color(0xFFF1F5F9);
    }
  }

  Color _statusFg(String status) {
    switch (status) {
      case 'CONFIRMED':
        return const Color(0xFF166534);
      case 'RETURNED':
        return const Color(0xFF0C4A6E);
      case 'VOID':
        return const Color(0xFF9F1239);
      case 'OUTBOUND_ONLY':
        return const Color(0xFF92400E);
      default:
        return const Color(0xFF475569);
    }
  }

  // ── Computed total for the current page ──────────────────────────────────

  String _computePageTotal(List<SalesOrderData> orders) {
    double sum = 0;
    for (final o in orders) {
      sum += double.tryParse(o.totalAmount) ?? 0;
    }
    return '¥${sum.toStringAsFixed(2)}';
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
        final totalPages =
            total == 0 ? 1 : ((total + pSize - 1) ~/ pSize);
        final loading = widget.controller.loading;

        return Scaffold(
          appBar: AppBar(
            title: const Text('订单下钻'),
            actions: <Widget>[
              IconButton(
                icon: const Icon(Icons.refresh),
                tooltip: '刷新',
                onPressed: loading ? null : () => _query(resetPage: false),
              ),
            ],
          ),
          body: ListView(
            padding: const EdgeInsets.all(16),
            children: <Widget>[
              // ── Hero Banner ───────────────────────────────────────────────
              const BrandHeroBanner(
                title: '看板订单下钻',
                subtitle: '按日期范围查看订单明细并支持分页查询',
                icon: Icons.receipt_long_rounded,
                gradientSeedColor: Color(0xFF8B5CF6),
              ),
              const SizedBox(height: 16),

              // ── Filter Section ────────────────────────────────────────────
              _FilterSection(
                startDate: _startDate,
                endDate: _endDate,
                pageSizeOptions: _pageSizeOptions,
                selectedPageSizeIdx: _selectedPageSizeIdx,
                loading: loading,
                fmtDate: _fmtDate,
                onPickStart: _pickStartDate,
                onPickEnd: _pickEndDate,
                onApplyQuick: _applyQuick,
                onPageSizeChanged: (idx) {
                  setState(() => _selectedPageSizeIdx = idx);
                  _query(resetPage: true);
                },
                onQuery: () => _query(resetPage: true),
              ),
              const SizedBox(height: 12),

              // ── Error ─────────────────────────────────────────────────────
              if (widget.controller.errorMessage != null)
                Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: StatusNotice(
                    message: widget.controller.errorMessage!,
                    tone: NoticeTone.error,
                  ),
                ),

              // ── Summary Bar ───────────────────────────────────────────────
              if (!loading && data != null && data.list.isNotEmpty)
                _SummaryBar(
                  startDate: data.startDate,
                  endDate: data.endDate,
                  total: data.total,
                  pageTotal: _computePageTotal(data.list),
                  currentPage: currentPage,
                  totalPages: totalPages,
                ),
              if (!loading && data != null && data.list.isNotEmpty)
                const SizedBox(height: 12),

              // ── Orders List ───────────────────────────────────────────────
              if (loading)
                ..._skeletons(4)
              else if (data == null || data.list.isEmpty)
                _EmptyState(hasData: data != null)
              else
                ...data.list.map(
                  (o) => Padding(
                    padding: const EdgeInsets.only(bottom: 12),
                    child: _OrderReceiptCard(
                      order: o,
                      statusLabel: _statusLabel(o.status),
                      statusBg: _statusBg(o.status),
                      statusFg: _statusFg(o.status),
                      formatDateTime: _formatDateTime,
                    ),
                  ),
                ),

              const SizedBox(height: 8),

              // ── Pagination ────────────────────────────────────────────────
              if (!loading && data != null)
                PaginationBar(
                  page: currentPage,
                  totalPages: totalPages,
                  loading: loading,
                  onPrev:
                      currentPage > 1 ? () => _gotoPage(currentPage - 1) : null,
                  onNext: currentPage < totalPages
                      ? () => _gotoPage(currentPage + 1)
                      : null,
                ),
              const SizedBox(height: 16),
            ],
          ),
        );
      },
    );
  }

  List<Widget> _skeletons(int count) {
    return List<Widget>.generate(
      count,
      (_) => const Padding(
        padding: EdgeInsets.only(bottom: 12),
        child: _SkeletonCard(),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Sub-widgets
// ════════════════════════════════════════════════════════════════════════════

class _FilterSection extends StatelessWidget {
  const _FilterSection({
    required this.startDate,
    required this.endDate,
    required this.pageSizeOptions,
    required this.selectedPageSizeIdx,
    required this.loading,
    required this.fmtDate,
    required this.onPickStart,
    required this.onPickEnd,
    required this.onApplyQuick,
    required this.onPageSizeChanged,
    required this.onQuery,
  });

  final DateTime startDate;
  final DateTime endDate;
  final List<int> pageSizeOptions;
  final int selectedPageSizeIdx;
  final bool loading;
  final String Function(DateTime) fmtDate;
  final VoidCallback onPickStart;
  final VoidCallback onPickEnd;
  final void Function(DateTime, DateTime) onApplyQuick;
  final void Function(int) onPageSizeChanged;
  final VoidCallback onQuery;

  @override
  Widget build(BuildContext context) {
    final pageSize = pageSizeOptions[selectedPageSizeIdx];
    return SectionCard(
      title: '查询筛选',
      subtitle: '支持快捷日期和自定义范围',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          // Quick date chips
          QuickDateChipsRow(
            disabled: loading,
            onApply: onApplyQuick,
          ),
          const SizedBox(height: 10),
          // Date range
          Row(
            children: <Widget>[
              Expanded(
                child: DatePickerField(
                  label: '开始日期',
                  value: fmtDate(startDate),
                  onTap: loading ? null : onPickStart,
                ),
              ),
              const Padding(
                padding: EdgeInsets.symmetric(horizontal: 8),
                child: Text('—', style: TextStyle(color: Color(0xFF94A3B8))),
              ),
              Expanded(
                child: DatePickerField(
                  label: '结束日期',
                  value: fmtDate(endDate),
                  onTap: loading ? null : onPickEnd,
                ),
              ),
            ],
          ),
          const SizedBox(height: 10),
          // Page size + query button
          Row(
            children: <Widget>[
              const Text('每页：',
                  style:
                      TextStyle(fontSize: 13, color: Color(0xFF64748B))),
              const SizedBox(width: 4),
              PageSizeSegmented(
                options: pageSizeOptions,
                selected: pageSize,
                disabled: loading,
                onChanged: (v) =>
                    onPageSizeChanged(pageSizeOptions.indexOf(v)),
              ),
              const Spacer(),
              FilledButton.icon(
                onPressed: loading ? null : onQuery,
                icon: const Icon(Icons.search, size: 18),
                label: Text(loading ? '查询中…' : '查询'),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────

class _SummaryBar extends StatelessWidget {
  const _SummaryBar({
    required this.startDate,
    required this.endDate,
    required this.total,
    required this.pageTotal,
    required this.currentPage,
    required this.totalPages,
  });

  final String startDate;
  final String endDate;
  final int total;
  final String pageTotal;
  final int currentPage;
  final int totalPages;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      decoration: BoxDecoration(
        gradient: const LinearGradient(
          colors: <Color>[Color(0xFFEDE9FE), Color(0xFFF5F3FF)],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: const Color(0xFFC4B5FD)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          // Date range and order count
          Row(
            children: <Widget>[
              const Icon(Icons.date_range_rounded,
                  size: 16, color: Color(0xFF7C3AED)),
              const SizedBox(width: 6),
              Text(
                '$startDate ~ $endDate',
                style: const TextStyle(
                    fontSize: 12,
                    color: Color(0xFF6D28D9),
                    fontWeight: FontWeight.w600),
              ),
              const Spacer(),
              Text(
                '共 $total 笔订单',
                style: const TextStyle(fontSize: 12, color: Color(0xFF7C3AED)),
              ),
            ],
          ),
          const SizedBox(height: 8),
          // Page total and page info
          Row(
            children: <Widget>[
              const Icon(Icons.payments_rounded,
                  size: 16, color: Color(0xFF7C3AED)),
              const SizedBox(width: 6),
              Text(
                '本页合计 $pageTotal',
                style: const TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w700,
                  color: Color(0xFF6D28D9),
                ),
              ),
              const Spacer(),
              Text(
                '第 $currentPage / $totalPages 页',
                style: const TextStyle(fontSize: 12, color: Color(0xFF7C3AED)),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────

class _OrderReceiptCard extends StatelessWidget {
  const _OrderReceiptCard({
    required this.order,
    required this.statusLabel,
    required this.statusBg,
    required this.statusFg,
    required this.formatDateTime,
  });

  final SalesOrderData order;
  final String statusLabel;
  final Color statusBg;
  final Color statusFg;
  final String Function(String?) formatDateTime;

  @override
  Widget build(BuildContext context) {
    final ColorScheme scheme = Theme.of(context).colorScheme;
    final double amount = double.tryParse(order.totalAmount) ?? 0;

    return Container(
      width: double.infinity,
      decoration: BoxDecoration(
        color: scheme.surface,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: scheme.outlineVariant),
        boxShadow: <BoxShadow>[
          BoxShadow(
            color: Colors.black.withValues(alpha: 0.04),
            blurRadius: 10,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      child: Column(
        children: <Widget>[
          // ── Header ──────────────────────────────────────────────────────
          Padding(
            padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
            child: Row(
              children: <Widget>[
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        order.bizNo,
                        style: const TextStyle(
                            fontWeight: FontWeight.w800, fontSize: 14),
                        overflow: TextOverflow.ellipsis,
                      ),
                      const SizedBox(height: 2),
                      Text(
                        '#${order.id}',
                        style: TextStyle(
                            fontSize: 11, color: scheme.onSurfaceVariant),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                // Status badge
                Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
                  decoration: BoxDecoration(
                    color: statusBg,
                    borderRadius: BorderRadius.circular(999),
                  ),
                  child: Text(
                    statusLabel,
                    style: TextStyle(
                      color: statusFg,
                      fontWeight: FontWeight.w700,
                      fontSize: 11,
                    ),
                  ),
                ),
              ],
            ),
          ),

          // ── Amount highlight ─────────────────────────────────────────────
          Container(
            margin: const EdgeInsets.symmetric(horizontal: 14),
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            decoration: BoxDecoration(
              color: const Color(0xFF10B981).withValues(alpha: 0.08),
              borderRadius: BorderRadius.circular(10),
            ),
            child: Row(
              children: <Widget>[
                Text(
                  '订单总额',
                  style:
                      TextStyle(fontSize: 12, color: scheme.onSurfaceVariant),
                ),
                const Spacer(),
                Text(
                  '¥${amount.toStringAsFixed(2)}',
                  style: const TextStyle(
                    fontSize: 20,
                    fontWeight: FontWeight.w900,
                    color: Color(0xFF10B981),
                    letterSpacing: -0.5,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 10),

          // ── Items divider ────────────────────────────────────────────────
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14),
            child: Divider(height: 1, color: scheme.outlineVariant),
          ),
          const SizedBox(height: 10),

          // ── Items table ──────────────────────────────────────────────────
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14),
            child: order.items.isEmpty
                ? _EmptyItemsNotice(scheme: scheme)
                : _ItemsTable(items: order.items, scheme: scheme),
          ),
          const SizedBox(height: 10),

          // ── Footer ───────────────────────────────────────────────────────
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14),
            child: Divider(height: 1, color: scheme.outlineVariant),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(14, 8, 14, 12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Row(
                  children: <Widget>[
                    Icon(Icons.access_time_rounded,
                        size: 13, color: scheme.onSurfaceVariant),
                    const SizedBox(width: 4),
                    Text(
                      formatDateTime(order.createdAt),
                      style: TextStyle(
                          fontSize: 12, color: scheme.onSurfaceVariant),
                    ),
                    if (order.confirmedAt != null &&
                        order.confirmedAt!.isNotEmpty) ...<Widget>[
                      const SizedBox(width: 12),
                      Icon(Icons.check_circle_outline_rounded,
                          size: 13, color: scheme.onSurfaceVariant),
                      const SizedBox(width: 4),
                      Text(
                        '确认 ${formatDateTime(order.confirmedAt)}',
                        style: TextStyle(
                            fontSize: 12, color: scheme.onSurfaceVariant),
                      ),
                    ],
                  ],
                ),
                if (_notBlank(order.remark)) ...<Widget>[
                  const SizedBox(height: 5),
                  Row(
                    children: <Widget>[
                      Icon(Icons.notes_rounded,
                          size: 13, color: scheme.onSurfaceVariant),
                      const SizedBox(width: 4),
                      Expanded(
                        child: Text(
                          order.remark!,
                          style: TextStyle(
                              fontSize: 12, color: scheme.onSurfaceVariant),
                          overflow: TextOverflow.ellipsis,
                          maxLines: 2,
                        ),
                      ),
                    ],
                  ),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }

  bool _notBlank(String? v) => v != null && v.trim().isNotEmpty;
}

// ─────────────────────────────────────────────────────────────────────────────

class _ItemsTable extends StatelessWidget {
  const _ItemsTable({required this.items, required this.scheme});

  final List<SalesOrderItemData> items;
  final ColorScheme scheme;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: <Widget>[
        // Header row
        Row(
          children: <Widget>[
            Expanded(
              flex: 5,
              child: Text('商品',
                  style: TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.w700,
                      color: scheme.onSurfaceVariant)),
            ),
            SizedBox(
              width: 44,
              child: Text('数量',
                  textAlign: TextAlign.right,
                  style: TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.w700,
                      color: scheme.onSurfaceVariant)),
            ),
            SizedBox(
              width: 56,
              child: Text('单价',
                  textAlign: TextAlign.right,
                  style: TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.w700,
                      color: scheme.onSurfaceVariant)),
            ),
            SizedBox(
              width: 64,
              child: Text('小计',
                  textAlign: TextAlign.right,
                  style: TextStyle(
                      fontSize: 11,
                      fontWeight: FontWeight.w700,
                      color: scheme.onSurfaceVariant)),
            ),
          ],
        ),
        const SizedBox(height: 6),
        ...items.map((item) => Padding(
              padding: const EdgeInsets.only(top: 6),
              child: _ItemRow(item: item, scheme: scheme),
            )),
      ],
    );
  }
}

class _ItemRow extends StatelessWidget {
  const _ItemRow({required this.item, required this.scheme});

  final SalesOrderItemData item;
  final ColorScheme scheme;

  @override
  Widget build(BuildContext context) {
    final hasReturn = item.returnedQty > 0;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Expanded(
              flex: 5,
              child: Text(
                item.productName,
                style:
                    const TextStyle(fontSize: 13, fontWeight: FontWeight.w500),
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            SizedBox(
              width: 44,
              child: Text(
                '${item.qty}',
                textAlign: TextAlign.right,
                style: const TextStyle(fontSize: 13),
              ),
            ),
            SizedBox(
              width: 56,
              child: Text(
                '¥${item.sellPrice}',
                textAlign: TextAlign.right,
                style: const TextStyle(fontSize: 13),
              ),
            ),
            SizedBox(
              width: 64,
              child: Text(
                '¥${item.lineAmount}',
                textAlign: TextAlign.right,
                style:
                    const TextStyle(fontSize: 13, fontWeight: FontWeight.w700),
              ),
            ),
          ],
        ),
        if (hasReturn)
          Padding(
            padding: const EdgeInsets.only(top: 3),
            child: Row(
              children: <Widget>[
                Icon(Icons.keyboard_return_rounded,
                    size: 12, color: scheme.error),
                const SizedBox(width: 3),
                Text(
                  '已退 ${item.returnedQty} 件',
                  style: TextStyle(fontSize: 11, color: scheme.error),
                ),
              ],
            ),
          ),
      ],
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────

class _EmptyItemsNotice extends StatelessWidget {
  const _EmptyItemsNotice({required this.scheme});
  final ColorScheme scheme;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      decoration: BoxDecoration(
        color: scheme.surfaceContainerHighest.withValues(alpha: 0.6),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Text(
        '无明细（流水聚合行）',
        style: TextStyle(
            fontSize: 12,
            color: scheme.onSurfaceVariant,
            fontWeight: FontWeight.w600),
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────


// ─────────────────────────────────────────────────────────────────────────────

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.hasData});
  final bool hasData;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 48),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(
            hasData ? Icons.receipt_long_outlined : Icons.hourglass_empty,
            size: 64,
            color: const Color(0xFF94A3B8),
          ),
          const SizedBox(height: 16),
          Text(
            hasData ? '当前筛选条件下暂无订单' : '暂无数据，请先查询',
            style: const TextStyle(fontSize: 15, color: Color(0xFF64748B)),
          ),
          const SizedBox(height: 8),
          Text(
            hasData ? '可调整日期范围后重新查询' : '选择日期后点击"查询"按钮',
            style: const TextStyle(fontSize: 13, color: Color(0xFF94A3B8)),
          ),
        ],
      ),
    );
  }
}

// ─────────────────────────────────────────────────────────────────────────────

class _SkeletonCard extends StatelessWidget {
  const _SkeletonCard();

  @override
  Widget build(BuildContext context) {
    final color = Theme.of(context).colorScheme.surfaceContainerHighest;
    return Container(
      height: 172,
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(16),
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            // header skeleton
            Row(
              children: <Widget>[
                _SkeletonBox(width: 140, height: 14, color: color),
                const Spacer(),
                _SkeletonBox(width: 60, height: 20, color: color),
              ],
            ),
            const SizedBox(height: 12),
            _SkeletonBox(width: double.infinity, height: 42, color: color),
            const SizedBox(height: 12),
            _SkeletonBox(width: 200, height: 12, color: color),
            const SizedBox(height: 6),
            _SkeletonBox(width: 140, height: 12, color: color),
          ],
        ),
      ),
    );
  }
}

class _SkeletonBox extends StatelessWidget {
  const _SkeletonBox(
      {required this.width, required this.height, required this.color});
  final double width;
  final double height;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: width,
      height: height,
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.5),
        borderRadius: BorderRadius.circular(6),
      ),
    );
  }
}
