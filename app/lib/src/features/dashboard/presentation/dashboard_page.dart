import 'package:flutter/material.dart';

import '../../auth/application/session_controller.dart';
import '../application/dashboard_controller.dart';
import '../models/dashboard_data.dart';

class DashboardPage extends StatefulWidget {
  const DashboardPage({
    super.key,
    required this.sessionController,
    required this.dashboardController,
  });

  final SessionController sessionController;
  final DashboardController dashboardController;

  @override
  State<DashboardPage> createState() => _DashboardPageState();
}

class _DashboardPageState extends State<DashboardPage> {
  static final RegExp _datePattern = RegExp(r'^\d{4}-\d{2}-\d{2}$');

  final TextEditingController _dateController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _dateController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    final rawDate = _dateController.text.trim();
    if (rawDate.isNotEmpty && !_datePattern.hasMatch(rawDate)) {
      _showMessage('日期格式错误，请输入 YYYY-MM-DD');
      return;
    }

    await widget.dashboardController.load(
      date: rawDate.isEmpty ? null : rawDate,
    );
  }

  void _showMessage(String text) {
    if (!mounted) {
      return;
    }

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(text)),
    );
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: Listenable.merge(<Listenable>[
        widget.sessionController,
        widget.dashboardController,
      ]),
      builder: (BuildContext context, Widget? child) {
        final session = widget.sessionController.session;
        final dashboard = widget.dashboardController.data;

        return Scaffold(
          appBar: AppBar(
            title: const Text('经营看板'),
            actions: <Widget>[
              IconButton(
                tooltip: '刷新',
                onPressed: widget.dashboardController.loading ? null : _load,
                icon: const Icon(Icons.refresh),
              ),
              TextButton.icon(
                onPressed: widget.sessionController.submitting
                    ? null
                    : () async {
                        await widget.sessionController.logout();
                      },
                icon: const Icon(Icons.logout),
                label: Text(widget.sessionController.submitting ? '退出中...' : '退出登录'),
              ),
              const SizedBox(width: 8),
            ],
          ),
          body: RefreshIndicator(
            onRefresh: _load,
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: <Widget>[
                if (session != null)
                  Card(
                    child: ListTile(
                      leading: const Icon(Icons.account_circle_outlined),
                      title: Text('${session.user.name}（${session.user.role.serverValue}）'),
                      subtitle: Text('tenant_id: ${session.tenantId}'),
                    ),
                  ),
                const SizedBox(height: 8),
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        const Text('按日期查询（YYYY-MM-DD，可选）'),
                        const SizedBox(height: 8),
                        Row(
                          children: <Widget>[
                            Expanded(
                              child: TextField(
                                controller: _dateController,
                                decoration: const InputDecoration(
                                  hintText: '例如：2026-03-08',
                                  border: OutlineInputBorder(),
                                  isDense: true,
                                ),
                              ),
                            ),
                            const SizedBox(width: 8),
                            FilledButton(
                              onPressed: widget.dashboardController.loading ? null : _load,
                              child: const Text('查询'),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: 8),
                if (widget.dashboardController.errorMessage != null)
                  Card(
                    color: Theme.of(context).colorScheme.errorContainer,
                    child: Padding(
                      padding: const EdgeInsets.all(12),
                      child: Text(
                        widget.dashboardController.errorMessage!,
                        style: TextStyle(color: Theme.of(context).colorScheme.onErrorContainer),
                      ),
                    ),
                  ),
                if (widget.dashboardController.loading)
                  const Padding(
                    padding: EdgeInsets.symmetric(vertical: 24),
                    child: Center(child: CircularProgressIndicator()),
                  ),
                if (!widget.dashboardController.loading && dashboard != null) ...<Widget>[
                  _MetricCard(title: '今日销售额', value: dashboard.totalSales),
                  _MetricCard(title: '今日毛利润', value: dashboard.totalGrossProfit),
                  _MetricCard(title: '今日订单数', value: '${dashboard.totalOrders}'),
                  _MetricCard(title: '低库存商品数', value: '${dashboard.lowStockCount}'),
                  _MetricCard(title: '热销商品', value: dashboard.topSellingItem),
                ],
              ],
            ),
          ),
        );
      },
    );
  }
}

class _MetricCard extends StatelessWidget {
  const _MetricCard({required this.title, required this.value});

  final String title;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: ListTile(
        title: Text(title),
        trailing: Text(
          value,
          style: Theme.of(context).textTheme.titleMedium,
        ),
      ),
    );
  }
}
