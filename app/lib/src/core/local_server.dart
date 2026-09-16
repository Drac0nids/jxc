import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';
import 'package:path_provider/path_provider.dart';

typedef _StartServerNative = Int32 Function(Pointer<Utf8> dataDir);
typedef _StartServerDart = int Function(Pointer<Utf8> dataDir);
typedef _ServerPortNative = Int32 Function();
typedef _ServerPortDart = int Function();

/// Android 单机版：在 App 进程内启动内嵌服务端（Rust + SQLite）。
///
/// 服务端以 `cdylib`（`libjxc_server.so`）形式随 APK 分发，只监听
/// `127.0.0.1`，端口由系统分配；启动后把所有 API 请求指向本地回环地址，
/// 因此安装即可使用，不需要服务器、不需要联网、也不需要单独安装数据库。
///
/// 非 Android 平台（SaaS 形态）不启动内嵌服务端，返回 null。
class LocalServer {
  const LocalServer._();

  static const String _libraryName = 'jxc_server';

  static int? _port;
  static String? _baseUrl;

  /// 内嵌服务端是否已启动。
  static bool get isRunning => _port != null && _port! > 0;

  /// 内嵌服务端 API 根地址，形如 `http://127.0.0.1:<port>/api/v1`。
  static String? get baseUrl => _baseUrl;

  /// 当前监听端口（未启动时为 null）。
  static int? get port => _port;

  /// 启动内嵌服务端（幂等）。返回监听端口，失败或非 Android 平台返回 null。
  static Future<int?> startIfNeeded() async {
    if (!Platform.isAndroid) {
      return null;
    }
    if (isRunning) {
      return _port;
    }

    try {
      final library = DynamicLibrary.open('lib$_libraryName.so');
      final start = library.lookupFunction<_StartServerNative, _StartServerDart>(
        'jxc_start_server',
      );

      // 数据文件落在应用私有目录，卸载应用即随之清理。
      final directory = await getApplicationSupportDirectory();
      final dataDir = directory.path.toNativeUtf8();
      try {
        final port = start(dataDir);
        if (port <= 0) {
          return null;
        }
        _port = port;
        _baseUrl = 'http://127.0.0.1:$port/api/v1';
        return port;
      } finally {
        malloc.free(dataDir);
      }
    } catch (_) {
      // 内嵌服务端不可用时保持现有配置，由登录页给出网络错误提示。
      return null;
    }
  }

  /// 查询内嵌服务端当前端口（进程内幂等）。
  static int? queryPort() {
    if (!Platform.isAndroid) {
      return null;
    }
    try {
      final library = DynamicLibrary.open('lib$_libraryName.so');
      final port = library.lookupFunction<_ServerPortNative, _ServerPortDart>(
        'jxc_server_port',
      );
      final value = port();
      return value > 0 ? value : null;
    } catch (_) {
      return null;
    }
  }
}
