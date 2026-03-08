class Env {
  const Env._();

  static const String defaultApiBaseUrl = 'http://1.14.45.242:8080/api/v1';

  static const String apiBaseUrl =
      String.fromEnvironment('API_BASE_URL', defaultValue: defaultApiBaseUrl);
}
