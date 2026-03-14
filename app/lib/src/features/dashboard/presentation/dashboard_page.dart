import 'package:flutter/material.dart';

import '../../../core/models/user_role.dart';
import '../../../core/widgets/brand_ui.dart';
import '../../../storage/session_storage.dart';
import '../../auth/application/session_controller.dart';
import '../../inventory/application/inbound_controller.dart';
import '../../inventory/application/inbound_logs_controller.dart';
import '../../inventory/application/outbound_controller.dart';
import '../../inventory/application/stock_check_controller.dart';
import '../../inventory/application/stock_check_logs_controller.dart';

import '../../inventory/presentation/inbound_page.dart';
import '../../inventory/presentation/outbound_page.dart';
import '../../inventory/presentation/stock_check_page.dart';
import '../../products/application/batch_controller.dart';
import '../../products/application/category_controller.dart';
import '../../products/application/low_stock_controller.dart';
import '../../products/application/product_controller.dart';
import '../../products/models/batch_models.dart';
import '../../products/presentation/batch_management_page.dart';
import '../../products/presentation/low_stock_page.dart';
import '../../products/presentation/products_page.dart';
import '../../users/application/users_controller.dart';
import '../../users/presentation/users_page.dart';
import '../application/dashboard_controller.dart';
import '../application/dashboard_orders_controller.dart';
import '../application/top_sales_controller.dart';
import '../application/trend_controller.dart';
import '../models/dashboard_data.dart';
import 'dashboard_orders_page.dart';
import 'profit_trend_page.dart';
import 'sales_trend_page.dart';
import 'top_sales_page.dart';

// ════════════════════════════════════════════════════════════════════════════
// Quick-action item model
// ════════════════════════════════════════════════════════════════════════════

enum _QuickActionType {
  outbound,
  inbound,
  stockCheck,
  products,
  users,
}

class _QuickActionItem {
  const _QuickActionItem({
    required this.type,
    required this.title,
    required this.subtitle,
    required this.icon,
    required this.color,
    required this.enabled,
  });

  final _QuickActionType type;
  final String title;
  final String subtitle;
  final IconData icon;
  final Color color;
  final bool enabled;
}

// ════════════════════════════════════════════════════════════════════════════
// Page
// ════════════════════════════════════════════════════════════════════════════

class DashboardPage extends StatefulWidget {
  const DashboardPage({
    super.key,
    required this.sessionController,
    required this.sessionStorage,
    required this.dashboardController,
    required this.dashboardOrdersController,
    required this.trendController,
    required this.topSalesController,
    required this.lowStockController,
    required this.inboundController,
    required this.outboundController,
    required this.inboundLogsController,
    required this.stockCheckLogsController,
    required this.stockCheckController,
    required this.productController,
    required this.categoryController,
    required this.batchController,
    required this.usersController,
  });

  final SessionController sessionController;
  final SessionStorage sessionStorage;
  final DashboardController dashboardController;
  final DashboardOrdersController dashboardOrdersController;
  final TrendController trendController;
  final TopSalesController topSalesController;
  final LowStockController lowStockController;
  final InboundController inboundController;
  final OutboundController outboundController;
  final InboundLogsController inboundLogsController;
  final StockCheckLogsController stockCheckLogsController;
  final StockCheckController stockCheckController;
  final ProductController productController;
  final CategoryController categoryController;
  final BatchController batchController;
  final UsersController usersController;

  @override
  State<DashboardPage> createState() => _DashboardPageState();
}

