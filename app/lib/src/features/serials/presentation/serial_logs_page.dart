import 'package:flutter/material.dart';
import '../models/serial_models.dart';
import '../models/serial_repository.dart';

// ════════════════════════════════════════════════════════════════════════════
// SerialLogsPage — SN 入库 / 出库历史记录
// ════════════════════════════════════════════════════════════════════════════

class SerialLogsPage extends StatefulWidget {
  const SerialLogsPage({super.key, required this.repository});
  final SerialRepository repository;

  @override
  State<SerialLogsPage> createState() => _SerialLogsPageState();
}

class _SerialLogsPageState extends State<SerialLogsPage> {
  static const List<_Tab> _tabs = [
    _Tab(label: '全部', status: null),
    _Tab(label: '在库', status: 'IN_STOCK'),
    _Tab(label: '已出库', status: 'SOLD'),
  ];

  int _tabIndex = 0;
  int _page = 1;
  static const int _pageSize = 30;

  bool _loading = false;
  String? _error;
  SerialHistoryResult? _result;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load({bool reset = false}) async {
    if (reset) _page = 1;
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final r = await widget.repository.fetchHistory(
        status: _tabs[_tabIndex].status,
        page: _page,
        pageSize: _pageSize,
      );
      setState(() => _result = r);
    } catch (e) {
      setState(() => _error = e.toString().replaceFirst('Exception: ', ''));
    } finally {
      setState(() => _loading = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return DefaultTabController(
      length: _tabs.length,
      child: Scaffold(
        backgroundColor: cs.surface,
        appBar: AppBar(
          title: const Text('SN 记录'),
          bottom: TabBar(
            tabs: _tabs.map((t) => Tab(text: t.label)).toList(),
            onTap: (i) {
              _tabIndex = i;
              _load(reset: true);
            },
          ),
        ),
        body: _buildBody(cs),
      ),
    );
  }

  Widget _buildBody(ColorScheme cs) {
    if (_loading && _result == null) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_error != null) {
      return Center(
        child: Column(mainAxisSize: MainAxisSize.min, children: [
          Icon(Icons.error_outline, size: 48, color: cs.error),
          const SizedBox(height: 8),
          Text(_error!, textAlign: TextAlign.center),
          const SizedBox(height: 16),
          FilledButton(onPressed: _load, child: const Text('重试')),
        ]),
      );
    }
    final list = _result?.list ?? [];
    if (list.isEmpty) {
      return Center(
        child: Column(mainAxisSize: MainAxisSize.min, children: [
          Icon(Icons.inbox_outlined, size: 64, color: cs.outlineVariant),
          const SizedBox(height: 12),
          Text('暂无记录', style: TextStyle(color: cs.onSurfaceVariant)),
        ]),
      );
    }
    return RefreshIndicator(
      onRefresh: () => _load(reset: true),
      child: ListView.separated(
        padding: const EdgeInsets.symmetric(vertical: 8),
        itemCount: list.length + 1, // +1 for pagination footer
        separatorBuilder: (_, __) =>
            Divider(height: 1, indent: 16, color: cs.outlineVariant),
        itemBuilder: (ctx, i) {
          if (i == list.length) return _buildPagination(cs);
          return _SerialLogTile(sn: list[i], cs: cs);
        },
      ),
    );
  }

  Widget _buildPagination(ColorScheme cs) {
    final total = _result?.total ?? 0;
    final totalPages = (total / _pageSize).ceil();
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 16, horizontal: 16),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          IconButton(
            onPressed: _page > 1
                ? () {
                    _page--;
                    _load();
                  }
                : null,
            icon: const Icon(Icons.chevron_left),
          ),
          Text('$_page / $totalPages  共 $total 条',
              style: TextStyle(fontSize: 13, color: cs.onSurfaceVariant)),
          IconButton(
            onPressed: _page < totalPages
                ? () {
                    _page++;
                    _load();
                  }
                : null,
            icon: const Icon(Icons.chevron_right),
          ),
          if (_loading)
            const SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
        ],
      ),
    );
  }
}

// ── 每条 SN 记录 ──────────────────────────────────────────────────────────────

class _SerialLogTile extends StatelessWidget {
  const _SerialLogTile({required this.sn, required this.cs});
  final SerialNumber sn;
  final ColorScheme cs;

  @override
  Widget build(BuildContext context) {
    final isSold = sn.status == 'SOLD';
    final statusColor = isSold ? const Color(0xFFEC4899) : const Color(0xFF10B981);
    final statusLabel = switch (sn.status) {
      'SOLD' => '已出库',
      'RETURNED' => '已退货',
      _ => '在库',
    };
    final timeLabel = _fmt(sn.updatedAt ?? sn.createdAt);
    final bizNo = isSold ? sn.outboundBizNo : sn.inboundBizNo;

    return ListTile(
      contentPadding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
      leading: Container(
        width: 36,
        height: 36,
        decoration: BoxDecoration(
          color: statusColor.withValues(alpha: 0.12),
          shape: BoxShape.circle,
        ),
        child: Icon(
          isSold ? Icons.local_shipping_rounded : Icons.inbox_rounded,
          size: 18,
          color: statusColor,
        ),
      ),
      title: Text(
        sn.sn,
        style: const TextStyle(
            fontWeight: FontWeight.w600, fontSize: 14, fontFamily: 'monospace'),
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (bizNo != null)
            Text(bizNo,
                style:
                    TextStyle(fontSize: 11, color: cs.onSurfaceVariant)),
          if (sn.unitCost != null && !isSold)
            Text('进价 ¥${sn.unitCost}',
                style:
                    TextStyle(fontSize: 11, color: cs.onSurfaceVariant)),
          if (sn.sellPrice != null && isSold)
            Text('售价 ¥${sn.sellPrice}',
                style:
                    TextStyle(fontSize: 11, color: cs.onSurfaceVariant)),
        ],
      ),
      trailing: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
            decoration: BoxDecoration(
              color: statusColor.withValues(alpha: 0.12),
              borderRadius: BorderRadius.circular(12),
            ),
            child: Text(statusLabel,
                style: TextStyle(
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                    color: statusColor)),
          ),
          const SizedBox(height: 2),
          Text(timeLabel,
              style: TextStyle(fontSize: 10, color: cs.onSurfaceVariant)),
        ],
      ),
    );
  }

  String _fmt(DateTime? dt) {
    if (dt == null) return '';
    final l = dt.toLocal();
    return '${l.month.toString().padLeft(2,'0')}-${l.day.toString().padLeft(2,'0')} ${l.hour.toString().padLeft(2,'0')}:${l.minute.toString().padLeft(2,'0')}';
  }
}

// ── Tab 配置 ─────────────────────────────────────────────────────────────────

class _Tab {
  const _Tab({required this.label, required this.status});
  final String label;
  final String? status;
}
