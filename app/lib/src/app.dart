import 'package:flutter/material.dart';

import 'core/app_services.dart';
import 'features/auth/application/session_controller.dart';
import 'features/auth/presentation/auth_page.dart';
import 'features/dashboard/application/dashboard_controller.dart';
import 'features/dashboard/presentation/dashboard_page.dart';

class JxcApp extends StatefulWidget {
  const JxcApp({super.key});

  @override
  State<JxcApp> createState() => _JxcAppState();
}

class _JxcAppState extends State<JxcApp> {
  late final AppServices _services;
  late final SessionController _sessionController;
  late final DashboardController _dashboardController;

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

    _restoreSession();
  }

  @override
  void dispose() {
    _sessionController.dispose();
    _dashboardController.dispose();
    super.dispose();
  }

  Future<void> _restoreSession() async {
    await _sessionController.restoreSession();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: '极速云进销存',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF2563EB)),
        useMaterial3: true,
      ),
      home: AnimatedBuilder(
        animation: _sessionController,
        builder: (BuildContext context, Widget? child) {
          if (_sessionController.initializing) {
            return const _AppBootPage();
          }

          if (!_sessionController.isAuthenticated) {
            return AuthPage(sessionController: _sessionController);
          }

          return DashboardPage(
            key: ValueKey<String>(_sessionController.session!.user.id),
            sessionController: _sessionController,
            dashboardController: _dashboardController,
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
