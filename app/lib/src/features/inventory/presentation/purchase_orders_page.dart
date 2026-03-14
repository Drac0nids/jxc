import 'package:flutter/material.dart';

import '../../../core/widgets/brand_ui.dart';
import '../application/purchase_orders_controller.dart';
import '../models/inventory_models.dart';

class PurchaseOrdersPage extends StatefulWidget {
  const PurchaseOrdersPage({
    super.key,
    required this.controller,
    required this.initialStartDate,
    required this.initialEndDate,
    required this.initialPageSize,
  });

  final PurchaseOrdersController controller;
  final String initialStartDate;
  final String initialEndDate;
  final int initialPageSize;

  @override
  State<PurchaseOrdersPage> createState() => _PurchaseOrdersPageState();
}

class _PurchaseOrdersPageState extends State<PurchaseOrdersPage> {
  static final RegExp _datePattern = RegExp(r'^\d{4}-\d{2}-\d{2}$');
  static const int _defaultPageSize = 10;

  final TextEditingController _startDateController = TextEditingController();
  final TextEditingController _endDateController = TextEditingController();
  final TextEditingController _pageSizeController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _startDateController.text = _normalizeDateOrToday(widget.initialStartDate);
    _endDateController.text = _normalizeDateOrToday(widget.initialEndDate);
    _pageSizeController.text = widget.initialPageSize.toString();
    _query(resetPage: true);
  }

  @override
  void dispose() {
    _startDateController.dispose();
    _endDateController.dispose();
    _pageSizeController.dispose();
    super.dispose();
  }

  Future<void> _query({required bool resetPage}) async {
    final currentData = widget.controller.data;
    final page = resetPage ? 1 : (currentData?.page ?? 1);
    final pageSize = _resolvePageSizeOrDefault(_pageSizeController.text);
    final startDate = _normalizeDateOrToday(_startDateController.text);
    final endDate = _normalizeDateOrToday(_endDateController.text);

    _startDateController.text = startDate;
    _endDateController.text = endDate;

    await widget.controller.load(
      startDate: startDate,
      endDate: endDate,
      page: page,
      pageSize: pageSize,
    );

    final data = widget.controller.data;
    if (data != null) {
      _startDateController.text = _normalizeDateOrToday(data.startDate);
      _endDateController.text = _normalizeDateOrToday(data.endDate);
      _pageSizeController.text = data.pageSize.toString();
    }
  }

  Future<void> _gotoPage(int page) async {
    final pageSize = _resolvePageSizeOrDefault(_pageSizeController.text);
    final startDate = _normalizeDateOrToday(_startDateController.text);
    final endDate = _normalizeDateOrToday(_endDateController.text);

    _startDateController.text = startDate;
    _endDateController.text = endDate;

    await widget.controller.load(
      startDate: startDate,
      endDate: endDate,
      page: page,
      pageSize: pageSize,
    );

    final data = widget.controller.data;
    if (data != null) {
      _startDateController.text = _normalizeDateOrToday(data.startDate);
      _endDateController.text = _normalizeDateOrToday(data.endDate);
      _pageSizeController.text = data.pageSize.toString();
    }
  }

  Future<void> _pickStartDate() async {
    final DateTime initialDate =
        _parseDateStrict(_startDateController.text.trim()) ??
            DateUtils.dateOnly(DateTime.now());
    final DateTime? picked = await showDatePicker(
      context: context,
      initialDate: initialDate,
      firstDate: DateTime(2000, 1, 1),
      lastDate: DateTime(2100, 12, 31),
    );
    if (picked == null || !mounted) {
      return;
    }

    final String formatted = _formatDate(picked);
    setState(() {
      _startDateController.text = formatted;
      final DateTime? end = _parseDateStrict(_endDateController.text.trim());
      if (end != null && end.isBefore(DateUtils.dateOnly(picked))) {
        _endDateController.text = formatted;
      }
    });
  }

  Future<void> _pickEndDate() async {
    final DateTime initialDate =
        _parseDateStrict(_endDateController.text.trim()) ??
            DateUtils.dateOnly(DateTime.now());
    final DateTime? picked = await showDatePicker(
      context: context,
      initialDate: initialDate,
      firstDate: DateTime(2000, 1, 1),
      lastDate: DateTime(2100, 12, 31),
    );
    if (picked == null || !mounted) {
      return;
    }

    final String formatted = _formatDate(picked);
    setState(() {
      _endDateController.text = formatted;
      final DateTime? start =
          _parseDateStrict(_startDateController.text.trim());
      if (start != null && start.isAfter(DateUtils.dateOnly(picked))) {
        _startDateController.text = formatted;
      }
    });
  }

  String _todayString() => _formatDate(DateTime.now());

  String _formatDate(DateTime date) {
    final DateTime day = DateUtils.dateOnly(date);
    final String year = day.year.toString().padLeft(4, '0');
    final String month = day.month.toString().padLeft(2, '0');
    final String dayText = day.day.toString().padLeft(2, '0');
    return '$year-$month-$dayText';
  }

  DateTime? _parseDateStrict(String raw) {
    if (!_datePattern.hasMatch(raw)) {
      return null;
    }

    final DateTime? parsed = DateTime.tryParse(raw);
    if (parsed == null) {
      return null;
    }

    return _formatDate(parsed) == raw ? parsed : null;
  }

  String _normalizeDateOrToday(String raw) {
    final String trimmed = raw.trim();
    if (trimmed.isEmpty) {
      return _todayString();
    }

    final DateTime? parsed = _parseDateStrict(trimmed);
    if (parsed == null) {
      return _todayString();
    }
    return _formatDate(parsed);
  }

  int _resolvePageSizeOrDefault(String raw) {
    final int? parsed = int.tryParse(raw.trim());
    if (parsed == null || parsed <= 0) {
      return _defaultPageSize;
    }
    return parsed;
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.controller,
      builder: (BuildContext context, Widget? child) {
        final data = widget.controller.data;
        final currentPage = data?.page ?? 1;
        final pageSize = data?.pageSize ??
            _resolvePageSizeOrDefault(_pageSizeController.text);
        final total = data?.total ?? 0;
        final totalPages =
            total == 0 ? 1 : ((total + pageSize - 1) / pageSize).floor();

        return Scaffold(
          appBar: AppBar(title: const Text('采购记录')),
          body: ListView(
            padding: const EdgeInsets.all(16),
            children: <Widget>[
              const BrandHeroBanner(
                title: '采购记录查询',
                subtitle: '只读查看采购订单，支持日期范围与分页',
                icon: Icons.receipt_long_rounded,
                gradientSeedColor: Color(0xFF14B8A6),
              ),
              const SizedBox(height: 12),
              SectionCard(
                title: '筛选条件',
                subtitle: '默认当天，每页默认 10 条，可调整',
                child: Column(
                  children: <Widget>[
                    Row(
                      children: <Widget>[
                        Expanded(
                          child: TextField(
                            controller: _startDateController,
                            readOnly: true,
                            onTap: widget.controller.loading
                                ? null
                                : _pickStartDate,
                            decoration: InputDecoration(
                              labelText: '开始日期',
                              border: const OutlineInputBorder(),
                              isDense: true,
                              suffixIcon: IconButton(
                                tooltip: '选择开始日期',
                                onPressed: widget.controller.loading
                                    ? null
                                    : _pickStartDate,
                                icon: const Icon(Icons.calendar_month_outlined),
                              ),
                            ),
                          ),
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: TextField(
                            controller: _endDateController,
                            readOnly: true,
                            onTap:
                                widget.controller.loading ? null : _pickEndDate,
                            decoration: InputDecoration(
                              labelText: '结束日期',
                              border: const OutlineInputBorder(),
                              isDense: true,
                              suffixIcon: IconButton(
                                tooltip: '选择结束日期',
                                onPressed: widget.controller.loading
                                    ? null
                                    : _pickEndDate,
                                icon: const Icon(Icons.calendar_month_outlined),
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),
                    Row(
                      children: <Widget>[
                        SizedBox(
                          width: 140,
                          child: TextField(
                            controller: _pageSizeController,
                            keyboardType: TextInputType.number,
                            decoration: const InputDecoration(
                              labelText: '每页条数',
                              border: OutlineInputBorder(),
                              isDense: true,
                            ),
                          ),
                        ),
                        const SizedBox(width: 8),
                        FilledButton.icon(
                          onPressed: widget.controller.loading
                              ? null
                              : () => _query(resetPage: true),
                          icon: const Icon(Icons.search),
                          label:
                              Text(widget.controller.loading ? '查询中...' : '查询'),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 8),
              if (widget.controller.errorMessage != null)
                StatusNotice(
                  message: widget.controller.errorMessage!,
                  tone: NoticeTone.error,
                ),
              if (data != null)
                Text(
                  '区间：${data.startDate} ~ ${data.endDate}，共 ${data.total} 单，当前第 ${data.page} / $totalPages 页',
                ),
              const SizedBox(height: 8),
              if (widget.controller.loading)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 24),
                  child: Center(child: CircularProgressIndicator()),
                ),
              if (!widget.controller.loading && data != null)
                SectionCard(
                  title: '采购记录列表（只读）',
                  child: Column(
                    children: <Widget>[
                      if (data.list.isEmpty)
                        const Text('当前筛选条件下暂无采购记录')
                      else
                        ...data.list.map(_buildPurchaseOrderCard),
                    ],
                  ),
                ),
              const SizedBox(height: 8),
              Row(
                children: <Widget>[
                  OutlinedButton(
                    onPressed: widget.controller.loading || currentPage <= 1
                        ? null
                        : () => _gotoPage(currentPage - 1),
                    child: const Text('上一页'),
                  ),
                  const SizedBox(width: 8),
                  OutlinedButton(
                    onPressed:
                        widget.controller.loading || currentPage >= totalPages
                            ? null
                            : () => _gotoPage(currentPage + 1),
                    child: const Text('下一页'),
                  ),
                ],
              ),
            ],
          ),
        );
      },
    );
  }

  Widget _buildPurchaseOrderCard(PurchaseOrderData order) {
    final ColorScheme scheme = Theme.of(context).colorScheme;
    return Container(
      width: double.infinity,
      margin: const EdgeInsets.only(bottom: 12),
      decoration: BoxDecoration(
        color: scheme.surface,
        borderRadius: BorderRadius.circular(14),
        border: Border.all(color: scheme.outlineVariant),
      ),
      padding: const EdgeInsets.all(12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      order.bizNo,
                      style: const TextStyle(
                          fontWeight: FontWeight.w800, fontSize: 16),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      '订单ID：#${order.id}  供应商：${order.supplierId?.toString() ?? '-'}',
                      style: TextStyle(
                          fontSize: 12, color: scheme.onSurfaceVariant),
                    ),
                  ],
                ),
              ),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: scheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(999),
                ),
                child: Text(
                  order.status,
                  style: TextStyle(
                    color: scheme.onSurfaceVariant,
                    fontWeight: FontWeight.w700,
                    fontSize: 11,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: <Widget>[
              Text(
                '采购总金额',
                style: TextStyle(color: scheme.onSurfaceVariant, fontSize: 13),
              ),
              Text(
                order.totalAmount,
                style: const TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.w900,
                    fontFamily: 'RobotoMono',
                    letterSpacing: -0.5),
              ),
            ],
          ),
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 8),
            child: Divider(height: 1),
          ),
          if (order.items.isEmpty)
            Text('无采购明细', style: TextStyle(color: scheme.onSurfaceVariant))
          else
            ...order.items.map(
              (item) => Padding(
                padding: const EdgeInsets.only(bottom: 6),
                child: Text(
                  '${item.productName}（ID:${item.productId}） 数量:${item.qty} 单价:${item.unitCost} 小计:${item.lineAmount}',
                  style: const TextStyle(fontSize: 12),
                ),
              ),
            ),
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 8),
            child: Divider(height: 1),
          ),
          Text('版本：${order.version}',
              style: TextStyle(fontSize: 12, color: scheme.onSurfaceVariant)),
          Text('创建：${order.createdAt}',
              style: TextStyle(fontSize: 12, color: scheme.onSurfaceVariant)),
          Text('更新：${order.updatedAt}',
              style: TextStyle(fontSize: 12, color: scheme.onSurfaceVariant)),
          Text('确认：${order.confirmedAt ?? '-'}',
              style: TextStyle(fontSize: 12, color: scheme.onSurfaceVariant)),
          Text('作废：${order.voidedAt ?? '-'}',
              style: TextStyle(fontSize: 12, color: scheme.onSurfaceVariant)),
          if (!_isBlank(order.remark))
            Text('备注：${order.remark}',
                style: TextStyle(fontSize: 12, color: scheme.onSurfaceVariant)),
        ],
      ),
    );
  }

  bool _isBlank(String? value) => value == null || value.trim().isEmpty;
}