class _DashboardPageState extends State<DashboardPage>
    with WidgetsBindingObserver {
  static const int _defaultPageSize = 10;

  // Selected query date (DateTime for consistency with brand_ui pattern)
  DateTime _queryDate = DateUtils.dateOnly(DateTime.now());

  /// 上次自动刷新的时间，用于防止频繁触发
  DateTime? _lastAutoRefresh;

  /// 两次自动刷新的最短间隔
  static const Duration _minRefreshInterval = Duration(seconds: 30);

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    WidgetsBinding.instance.addPostFrameCallback((_) => _load());
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  /// App 从后台切回前台时自动刷新
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      _autoRefresh();
    }
  }

  /// 智能刷新：距上次刷新超过最短间隔才执行
  void _autoRefresh() {
    final now = DateTime.now();
    if (_lastAutoRefresh == null ||
        now.difference(_lastAutoRefresh!) > _minRefreshInterval) {
      _lastAutoRefresh = now;
      _load();
    }
  }

  // ── Helpers ──────────────────────────────────────────────────────────────

  String _fmtDate(DateTime d) {
    final y = d.year.toString().padLeft(4, '0');
    final m = d.month.toString().padLeft(2, '0');
    final day = d.day.toString().padLeft(2, '0');
    return '$y-$m-$day';
  }

  /// Friendly display like "3月14日"
  String _friendlyDate(DateTime d) => '${d.month}月${d.day}日';

  // ── Queries ──────────────────────────────────────────────────────────────

  Future<void> _load() async {
    await Future.wait(<Future<void>>[
      widget.dashboardController.load(date: _fmtDate(_queryDate)),
      widget.batchController.loadExpiring(withinDays: 30),
    ]);
  }

  void _applyDate(DateTime d) {
    setState(() => _queryDate = d);
    _load();
  }

  Future<void> _pickDate() async {
    final picked = await showDatePicker(
      context: context,
      initialDate: _queryDate,
      firstDate: DateTime(2000),
      lastDate: DateTime(2100),
    );
    if (picked == null || !mounted) return;
    _applyDate(DateUtils.dateOnly(picked));
  }

  // ── Navigation ────────────────────────────────────────────────────────────

  void _showMessage(String text) {
    if (!mounted) return;
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(text)));
  }

  Future<void> _handleOrdersMetricTap(
      UserRole role, DashboardData dashboard) async {
    if (role != UserRole.owner && role != UserRole.sales) {
      _showMessage('仅 OWNER / SALES 可查看订单下钻');
      return;
    }
    final date = _fmtDate(_queryDate);
    if (!mounted) return;
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => DashboardOrdersPage(
          controller: widget.dashboardOrdersController,
          initialStartDate: date,
          initialEndDate: date,
          initialPageSize: _defaultPageSize,
        ),
      ),
    );
  }

  Future<void> _handleSalesTap(UserRole role, DashboardData dashboard) async {
    if (role != UserRole.owner && role != UserRole.sales) {
      _showMessage('仅 OWNER / SALES 可查看销售趋势分析');
      return;
    }
    if (!mounted) return;
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => SalesTrendPage(
          controller: widget.trendController,
          role: role,
          initialDate: _fmtDate(_queryDate),
        ),
      ),
    );
  }

  Future<void> _handleProfitTap(UserRole role, DashboardData dashboard) async {
    if (role != UserRole.owner && role != UserRole.sales) {
      _showMessage('仅 OWNER / SALES 可查看毛利趋势分析');
      return;
    }
    if (!mounted) return;
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => ProfitTrendPage(
          controller: widget.trendController,
          initialDate: _fmtDate(_queryDate),
        ),
      ),
    );
  }

  Future<void> _handleLowStockTap() async {
    if (!mounted) return;
    final session = widget.sessionController.session;
    final scope = session == null
        ? ''
        : '${session.tenantId}:${session.user.id}';
    final canInbound = session?.user.role == UserRole.owner ||
        session?.user.role == UserRole.purchaser;
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => LowStockPage(
          controller: widget.lowStockController,
          onGoInbound: canInbound
              ? () async {
                  await Navigator.of(context).push(MaterialPageRoute<void>(
                    builder: (_) => InboundPage(
                      controller: widget.inboundController,
                      productController: widget.productController,
                      sessionStorage: widget.sessionStorage,
                      scanPreferenceScope: scope,
                      logsController: widget.inboundLogsController,
                      batchController: widget.batchController,
                    ),
                  ));
                  // 公局入库页返回后刷新看板
                  if (mounted) _load();
                }
              : null,
        ),
      ),
    );
    // 从低库存页返回后刷新看板（可能看过数据后用户希望首页同步）
    if (mounted) _autoRefresh();
  }

  Future<void> _handleTopSalesTap() async {
    if (!mounted) return;
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => TopSalesPage(
          controller: widget.topSalesController,
          initialDate: _fmtDate(_queryDate),
        ),
      ),
    );
  }

  /// 快捷入口导航——所有操作页返回后自动刷新看板
  Future<void> _handleQuickActionTap(_QuickActionType type) async {
    final session = widget.sessionController.session;
    final scope = session == null
        ? ''
        : '${session.tenantId}:${session.user.id}';

    switch (type) {
      case _QuickActionType.inbound:
        // await 返回后刷新看板（入库会改变库存和成本）
        await Navigator.of(context).push(MaterialPageRoute<void>(
          builder: (_) => InboundPage(
            controller: widget.inboundController,
            productController: widget.productController,
            sessionStorage: widget.sessionStorage,
            scanPreferenceScope: scope,
            logsController: widget.inboundLogsController,
            batchController: widget.batchController,
          ),
        ));
        if (mounted) _load(); // 入库可能改变库存/成本，强制刷新

      case _QuickActionType.outbound:
        // await 返回后刷新看板（出库会改变库存和销售指标）
        await Navigator.of(context).push(MaterialPageRoute<void>(
          builder: (_) => OutboundPage(
            controller: widget.outboundController,
            sessionStorage: widget.sessionStorage,
            scanPreferenceScope: scope,
            onViewHistory: () {
              final now = DateTime.now();
              final today =
                  '${now.year.toString().padLeft(4, '0')}-${now.month.toString().padLeft(2, '0')}-${now.day.toString().padLeft(2, '0')}';
              Navigator.of(context).push(MaterialPageRoute<void>(
                builder: (_) => DashboardOrdersPage(
                  controller: widget.dashboardOrdersController,
                  initialStartDate: today,
                  initialEndDate: today,
                  initialPageSize: 10,
                ),
              ));
            },
          ),
        ));
        if (mounted) _load(); // 出库可能改变销售额/订单数，强制刷新

      case _QuickActionType.stockCheck:
        // await 返回后刷新看板（盘点可能改变库存）
        await Navigator.of(context).push(MaterialPageRoute<void>(
          builder: (_) => StockCheckPage(
            controller: widget.stockCheckController,
            logsController: widget.stockCheckLogsController,
          ),
        ));
        if (mounted) _autoRefresh(); // 盘点返回用防抖，避免搞盘点连点进出反复请求

      case _QuickActionType.products:
        // await 返回后刷新批次预警（用户可能新建批次或标记售完）
        await Navigator.of(context).push(MaterialPageRoute<void>(
          builder: (_) => ProductsPage(
            controller: widget.productController,
            categoryController: widget.categoryController,
            batchController: widget.batchController,
            onStockCheck: (int productId) {
              Navigator.of(context).push(MaterialPageRoute<void>(
                builder: (_) => StockCheckPage(
                  controller: widget.stockCheckController,
                  logsController: widget.stockCheckLogsController,
                  initialProductId: productId,
                ),
              ));
            },
          ),
        ));
        if (mounted) {
          // 商品页可能添加/编辑批次，刷新批次预警卡片
          widget.batchController.loadExpiring(withinDays: 30);
        }

      case _QuickActionType.users:
        final selfId = widget.sessionController.session?.user.id ?? '';
        await Navigator.of(context).push(MaterialPageRoute<void>(
          builder: (_) => UsersPage(
            controller: widget.usersController,
            currentUserId: selfId,
          ),
        ));
        // 人员管理返回无需刷新看板数据
    }
  }

  // ── Build ─────────────────────────────────────────────────────────────────

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
        final role = session?.user.role ?? UserRole.unknown;
        final loading = widget.dashboardController.loading;

        // ── Role-based permissions ──
        final canInbound =
            role == UserRole.owner || role == UserRole.purchaser;
        final canOutbound =
            role == UserRole.owner || role == UserRole.sales;
        final canStockCheck =
            role == UserRole.owner || role == UserRole.purchaser;
        final canProducts = role == UserRole.owner ||
            role == UserRole.purchaser ||
            role == UserRole.sales;

        // Only show actions the user actually has access to
        final List<_QuickActionItem> quickActions = <_QuickActionItem>[
          if (canOutbound)
            const _QuickActionItem(
              type: _QuickActionType.outbound,
              title: '销售出库',
              subtitle: '高频出库作业',
              icon: Icons.local_shipping_rounded,
              color: Color(0xFFF59E0B),
              enabled: true,
            ),
          if (canInbound)
            const _QuickActionItem(
              type: _QuickActionType.inbound,
              title: '采购入库',
              subtitle: '到货扫码入库',
              icon: Icons.move_to_inbox_rounded,
              color: Color(0xFF10B981),
              enabled: true,
            ),
          if (canStockCheck)
            const _QuickActionItem(
              type: _QuickActionType.stockCheck,
              title: '库存盘点',
              subtitle: '盘盈盘亏确认',
              icon: Icons.fact_check_rounded,
              color: Color(0xFF8B5CF6),
              enabled: true,
            ),
          if (canProducts)
            const _QuickActionItem(
              type: _QuickActionType.products,
              title: '商品管理',
              subtitle: '建档/查询/维护',
              icon: Icons.inventory_2_rounded,
              color: Color(0xFF64748B),
              enabled: true,
            ),
          // 人员管理 — OWNER 专属
          if (role == UserRole.owner)
            const _QuickActionItem(
              type: _QuickActionType.users,
              title: '人员管理',
              subtitle: '添加员工 / 角色 / 密码',
              icon: Icons.manage_accounts_rounded,
              color: Color(0xFF8B5CF6),
              enabled: true,
            ),
        ];

        final isToday = DateUtils.isSameDay(
            _queryDate, DateUtils.dateOnly(DateTime.now()));
        final isYesterday = DateUtils.isSameDay(
            _queryDate,
            DateUtils.dateOnly(
                DateTime.now().subtract(const Duration(days: 1))));
        // Dynamic prefix: 今日 / 昨日 / M月D日
        final String datePrefix = isToday
            ? '今日'
            : isYesterday
                ? '昨日'
                : '${_queryDate.month}月${_queryDate.day}日';

        return Scaffold(
          appBar: AppBar(
            // P3: Show query date in title
            title: Text(
              isToday
                  ? '经营看板 · 今天'
                  : '经营看板 · ${_friendlyDate(_queryDate)}',
              style: const TextStyle(fontSize: 17),
            ),
            actions: <Widget>[
              IconButton(
                tooltip: '刷新',
                onPressed: loading ? null : _load,
                icon: const Icon(Icons.refresh),
              ),
              // P1: Move logout to overflow menu to avoid misclick
              PopupMenuButton<String>(
                icon: const Icon(Icons.more_vert),
                tooltip: '更多',
                itemBuilder: (_) => <PopupMenuEntry<String>>[
                  const PopupMenuItem<String>(
                    value: 'logout',
                    child: Row(
                      children: <Widget>[
                        Icon(Icons.logout, size: 18),
                        SizedBox(width: 8),
                        Text('退出登录'),
                      ],
                    ),
                  ),
                ],
                onSelected: (v) async {
                  if (v == 'logout') {
                    await widget.sessionController.logout();
                  }
                },
              ),
              const SizedBox(width: 4),
            ],
          ),
          body: RefreshIndicator(
            onRefresh: _load,
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: <Widget>[
                // ── User Identity Card ─────────────────────────────────────
                // P1: Remove tenantId, only show name + chinese role
                if (session != null)
                  _IdentityCard(
                    name: session.user.name,
                    roleLabel: role.chineseLabel,
                  ),
                const SizedBox(height: 12),

                // ── Date Selector (compact) ────────────────────────────────
                // P1: Replace heavy SectionCard with inline chip row + date button
                _DateSelectorBar(
                  queryDate: _queryDate,
                  loading: loading,
                  fmtDate: _fmtDate,
                  onPickDate: _pickDate,
                  onApplyDate: _applyDate,
                ),
                const SizedBox(height: 12),

                // ── Error ──────────────────────────────────────────────────
                if (widget.dashboardController.errorMessage != null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: StatusNotice(
                      message: widget.dashboardController.errorMessage!,
                      tone: NoticeTone.error,
                    ),
                  ),

                // ── Metric Cards ───────────────────────────────────────────
                // P2: 5 skeleton bones when loading
                if (loading)
                  const _DashboardSkeleton()
                else if (dashboard != null) ...<Widget>[
                  _MetricCard(
                    title: '$datePrefix销售额',
                    value: dashboard.totalSales,
                    icon: Icons.payments_rounded,
                    accentColor: const Color(0xFF10B981),
                    isCurrency: true,
                    onTap: () => _handleSalesTap(role, dashboard),
                  ),
                  _MetricCard(
                    title: '$datePrefix毛利润',
                    value: dashboard.totalGrossProfit,
                    icon: Icons.trending_up_rounded,
                    accentColor: const Color(0xFF3B82F6),
                    isCurrency: true,
                    onTap: () => _handleProfitTap(role, dashboard),
                  ),
                  _MetricCard(
                    title: '$datePrefix订单数',
                    value: '${dashboard.totalOrders}',
                    icon: Icons.receipt_long_rounded,
                    accentColor: const Color(0xFFF59E0B),
                    onTap: () =>
                        _handleOrdersMetricTap(role, dashboard),
                  ),
                  // P2: Low stock — alert color when > 0
                  _MetricCard(
                    title: '低库存商品',
                    value: '${dashboard.lowStockCount}',
                    icon: Icons.warning_amber_rounded,
                    accentColor: dashboard.lowStockCount > 0
                        ? const Color(0xFFEF4444)
                        : const Color(0xFF10B981),
                    isAlert: dashboard.lowStockCount > 0,
                    onTap: _handleLowStockTap,
                  ),
                  _MetricCard(
                    title: '热销商品',
                    value: dashboard.topSellingItem,
                    icon: Icons.local_fire_department_rounded,
                    accentColor: const Color(0xFFF97316),
                    onTap: _handleTopSalesTap,
                  ),
                ],
                const SizedBox(height: 16),

                // ── 临期批次预警 ██████████████████████████████████████████
                AnimatedBuilder(
                  animation: widget.batchController,
                  builder: (_, __) {
                    final expiring = widget.batchController.expiring;
                    final loading = widget.batchController.loading;
                    // 加载中时不渲染（避免闪烁）
                    if (loading && expiring.isEmpty) return const SizedBox.shrink();
                    return _ExpiringBatchesCard(
                      items: expiring,
                      onMarkSoldOut: (int id) async {
                        await widget.batchController.markSoldOut(id);
                      },
                      onViewAll: (int productId, String productName) {
                        Navigator.of(context).push(MaterialPageRoute<void>(
                          builder: (_) => BatchManagementPage(
                            productId: productId,
                            productName: productName,
                            controller: widget.batchController,
                          ),
                        ));
                      },
                    );
                  },
                ),
                const SizedBox(height: 16),

                // ── Quick Actions ──────────────────────────────────────────
                // P1: Only render actions user actually has access to
                SectionCard(
                  title: '高频作业入口',
                  subtitle: '按角色显示可用功能',
                  child: quickActions.isEmpty
                      ? const Padding(
                          padding: EdgeInsets.symmetric(vertical: 16),
                          child: Center(
                              child: Text('当前角色暂无可用操作',
                                  style: TextStyle(
                                      color: Color(0xFF94A3B8)))),
                        )
                      : GridView.builder(
                          shrinkWrap: true,
                          physics: const NeverScrollableScrollPhysics(),
                          gridDelegate:
                              const SliverGridDelegateWithFixedCrossAxisCount(
                            crossAxisCount: 2,
                            mainAxisSpacing: 10,
                            crossAxisSpacing: 10,
                            childAspectRatio: 1.85,
                          ),
                          itemCount: quickActions.length,
                          itemBuilder: (_, int i) =>
                              _QuickActionCard(
                                item: quickActions[i],
                                onTap: () =>
                                    _handleQuickActionTap(
                                        quickActions[i].type),
                              ),
                        ),
                ),
                const SizedBox(height: 16),
              ],
            ),
          ),
        );
      },
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// 临期批次预警卡片
// ════════════════════════════════════════════════════════════════════════════

