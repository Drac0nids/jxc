/// 全局运行配置。
///
/// SaaS 形态通过 `--dart-define=API_BASE_URL=...` 指定服务端地址；
/// 单机版（Android 内嵌服务端 / 桌面单机版）在启动时用
/// [useLocalServer] 覆盖为本地回环地址。
class Env {
  const Env._();

  /// 默认服务端地址（SaaS 部署可用 `--dart-define=API_BASE_URL` 覆盖）。
  static const String defaultApiBaseUrl = 'http://1.14.45.242:8080/api/v1';

  static const String _configuredApiBaseUrl =
      String.fromEnvironment('API_BASE_URL', defaultValue: defaultApiBaseUrl);

  static String? _runtimeApiBaseUrl;

  /// 当前生效的 API 地址。
  static String get apiBaseUrl => _runtimeApiBaseUrl ?? _configuredApiBaseUrl;

  /// 单机版固定租户码（服务端单机模式首次启动自动创建）。
  static const String standaloneTenantCode = 'local';

  /// 是否运行在单机版形态（服务端跑在本机 / App 进程内）。
  static bool get isStandalone => _runtimeApiBaseUrl != null;

  /// 切换为单机版本地服务地址。
  static void useLocalServer(String baseUrl) {
    _runtimeApiBaseUrl = baseUrl;
  }
}
