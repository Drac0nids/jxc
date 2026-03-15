import 'package:flutter/material.dart';
import 'package:flutter_localizations/flutter_localizations.dart';

import 'core/app_services.dart';
import 'core/theme/app_theme.dart';
import 'features/auth/application/session_controller.dart';
import 'features/auth/presentation/auth_page.dart';
import 'features/dashboard/application/dashboard_controller.dart';
import 'features/dashboard/application/dashboard_orders_controller.dart';
import 'features/dashboard/application/top_sales_controller.dart';
import 'features/dashboard/application/trend_controller.dart';
import 'features/dashboard/presentation/dashboard_page.dart';
import 'features/inventory/application/inbound_controller.dart';
import 'features/inventory/application/inbound_logs_controller.dart';
import 'features/inventory/application/outbound_controller.dart';
import 'features/inventory/application/stock_check_controller.dart';
import 'features/inventory/application/stock_check_logs_controller.dart';
import 'features/products/application/batch_controller.dart';
import 'features/products/application/category_controller.dart';
import 'features/products/application/product_controller.dart';
import 'features/products/application/low_stock_controller.dart';
import 'features/products/application/supplier_controller.dart';
import 'features/users/application/users_controller.dart';

class JxcApp extends StatefulWidget {
  const JxcApp({super.key});

  @override
  State<JxcApp> createState() => _JxcAppState();
}

class _JxcAppState extends State<JxcApp> {
  late final AppServices _services;
  late final SessionController _sessionController;
  late final DashboardController _dashboardController;
  late final DashboardOrdersController _dashboardOrdersController;
  InboundController? _inboundController;
  OutboundController? _outboundController;
  InboundLogsController? _inboundLogsController;
  StockCheckLogsController? _stockCheckLogsController;
  StockCheckController? _stockCheckController;
  ProductController? _productController;
  LowStockController? _lowStockController;
  CategoryController? _categoryController;
  BatchController? _batchController;
  SupplierController? _supplierController;
  UsersController? _usersController;
  late final TrendController _trendController;
  late final TopSalesController _topSalesController;
  String? _boundUserId;

  @override
  void initState() {
    super.initState();
    _services = AppServices.bootstrap();
    _sessionController = SessionController(
      authRepository: _services.authRepository,
      sessionStorage: _services.sessionStorage,
    );
    _dashboardController = DashboardController(
      repository: _services.dashboardRepository,
    );
    _dashboardOrdersController = DashboardOrdersController(
      repository: _services.dashboardRepository,
    );
    _trendController = TrendController(
      repository: _services.dashboardRepository,
    );
    _topSalesController = TopSalesController(
      repository: _services.dashboardRepository,
    );

    _restoreSession();
  }

  @override
  void dispose() {
    _sessionController.dispose();
    _dashboardController.dispose();
    _dashboardOrdersController.dispose();
    _trendController.dispose();
    _topSalesController.dispose();
    _inboundController?.dispose();
    _outboundController?.dispose();
    _inboundLogsController?.dispose();
    _stockCheckLogsController?.dispose();
    _stockCheckController?.dispose();
    _productController?.dispose();
    _lowStockController?.dispose();
    _categoryController?.dispose();
    _batchController?.dispose();
    _supplierController?.dispose();
    _usersController?.dispose();
    super.dispose();

  }

  Future<void> _restoreSession() async {
    await _sessionController.restoreSession();
  }

