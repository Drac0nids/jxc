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
import '../../products/application/supplier_controller.dart';
import '../../products/presentation/batch_management_page.dart';
import '../../products/presentation/low_stock_page.dart';
import '../../products/presentation/products_page.dart';
import '../../users/application/users_controller.dart';
import '../../users/presentation/users_page.dart';
import '../../serials/models/serial_repository.dart';
import '../../serials/presentation/serial_inbound_page.dart';
import '../../serials/presentation/serial_outbound_page.dart';
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
  serialInbound,
  serialOutbound,
}

/// 首页「作业入口」扫码模式标记（用于区分确认写入 vs 连续扫码）
enum _ScanModeEntry { scanConfirm, continuousScan }

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
    required this.supplierController,
    required this.usersController,
    required this.serialRepository,
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
  final SupplierController supplierController;
  final UsersController usersController;
  final SerialRepository serialRepository;

  @override
  State<DashboardPage> createState() => _DashboardPageState();
}

class _DashboardPageState extends State<DashboardPage>
    with WidgetsBindingObserver {
  static const int _defaultPageSize = 10;

  // 当前底部 Tab：0=看板  1=功能（默认）  2=我的
  int _currentTabIndex = 1;

  // PageController 用于左右滑动切换
  late final PageController _pageController =
      PageController(initialPage: _currentTabIndex);

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
    _pageController.dispose();
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
            supplierController: widget.supplierController,
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
        final selfRole = widget.sessionController.session?.user.role.serverValue ?? 'SALES';
        await Navigator.of(context).push(MaterialPageRoute<void>(
          builder: (_) => UsersPage(
            controller: widget.usersController,
            currentUserId: selfId,
            currentUserRole: selfRole,
          ),
        ));
        // 人员管理返回无需刷新看板数据

      case _QuickActionType.serialInbound:
        await Navigator.of(context).push(MaterialPageRoute<void>(
          builder: (_) => SerialInboundPage(
            repository: widget.serialRepository,
            productController: widget.productController,
          ),
        ));
        if (mounted) _load();

      case _QuickActionType.serialOutbound:
        await Navigator.of(context).push(MaterialPageRoute<void>(
          builder: (_) => SerialOutboundPage(
            repository: widget.serialRepository,
          ),
        ));
        if (mounted) _load();
    }
  }

  /// 从首页直接以指定扫码模式进入出库/入库页，自动触发摄像头
  Future<void> _navigateWithScan(
    _QuickActionType type,
    _ScanModeEntry mode,
  ) async {
    final session = widget.sessionController.session;
    final scope = session == null
        ? ''
        : '${session.tenantId}:${session.user.id}';

    final bool continuous = mode == _ScanModeEntry.continuousScan;

    switch (type) {
      case _QuickActionType.outbound:
        await Navigator.of(context).push(MaterialPageRoute<void>(
          builder: (_) => OutboundPage(
            controller: widget.outboundController,
            sessionStorage: widget.sessionStorage,
            scanPreferenceScope: scope,
            autoScan: true,
            initialScanMode: continuous
                ? OutboundScanMode.continuousScan
                : OutboundScanMode.scanConfirm,
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
        if (mounted) _load();

      case _QuickActionType.inbound:
        await Navigator.of(context).push(MaterialPageRoute<void>(
          builder: (_) => InboundPage(
            controller: widget.inboundController,
            productController: widget.productController,
            sessionStorage: widget.sessionStorage,
            scanPreferenceScope: scope,
            logsController: widget.inboundLogsController,
            batchController: widget.batchController,
            supplierController: widget.supplierController,
            autoScan: true,
            initialScanMode: continuous
                ? InboundScanMode.continuousScan
                : InboundScanMode.scanConfirm,
          ),
        ));
        if (mounted) _load();

      default:
        // 其他类型走普通跳转
        await _handleQuickActionTap(type);
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
        final isOwner = role == UserRole.owner;

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

        // 功能 Tab 快捷入口（人员管理移至「我的」Tab）
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
          if (canInbound)
            const _QuickActionItem(
              type: _QuickActionType.serialInbound,
              title: '序列号入库',
              subtitle: '手机/设备逐台录入',
              icon: Icons.qr_code_2_rounded,
              color: Color(0xFF0EA5E9),
              enabled: true,
            ),
          if (canOutbound)
            const _QuickActionItem(
              type: _QuickActionType.serialOutbound,
              title: '序列号出库',
              subtitle: '扫 SN 直接出库',
              icon: Icons.document_scanner_rounded,
              color: Color(0xFFEC4899),
              enabled: true,
            ),
        ];

        final isToday = DateUtils.isSameDay(
            _queryDate, DateUtils.dateOnly(DateTime.now()));
        final isYesterday = DateUtils.isSameDay(
            _queryDate,
            DateUtils.dateOnly(
                DateTime.now().subtract(const Duration(days: 1))));
        final String datePrefix = isToday
            ? '今日'
            : isYesterday
                ? '昨日'
                : '${_queryDate.month}月${_queryDate.day}日';

        // ── AppBar 标题随 Tab 变化 ──
        final String appBarTitle = switch (_currentTabIndex) {
          0 => isToday ? '经营看板 · 今天' : '经营看板 · ${_friendlyDate(_queryDate)}',
          1 => '功能',
          _ => '我的',
        };

        return Scaffold(
          appBar: AppBar(
            title: Text(appBarTitle,
                style: const TextStyle(fontSize: 17)),
            actions: <Widget>[
              if (_currentTabIndex == 0)
                IconButton(
                  tooltip: '刷新',
                  onPressed: loading ? null : _load,
                  icon: const Icon(Icons.refresh),
                ),
            ],
          ),

          // ── 底部导航栏 ──────────────────────────────────────────────────
          bottomNavigationBar: NavigationBar(
            selectedIndex: _currentTabIndex,
            onDestinationSelected: (int i) {
              setState(() => _currentTabIndex = i);
              _pageController.animateToPage(
                i,
                duration: const Duration(milliseconds: 280),
                curve: Curves.easeInOut,
              );
            },
            destinations: const <NavigationDestination>[
              NavigationDestination(
                icon: Icon(Icons.bar_chart_outlined),
                selectedIcon: Icon(Icons.bar_chart_rounded),
                label: '看板',
              ),
              NavigationDestination(
                icon: Icon(Icons.apps_outlined),
                selectedIcon: Icon(Icons.apps_rounded),
                label: '功能',
              ),
              NavigationDestination(
                icon: Icon(Icons.person_outline_rounded),
                selectedIcon: Icon(Icons.person_rounded),
                label: '我的',
              ),
            ],
          ),

          body: PageView(
            controller: _pageController,
            onPageChanged: (int i) => setState(() => _currentTabIndex = i),
            children: <Widget>[

              // ══════════════════════════════════════════════════════════════
              // Tab 0: 看板
              // ══════════════════════════════════════════════════════════════
              _KeepAlivePage(
                child: RefreshIndicator(
                  onRefresh: _load,
                  child: ListView(
                    padding: const EdgeInsets.all(16),
                    children: <Widget>[
                      _DateSelectorBar(
                        queryDate: _queryDate,
                        loading: loading,
                        fmtDate: _fmtDate,
                        onPickDate: _pickDate,
                        onApplyDate: _applyDate,
                      ),
                      const SizedBox(height: 12),
                      if (widget.dashboardController.errorMessage != null)
                        Padding(
                          padding: const EdgeInsets.only(bottom: 8),
                          child: StatusNotice(
                            message: widget.dashboardController.errorMessage!,
                            tone: NoticeTone.error,
                          ),
                        ),
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
                          onTap: () => _handleOrdersMetricTap(role, dashboard),
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
                    ],
                  ),
              ),  // end _KeepAlivePage Tab0
                ),

              // ══════════════════════════════════════════════════════════════
              // Tab 1: 功能
              // ══════════════════════════════════════════════════════════════
              _KeepAlivePage(
                child: ListView(
                  padding: const EdgeInsets.fromLTRB(16, 16, 16, 24),
                  children: <Widget>[
                    if (session != null) ...<Widget>[
                      _IdentityCard(
                        name: session.user.name,
                        roleLabel: role.chineseLabel,
                      ),
                      const SizedBox(height: 20),
                    ],

                    // ── 预警提醒（常驻显示）────────────────────────────────
                    const _SectionTitle(label: '预警提醒'),
                    const SizedBox(height: 8),
                    // 低库存预警
                    if (dashboard != null)
                      _AlertStatusTile(
                        icon: Icons.inventory_2_outlined,
                        alertColor: const Color(0xFFEF4444),
                        okColor: const Color(0xFF10B981),
                        isAlert: dashboard.lowStockCount > 0,
                        alertTitle: '低库存商品',
                        alertSubtitle:
                            '${dashboard.lowStockCount} 件商品库存不足，建议尽快补货',
                        okTitle: '库存充足',
                        okSubtitle: '无低库存商品，状态良好',
                        onTap: dashboard.lowStockCount > 0
                            ? _handleLowStockTap
                            : null,
                      )
                    else
                      const _AlertStatusTile(
                        icon: Icons.inventory_2_outlined,
                        alertColor: Color(0xFFEF4444),
                        okColor: Color(0xFF10B981),
                        isAlert: false,
                        alertTitle: '库存充足',
                        alertSubtitle: '',
                        okTitle: '库存加载中...',
                        okSubtitle: '正在获取库存数据',
                        onTap: null,
                      ),
                    const SizedBox(height: 8),
                    // 临期批次预警
                    AnimatedBuilder(
                      animation: widget.batchController,
                      builder: (BuildContext ctx, _) {
                        final expiring = widget.batchController.expiring;
                        final batchLoading = widget.batchController.loading;
                        if (batchLoading && expiring.isEmpty) {
                          return const _AlertStatusTile(
                            icon: Icons.schedule_rounded,
                            alertColor: Color(0xFFF59E0B),
                            okColor: Color(0xFF10B981),
                            isAlert: false,
                            alertTitle: '批次健康',
                            alertSubtitle: '',
                            okTitle: '批次加载中...',
                            okSubtitle: '正在检查临期批次',
                            onTap: null,
                          );
                        }
                        return _AlertStatusTile(
                          icon: Icons.schedule_rounded,
                          alertColor: const Color(0xFFF59E0B),
                          okColor: const Color(0xFF10B981),
                          isAlert: expiring.isNotEmpty,
                          alertTitle: '临期批次提醒',
                          alertSubtitle:
                              '${expiring.length} 个批次将于 30 天内到期，请尽快处理',
                          okTitle: '批次健康',
                          okSubtitle: '30 天内无临期批次',
                          onTap: expiring.isEmpty
                              ? null
                              : () {
                                  final first = expiring.first;
                                  Navigator.of(ctx)
                                      .push(MaterialPageRoute<void>(
                                    builder: (_) => BatchManagementPage(
                                      productId: first.batch.productId,
                                      productName: first.productName,
                                      controller: widget.batchController,
                                    ),
                                  ));
                                },
                        );
                      },
                    ),
                    const SizedBox(height: 20),


                    // ── 作业入口（模块化） ───────────────────────────────────
                    if (quickActions.isEmpty)
                      const Center(
                        child: Padding(
                          padding: EdgeInsets.symmetric(vertical: 32),
                          child: Text(
                            '当前角色暂无可用操作',
                            style: TextStyle(color: Color(0xFF94A3B8)),
                          ),
                        ),
                      )
                    else ...<Widget>[
                      // ── 出库 / 入库（模式切换卡）──────────────
                      if (canOutbound || canInbound) ...<Widget>[
                        const _SectionTitle(label: '出入库'),
                        const SizedBox(height: 8),
                        GridView.count(
                          shrinkWrap: true,
                          physics: const NeverScrollableScrollPhysics(),
                          crossAxisCount: 2,
                          mainAxisSpacing: 10,
                          crossAxisSpacing: 10,
                          childAspectRatio: 1.45,
                          children: <Widget>[
                            if (canOutbound)
                              _ScanToggleCard(
                                title: '销售出库',
                                icon: Icons.local_shipping_rounded,
                                color: const Color(0xFFF59E0B),
                                onEnter: (bool continuous) => continuous
                                    ? _navigateWithScan(
                                        _QuickActionType.outbound,
                                        _ScanModeEntry.continuousScan)
                                    : _navigateWithScan(
                                        _QuickActionType.outbound,
                                        _ScanModeEntry.scanConfirm),
                              ),
                            if (canInbound)
                              _ScanToggleCard(
                                title: '采购入库',
                                icon: Icons.move_to_inbox_rounded,
                                color: const Color(0xFF10B981),
                                onEnter: (bool continuous) => continuous
                                    ? _navigateWithScan(
                                        _QuickActionType.inbound,
                                        _ScanModeEntry.continuousScan)
                                    : _navigateWithScan(
                                        _QuickActionType.inbound,
                                        _ScanModeEntry.scanConfirm),
                              ),
                          ],
                        ),
                        const SizedBox(height: 12),
                      ],

                      // ── 其他作业（盘点 / 商品）────────────────────
                      if (canStockCheck || canProducts) ...<Widget>[
                        const _SectionTitle(label: '其他作业'),
                        const SizedBox(height: 8),
                        GridView.count(
                          shrinkWrap: true,
                          physics: const NeverScrollableScrollPhysics(),
                          crossAxisCount: 2,
                          mainAxisSpacing: 10,
                          crossAxisSpacing: 10,
                          childAspectRatio: 1.9,
                          children: <Widget>[
                            if (canStockCheck)
                              _QuickActionCard(
                                item: const _QuickActionItem(
                                  type: _QuickActionType.stockCheck,
                                  title: '库存盘点',
                                  subtitle: '盘盈盘亏确认',
                                  icon: Icons.fact_check_rounded,
                                  color: Color(0xFF8B5CF6),
                                  enabled: true,
                                ),
                                onTap: () => _handleQuickActionTap(
                                    _QuickActionType.stockCheck),
                              ),
                            if (canProducts)
                              _QuickActionCard(
                                item: const _QuickActionItem(
                                  type: _QuickActionType.products,
                                  title: '商品管理',
                                  subtitle: '建档/查询/维护',
                                  icon: Icons.inventory_2_rounded,
                                  color: Color(0xFF64748B),
                                  enabled: true,
                                ),
                                onTap: () => _handleQuickActionTap(
                                    _QuickActionType.products),
                              ),
                          ],
                        ),
                      ],

                      // ── 序列号管理 ──────────────────────────────────
                      if (canInbound || canOutbound) ...<Widget>[
                        const SizedBox(height: 12),
                        const _SectionTitle(label: '序列号管理'),
                        const SizedBox(height: 8),
                        GridView.count(
                          shrinkWrap: true,
                          physics: const NeverScrollableScrollPhysics(),
                          crossAxisCount: 2,
                          mainAxisSpacing: 10,
                          crossAxisSpacing: 10,
                          childAspectRatio: 1.9,
                          children: <Widget>[
                            if (canInbound)
                              _QuickActionCard(
                                item: const _QuickActionItem(
                                  type: _QuickActionType.serialInbound,
                                  title: '序列号入库',
                                  subtitle: '手机/设备逐台录入',
                                  icon: Icons.qr_code_2_rounded,
                                  color: Color(0xFF0EA5E9),
                                  enabled: true,
                                ),
                                onTap: () => _handleQuickActionTap(
                                    _QuickActionType.serialInbound),
                              ),
                            if (canOutbound)
                              _QuickActionCard(
                                item: const _QuickActionItem(
                                  type: _QuickActionType.serialOutbound,
                                  title: '序列号出库',
                                  subtitle: '扫 SN 自动识别出库',
                                  icon: Icons.document_scanner_rounded,
                                  color: Color(0xFFEC4899),
                                  enabled: true,
                                ),
                                onTap: () => _handleQuickActionTap(
                                    _QuickActionType.serialOutbound),
                              ),
                          ],
                        ),
                      ],
                    ],
                  ],
                ),
              ),  // end _KeepAlivePage Tab1

              // ══════════════════════════════════════════════════════════════
              // Tab 2: 我的
              // ══════════════════════════════════════════════════════════════
              _KeepAlivePage(
                child: ListView(
                  padding: const EdgeInsets.fromLTRB(16, 20, 16, 32),
                  children: <Widget>[
                    if (session != null) ...<Widget>[
                      _ProfileHeader(
                        name: session.user.name,
                        roleLabel: role.chineseLabel,
                      ),
                      const SizedBox(height: 28),
                    ],
  
                    // 系统管理 —— 仅 OWNER
                    if (isOwner) ...<Widget>[
                      const _SectionTitle(label: '系统管理'),
                      const SizedBox(height: 8),
                      _ProfileTile(
                        icon: Icons.manage_accounts_rounded,
                        color: const Color(0xFF8B5CF6),
                        title: '人员管理',
                        subtitle: '添加员工、分配角色、重置密码',
                        onTap: () {
                          final selfId =
                              widget.sessionController.session?.user.id ?? '';
                          final selfRole =
                              widget.sessionController.session?.user.role.serverValue ?? 'SALES';
                          Navigator.of(context).push(MaterialPageRoute<void>(
                            builder: (_) => UsersPage(
                              controller: widget.usersController,
                              currentUserId: selfId,
                              currentUserRole: selfRole,
                            ),
                          ));
                        },
                      ),
                      const SizedBox(height: 20),
                    ],
  
                    // 关于
                    const _SectionTitle(label: '关于'),
                    const SizedBox(height: 8),
                    const _ProfileTile(
                      icon: Icons.info_outline_rounded,
                      color: Color(0xFF64748B),
                      title: '极速云进销存',
                      subtitle: '版本 0.1.0',
                      onTap: null,
                    ),
                    const SizedBox(height: 32),
  
                    // 退出登录
                    SizedBox(
                      width: double.infinity,
                      child: OutlinedButton.icon(
                        onPressed: () async {
                          final bool confirmed = await showDialog<bool>(
                                context: context,
                                builder: (_) => AlertDialog(
                                  title: const Text('确认退出'),
                                  content: const Text('退出后需要重新登录。'),
                                  actions: <Widget>[
                                    TextButton(
                                      onPressed: () =>
                                          Navigator.of(context).pop(false),
                                      child: const Text('取消'),
                                    ),
                                    FilledButton(
                                      style: FilledButton.styleFrom(
                                        backgroundColor: Theme.of(context)
                                            .colorScheme
                                            .error,
                                      ),
                                      onPressed: () =>
                                          Navigator.of(context).pop(true),
                                      child: const Text('退出'),
                                    ),
                                  ],
                                ),
                              ) ??
                              false;
                          if (confirmed) {
                            await widget.sessionController.logout();
                          }
                        },
                        style: OutlinedButton.styleFrom(
                          foregroundColor:
                              Theme.of(context).colorScheme.error,
                          side: BorderSide(
                            color: Theme.of(context)
                                .colorScheme
                                .error
                                .withValues(alpha: 0.5),
                          ),
                          padding:
                              const EdgeInsets.symmetric(vertical: 14),
                        ),
                        icon: const Icon(Icons.logout_rounded),
                        label: const Text('退出登录'),
                      ),
                    ),
                  ],
                ),  // close ListView
              ),  // end _KeepAlivePage Tab2
            ],
          ),
        );
      },
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// ════════════════════════════════════════════════════════════════════════════

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

// ════════════════════════════════════════════════════════════════════════════
// _ProfileHeader — 「我的」Tab 顶部用户头像卡
// ════════════════════════════════════════════════════════════════════════════

class _ProfileHeader extends StatelessWidget {
  const _ProfileHeader({required this.name, required this.roleLabel});
  final String name;
  final String roleLabel;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Column(
      children: <Widget>[
        CircleAvatar(
          radius: 36,
          backgroundColor: cs.primary.withValues(alpha: 0.15),
          child: Text(
            name.isNotEmpty ? name[0] : '?',
            style: TextStyle(
              fontSize: 28,
              fontWeight: FontWeight.w800,
              color: cs.primary,
            ),
          ),
        ),
        const SizedBox(height: 12),
        Text(
          name,
          style: const TextStyle(fontSize: 20, fontWeight: FontWeight.w800),
        ),
        const SizedBox(height: 4),
        Container(
          padding:
              const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
          decoration: BoxDecoration(
            color: cs.primary.withValues(alpha: 0.1),
            borderRadius: BorderRadius.circular(20),
          ),
          child: Text(
            roleLabel,
            style: TextStyle(
              fontSize: 13,
              fontWeight: FontWeight.w600,
              color: cs.primary,
            ),
          ),
        ),
      ],
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _SectionTitle — 分组标题
// ════════════════════════════════════════════════════════════════════════════

class _SectionTitle extends StatelessWidget {
  const _SectionTitle({required this.label});
  final String label;

  @override
  Widget build(BuildContext context) {
    return Text(
      label,
      style: TextStyle(
        fontSize: 13,
        fontWeight: FontWeight.w600,
        color: Theme.of(context).colorScheme.onSurfaceVariant,
        letterSpacing: 0.4,
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _ProfileTile — 列表项（带图标、标题、副标题、箭头）
// ════════════════════════════════════════════════════════════════════════════

class _ProfileTile extends StatelessWidget {
  const _ProfileTile({
    required this.icon,
    required this.color,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  final IconData icon;
  final Color color;
  final String title;
  final String subtitle;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Card(
      margin: EdgeInsets.zero,
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          child: Row(
            children: <Widget>[
              Container(
                width: 40,
                height: 40,
                decoration: BoxDecoration(
                  color: color.withValues(alpha: 0.12),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(icon, color: color, size: 22),
              ),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      title,
                      style: const TextStyle(
                          fontWeight: FontWeight.w600, fontSize: 15),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      subtitle,
                      style: TextStyle(
                          fontSize: 12, color: cs.onSurfaceVariant),
                    ),
                  ],
                ),
              ),
              if (onTap != null)
                Icon(Icons.chevron_right_rounded,
                    size: 20, color: cs.onSurfaceVariant),
            ],
          ),
        ),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _KeepAlivePage — 保持 PageView 子页不被销毁（保留滚动位置）
// ════════════════════════════════════════════════════════════════════════════

class _KeepAlivePage extends StatefulWidget {
  const _KeepAlivePage({required this.child});
  final Widget child;

  @override
  State<_KeepAlivePage> createState() => _KeepAlivePageState();
}

class _KeepAlivePageState extends State<_KeepAlivePage>
    with AutomaticKeepAliveClientMixin {
  @override
  bool get wantKeepAlive => true;

  @override
  Widget build(BuildContext context) {
    super.build(context);
    return widget.child;
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _AlertStatusTile — 预警状态 Tile（有问题=警告色，无问题=绿色健康）
// ════════════════════════════════════════════════════════════════════════════

class _AlertStatusTile extends StatelessWidget {
  const _AlertStatusTile({
    required this.icon,
    required this.alertColor,
    required this.okColor,
    required this.isAlert,
    required this.alertTitle,
    required this.alertSubtitle,
    required this.okTitle,
    required this.okSubtitle,
    required this.onTap,
  });

  final IconData icon;
  final Color alertColor;
  final Color okColor;
  final bool isAlert;
  final String alertTitle;
  final String alertSubtitle;
  final String okTitle;
  final String okSubtitle;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final color = isAlert ? alertColor : okColor;
    final title = isAlert ? alertTitle : okTitle;
    final subtitle = isAlert ? alertSubtitle : okSubtitle;
    final cs = Theme.of(context).colorScheme;

    return Card(
      margin: EdgeInsets.zero,
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
          child: Row(
            children: <Widget>[
              Container(
                width: 38,
                height: 38,
                decoration: BoxDecoration(
                  color: color.withValues(alpha: 0.12),
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(icon, color: color, size: 20),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      title,
                      style: TextStyle(
                        fontWeight: FontWeight.w600,
                        fontSize: 14,
                        color: isAlert ? color : null,
                      ),
                    ),
                    if (subtitle.isNotEmpty) ...<Widget>[
                      const SizedBox(height: 2),
                      Text(
                        subtitle,
                        style: TextStyle(
                          fontSize: 12,
                          color: cs.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ],
                ),
              ),
              if (onTap != null)
                Icon(Icons.chevron_right_rounded,
                    size: 18, color: cs.onSurfaceVariant)
              else
                Icon(
                  isAlert
                      ? Icons.error_outline_rounded
                      : Icons.check_circle_outline_rounded,
                  size: 18,
                  color: color.withValues(alpha: 0.7),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// _ScanToggleCard — 带模式切换的扫码入口卡
// ════════════════════════════════════════════════════════════════════════════

class _ScanToggleCard extends StatefulWidget {
  const _ScanToggleCard({
    required this.title,
    required this.icon,
    required this.color,
    required this.onEnter,
  });

  final String title;
  final IconData icon;
  final Color color;
  /// continuous=true → 连续扫码；false → 确认模式
  final void Function(bool continuous) onEnter;

  @override
  State<_ScanToggleCard> createState() => _ScanToggleCardState();
}

class _ScanToggleCardState extends State<_ScanToggleCard> {
  bool _continuous = true; // 默认：连续扫码

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final color = widget.color;

    return InkWell(
      borderRadius: BorderRadius.circular(16),
      onTap: () => widget.onEnter(_continuous),
      child: Ink(
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(16),
          color: color.withValues(alpha: 0.08),
          border: Border.all(color: color.withValues(alpha: 0.22), width: 1),
        ),
        padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            // 图标 + 标题行
            Row(
              children: <Widget>[
                Container(
                  padding: const EdgeInsets.all(7),
                  decoration: BoxDecoration(
                    color: color.withValues(alpha: 0.14),
                    shape: BoxShape.circle,
                  ),
                  child: Icon(widget.icon, color: color, size: 20),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    widget.title,
                    style: const TextStyle(
                        fontWeight: FontWeight.w700, fontSize: 14),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                Icon(Icons.arrow_forward_ios_rounded,
                    size: 12, color: cs.onSurfaceVariant),
              ],
            ),
            const Spacer(),
            // 模式切换胶囊
            GestureDetector(
              // 阻止点击切换器时同时触发外层 InkWell
              onTap: () {},
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  _ModeChip(
                    label: '确认',
                    selected: !_continuous,
                    color: color,
                    onTap: () => setState(() => _continuous = false),
                  ),
                  const SizedBox(width: 6),
                  _ModeChip(
                    label: '连续',
                    selected: _continuous,
                    color: color,
                    onTap: () => setState(() => _continuous = true),
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

class _ModeChip extends StatelessWidget {
  const _ModeChip({
    required this.label,
    required this.selected,
    required this.color,
    required this.onTap,
  });
  final String label;
  final bool selected;
  final Color color;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 180),
        padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 3),
        decoration: BoxDecoration(
          color: selected ? color.withValues(alpha: 0.18) : Colors.transparent,
          borderRadius: BorderRadius.circular(20),
          border: Border.all(
            color: selected ? color : color.withValues(alpha: 0.3),
            width: 1,
          ),
        ),
        child: Text(
          label,
          style: TextStyle(
            fontSize: 11,
            fontWeight: selected ? FontWeight.w700 : FontWeight.normal,
            color: selected ? color : color.withValues(alpha: 0.6),
          ),
        ),
      ),
    );
  }
}