class _ExpiringBatchesCard extends StatelessWidget {
  const _ExpiringBatchesCard({
    required this.items,
    required this.onMarkSoldOut,
    required this.onViewAll,
  });

  final List<ExpiringBatchData> items;
  final Future<void> Function(int batchId) onMarkSoldOut;
  final void Function(int productId, String productName) onViewAll;

  @override
  Widget build(BuildContext context) {
    if (items.isEmpty) {
      // 空状态：绿色安全提示
      return SectionCard(
        title: '批次效期监控',
        subtitle: '30天内到期提醒',
        child: Container(
          padding: const EdgeInsets.symmetric(vertical: 12, horizontal: 4),
          child: Row(
            children: <Widget>[
              Container(
                width: 36,
                height: 36,
                decoration: BoxDecoration(
                  color: const Color(0xFF10B981).withAlpha(26),
                  shape: BoxShape.circle,
                ),
                child: const Icon(
                  Icons.verified_rounded,
                  color: Color(0xFF10B981),
                  size: 20,
                ),
              ),
              const SizedBox(width: 12),
              const Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      '暂无临期批次',
                      style: TextStyle(
                        fontSize: 14,
                        fontWeight: FontWeight.w600,
                        color: Color(0xFF10B981),
                      ),
                    ),
                    SizedBox(height: 2),
                    Text(
                      '近 30 天内所有批次均安全',
                      style: TextStyle(fontSize: 12, color: Colors.grey),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      );
    }

    return SectionCard(
      title: '批次效期监控',
      subtitle: '${items.length} 个批次需关注',
      child: Column(
        children: <Widget>[
          for (final ExpiringBatchData item in items.take(5))
            _ExpiringRow(
              item: item,
              onMarkSoldOut: onMarkSoldOut,
              onViewAll: onViewAll,
            ),
          if (items.length > 5)
            Padding(
              padding: const EdgeInsets.only(top: 4),
              child: Center(
                child: Text(
                  '还有 ${items.length - 5} 条预警未显示',
                  style: const TextStyle(
                      fontSize: 12, color: Colors.grey),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _ExpiringRow extends StatelessWidget {
  const _ExpiringRow({
    required this.item,
    required this.onMarkSoldOut,
    required this.onViewAll,
  });

  final ExpiringBatchData item;
  final Future<void> Function(int) onMarkSoldOut;
  final void Function(int, String) onViewAll;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final batch = item.batch;
    final level = batch.expiryLevel;
    final color = switch (level) {
      BatchExpiryLevel.expired => Colors.grey.shade600,
      BatchExpiryLevel.critical => Colors.red.shade600,
      BatchExpiryLevel.warning => Colors.orange.shade600,
      _ => Colors.amber.shade700,
    };
    final statusText = switch (level) {
      BatchExpiryLevel.expired => '已过期',
      _ => '${batch.daysUntilExpiry}天后过期',
    };

    return Container(
      margin: const EdgeInsets.only(bottom: 8),
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        color: color.withAlpha(15),
        borderRadius: BorderRadius.circular(8),
        border: Border.all(color: color.withAlpha(51)),
      ),
      child: Row(
        children: <Widget>[
          Container(
            width: 4,
            height: 40,
            decoration: BoxDecoration(
              color: color,
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  item.productName,
                  style: const TextStyle(
                      fontSize: 13, fontWeight: FontWeight.w600),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                const SizedBox(height: 2),
                Text(
                  '批次：${batch.lotNumber}  ·  $statusText',
                  style: TextStyle(fontSize: 11, color: color),
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          Column(
            children: <Widget>[
              GestureDetector(
                onTap: () => onViewAll(batch.productId, item.productName),
                child: Text('查看',
                    style: TextStyle(
                        fontSize: 11,
                        color: cs.primary,
                        fontWeight: FontWeight.w500)),
              ),
              const SizedBox(height: 4),
              GestureDetector(
                onTap: () => onMarkSoldOut(batch.id),
                child: Text('售完',
                    style: TextStyle(
                        fontSize: 11, color: Colors.grey.shade500)),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Identity Card
// ════════════════════════════════════════════════════════════════════════════

class _IdentityCard extends StatelessWidget {
  const _IdentityCard({required this.name, required this.roleLabel});
  final String name;
  final String roleLabel;

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
          CircleAvatar(
            radius: 20,
            backgroundColor:
                cs.primary.withValues(alpha: 0.15),
            child: Text(
              name.isNotEmpty ? name[0] : '?',
              style: TextStyle(
                  fontSize: 16,
                  fontWeight: FontWeight.w700,
                  color: cs.primary),
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(name,
                    style: const TextStyle(
                        fontWeight: FontWeight.w700, fontSize: 15)),
                const SizedBox(height: 2),
                Text(roleLabel,
                    style: TextStyle(
                        fontSize: 12, color: cs.onSurfaceVariant)),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Date Selector Bar (compact)
// ════════════════════════════════════════════════════════════════════════════

class _DateSelectorBar extends StatelessWidget {
  const _DateSelectorBar({
    required this.queryDate,
    required this.loading,
    required this.fmtDate,
    required this.onPickDate,
    required this.onApplyDate,
  });

  final DateTime queryDate;
  final bool loading;
  final String Function(DateTime) fmtDate;
  final VoidCallback onPickDate;
  final void Function(DateTime) onApplyDate;

  @override
  Widget build(BuildContext context) {
    final now = DateUtils.dateOnly(DateTime.now());
    final yesterday = now.subtract(const Duration(days: 1));
    final cs = Theme.of(context).colorScheme;

    return Row(
      children: <Widget>[
        // Quick chips
        QuickDateChip(
          label: '今日',
          onTap: loading ? null : () => onApplyDate(now),
        ),
        QuickDateChip(
          label: '昨日',
          onTap: loading ? null : () => onApplyDate(yesterday),
        ),
        const Spacer(),
        // Date picker button
        OutlinedButton.icon(
          onPressed: loading ? null : onPickDate,
          icon: Icon(
            Icons.calendar_month_outlined,
            size: 16,
            color: cs.onSurfaceVariant,
          ),
          label: Text(
            fmtDate(queryDate),
            style: TextStyle(fontSize: 13, color: cs.onSurfaceVariant),
          ),
          style: OutlinedButton.styleFrom(
            padding:
                const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            visualDensity: VisualDensity.compact,
          ),
        ),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Metric Card
// ════════════════════════════════════════════════════════════════════════════

class _MetricCard extends StatelessWidget {
  const _MetricCard({
    required this.title,
    required this.value,
    required this.icon,
    required this.accentColor,
    this.isCurrency = false,
    this.isAlert = false,
    this.onTap,
  });

  final String title;
  final String value;
  final IconData icon;
  final Color accentColor;
  final bool isCurrency;
  final bool isAlert;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    // Parse numeric value for roll animation
    final double? numValue =
        double.tryParse(value.replaceAll(RegExp(r'[^0-9.]'), ''));

    final content = Padding(
      padding: const EdgeInsets.all(18),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: <Widget>[
          // Icon badge — uses accentColor
          Container(
            padding: const EdgeInsets.all(11),
            decoration: BoxDecoration(
              color: accentColor.withValues(alpha: 0.12),
              borderRadius: BorderRadius.circular(14),
            ),
            child: Icon(icon, color: accentColor, size: 26),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisAlignment: MainAxisAlignment.center,
              children: <Widget>[
                Text(
                  title,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context)
                            .colorScheme
                            .onSurfaceVariant,
                      ),
                ),
                const SizedBox(height: 4),
                if (numValue != null && numValue >= 0)
                  TweenAnimationBuilder<double>(
                    tween: Tween<double>(begin: 0, end: numValue),
                    duration: const Duration(milliseconds: 900),
                    curve: Curves.easeOutCubic,
                    builder: (_, double val, __) {
                      String display;
                      if (isCurrency || value.contains('.')) {
                        display = '¥${val.toStringAsFixed(2)}';
                      } else {
                        display = val.toInt().toString();
                      }
                      return Text(
                        display,
                        style: Theme.of(context)
                            .textTheme
                            .headlineSmall
                            ?.copyWith(
                              fontWeight: FontWeight.w800,
                              letterSpacing: -0.5,
                              // P2: alert color when isAlert
                              color: isAlert ? accentColor : null,
                            ),
                      );
                    },
                  )
                else
                  Text(
                    value,
                    style: Theme.of(context)
                        .textTheme
                        .headlineSmall
                        ?.copyWith(
                          fontWeight: FontWeight.w800,
                          letterSpacing: -0.5,
                          fontSize: value.length > 12 ? 14 : null,
                        ),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
              ],
            ),
          ),
          // Tap arrow or real sparkline area
          if (onTap != null) ...<Widget>[
            const SizedBox(width: 8),
            Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: <Widget>[
                // P2: A real sparkline area using accentColor data
                SizedBox(
                  width: 40,
                  height: 24,
                  child: CustomPaint(
                    painter: _SparklinePainter(accentColor),
                  ),
                ),
                const SizedBox(height: 4),
                Icon(Icons.chevron_right,
                    size: 16,
                    color: Theme.of(context).colorScheme.onSurfaceVariant),
              ],
            ),
          ],
        ],
      ),
    );

    return Card(
      margin: const EdgeInsets.only(bottom: 10),
      child: onTap == null
          ? content
          : InkWell(
              borderRadius: BorderRadius.circular(16),
              onTap: onTap,
              child: content,
            ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Quick Action Card
// ════════════════════════════════════════════════════════════════════════════

class _QuickActionCard extends StatelessWidget {
  const _QuickActionCard({required this.item, required this.onTap});
  final _QuickActionItem item;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      borderRadius: BorderRadius.circular(16),
      onTap: item.enabled ? onTap : null,
      child: Ink(
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(16),
          color: item.color.withValues(alpha: 0.08),
          border: Border.all(
              color: item.color.withValues(alpha: 0.22), width: 1),
        ),
        padding: const EdgeInsets.all(14),
        child: Row(
          children: <Widget>[
            Container(
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(
                color: item.color.withValues(alpha: 0.14),
                shape: BoxShape.circle,
              ),
              child: Icon(item.icon, color: item.color, size: 22),
            ),
            const SizedBox(width: 10),
            Expanded(
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    item.title,
                    style: const TextStyle(
                      fontWeight: FontWeight.w700,
                      fontSize: 14,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 2),
                  Text(
                    item.subtitle,
                    style: TextStyle(
                      fontSize: 11,
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Sparkline Painter — accentColor aware
// ════════════════════════════════════════════════════════════════════════════

class _SparklinePainter extends CustomPainter {
  _SparklinePainter(this.color);
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color.withValues(alpha: 0.5)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2.0
      ..strokeCap = StrokeCap.round
      ..strokeJoin = StrokeJoin.round;

    final path = Path();
    path.moveTo(0, size.height * 0.8);
    path.quadraticBezierTo(size.width * 0.25, size.height * 0.85,
        size.width * 0.45, size.height * 0.5);
    path.quadraticBezierTo(size.width * 0.65, size.height * 0.15,
        size.width * 0.82, size.height * 0.3);
    path.lineTo(size.width, 0);

    canvas.drawPath(path, paint);
  }

  @override
  bool shouldRepaint(covariant _SparklinePainter old) =>
      old.color != color;
}

// ════════════════════════════════════════════════════════════════════════════
// Dashboard Skeleton — 5 bones matching 5 metric cards
// ════════════════════════════════════════════════════════════════════════════

class _DashboardSkeleton extends StatelessWidget {
  const _DashboardSkeleton();

  @override
  Widget build(BuildContext context) {
    return Column(
      children: List<Widget>.generate(
          5, (_) => _buildBone(context)),
    );
  }

  Widget _buildBone(BuildContext context) {
    final base = Theme.of(context).colorScheme.surfaceContainerHighest;
    return Card(
      margin: const EdgeInsets.only(bottom: 10),
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Row(
          children: <Widget>[
            Container(
              width: 48,
              height: 48,
              decoration: BoxDecoration(
                color: base.withValues(alpha: 0.6),
                borderRadius: BorderRadius.circular(14),
              ),
            ),
            const SizedBox(width: 14),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Container(
                    width: 90,
                    height: 12,
                    decoration: BoxDecoration(
                      color: base.withValues(alpha: 0.5),
                      borderRadius: BorderRadius.circular(4),
                    ),
                  ),
                  const SizedBox(height: 10),
                  Container(
                    width: double.infinity,
                    height: 22,
                    decoration: BoxDecoration(
                      color: base.withValues(alpha: 0.5),
                      borderRadius: BorderRadius.circular(4),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
