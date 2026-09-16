import 'package:flutter/material.dart';

import 'src/app.dart';
import 'src/config/env.dart';
import 'src/core/local_server.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  // Android 单机版：把服务端跑在 App 进程内（SQLite 本地存储）。
  // SaaS 形态下该方法直接返回 null，配置保持不变。
  final port = await LocalServer.startIfNeeded();
  final localBaseUrl = LocalServer.baseUrl;
  if (port != null && localBaseUrl != null) {
    Env.useLocalServer(localBaseUrl);
  }

  runApp(const JxcApp());
}