  void _ensureInventoryControllers() {
    final session = _sessionController.session;
    if (session == null) {
      _disposeInventoryControllers();
      return;
    }

    if (_boundUserId == session.user.id &&
        _inboundController != null &&
        _outboundController != null &&
        _inboundLogsController != null &&
        _stockCheckLogsController != null &&
        _stockCheckController != null &&
        _productController != null &&
        _lowStockController != null &&
        _categoryController != null &&
        _batchController != null &&
        _supplierController != null &&
        _usersController != null) {
      return;
    }

    _disposeInventoryControllers();

    _boundUserId = session.user.id;
    _inboundController = InboundController(
      repository: _services.inventoryRepository,
      role: session.user.role,
    );
    _outboundController = OutboundController(
      repository: _services.inventoryRepository,
      role: session.user.role,
    );
    _inboundLogsController = InboundLogsController(
      repository: _services.inventoryRepository,
      role: session.user.role,
    );
    _stockCheckLogsController = StockCheckLogsController(
      repository: _services.inventoryRepository,
      role: session.user.role,
    );
    _stockCheckController = StockCheckController(
      repository: _services.inventoryRepository,
      role: session.user.role,
    );
    _productController = ProductController(
      repository: _services.productRepository,
      role: session.user.role,
    );
    _lowStockController = LowStockController(
      repository: _services.productRepository,
    );
    _categoryController = CategoryController(
      repository: _services.categoryRepository,
    );
    _batchController = BatchController(
      repository: _services.batchRepository,
    );
    _supplierController = SupplierController(
      repository: _services.supplierRepository,
    );
    _usersController = UsersController(
      repository: _services.usersRepository,
    );
  }

  void _disposeInventoryControllers() {
    _boundUserId = null;
    _inboundController?.dispose();
    _outboundController?.dispose();
    _inboundLogsController?.dispose();
    _stockCheckLogsController?.dispose();
    _stockCheckController?.dispose();
    _productController?.dispose();
    _lowStockController?.dispose();
    _categoryController?.dispose();
    _batchController?.dispose();
    _supplierController?.dispose();
    _usersController?.dispose();
    _inboundController = null;
    _outboundController = null;
    _inboundLogsController = null;
    _stockCheckLogsController = null;
    _stockCheckController = null;
    _productController = null;
    _lowStockController = null;
    _categoryController = null;
    _batchController = null;
    _supplierController = null;
    _usersController = null;
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: '极速云进销存',
      debugShowCheckedModeBanner: false,
      theme: JxcTheme.light(),
      darkTheme: JxcTheme.dark(),
      themeMode: ThemeMode.system,
      // 全局中文化：日历、对话框、按钮文字等均显示中文
      locale: const Locale('zh', 'CN'),
      localizationsDelegates: const <LocalizationsDelegate<dynamic>>[
        GlobalMaterialLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
      ],
      supportedLocales: const <Locale>[
        Locale('zh', 'CN'),
        Locale('en', 'US'),
      ],
      home: AnimatedBuilder(
        animation: _sessionController,
        builder: (BuildContext context, Widget? child) {
          if (_sessionController.initializing) {
            return const _AppBootPage();
          }

          if (!_sessionController.isAuthenticated) {
            _disposeInventoryControllers();
            return AuthPage(
              sessionController: _sessionController,
              sessionStorage: _services.sessionStorage,
            );
          }

          _ensureInventoryControllers();

          return DashboardPage(
            key: ValueKey<String>(_sessionController.session!.user.id),
            sessionController: _sessionController,
            sessionStorage: _services.sessionStorage,
            dashboardController: _dashboardController,
            dashboardOrdersController: _dashboardOrdersController,
            trendController: _trendController,
            topSalesController: _topSalesController,
            lowStockController: _lowStockController!,
            inboundController: _inboundController!,
            outboundController: _outboundController!,
            inboundLogsController: _inboundLogsController!,
            stockCheckLogsController: _stockCheckLogsController!,
            stockCheckController: _stockCheckController!,
            productController: _productController!,
            categoryController: _categoryController!,
            batchController: _batchController!,
            supplierController: _supplierController!,
            usersController: _usersController!,
          );
        },
      ),
    );
  }
}

class _AppBootPage extends StatelessWidget {
  const _AppBootPage();

  @override
  Widget build(BuildContext context) {
    return const Scaffold(
      body: Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            CircularProgressIndicator(),
            SizedBox(height: 16),
            Text('正在初始化应用...'),
          ],
        ),
      ),
    );
  }
}
