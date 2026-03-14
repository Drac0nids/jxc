import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

import '../core/models/user_session.dart';

class SessionStorage {
  SessionStorage({SharedPreferences? preferences}) : _preferences = preferences;

  static const String _sessionKey = 'jxc.user_session';
  static const String _scanPreferencesKey = 'jxc.scan_preferences';
  static const String _tenantCodeKey = 'jxc.last_tenant_code';

  SharedPreferences? _preferences;

  Future<SharedPreferences> _prefs() async {
    if (_preferences != null) {
      return _preferences!;
    }

    _preferences = await SharedPreferences.getInstance();
    return _preferences!;
  }

  Future<UserSession?> read() async {
    final prefs = await _prefs();
    final raw = prefs.getString(_sessionKey);
    if (raw == null || raw.isEmpty) {
      return null;
    }

    final decoded = jsonDecode(raw);
    if (decoded is! Map<String, dynamic>) {
      return null;
    }

    return UserSession.fromStorageJson(decoded);
  }

  Future<void> write(UserSession session) async {
    final prefs = await _prefs();
    final raw = jsonEncode(session.toStorageJson());
    await prefs.setString(_sessionKey, raw);
  }

  Future<void> clear() async {
    final prefs = await _prefs();
    await prefs.remove(_sessionKey);
  }

  /// 读取上次登录时保存的租户码（可能为 null）
  Future<String?> readTenantCode() async {
    final prefs = await _prefs();
    final v = prefs.getString(_tenantCodeKey);
    return (v == null || v.isEmpty) ? null : v;
  }

  /// 持久化租户码，供下次登录自动填写
  Future<void> writeTenantCode(String code) async {
    if (code.trim().isEmpty) return;
    final prefs = await _prefs();
    await prefs.setString(_tenantCodeKey, code.trim().toUpperCase());
  }

  Future<String?> readScanMode({
    required String scope,
    required String page,
  }) async {
    if (scope.trim().isEmpty || page.trim().isEmpty) {
      return null;
    }

    final prefs = await _prefs();
    final raw = prefs.getString(_scanPreferencesKey);
    if (raw == null || raw.isEmpty) {
      return null;
    }

    final decoded = jsonDecode(raw);
    if (decoded is! Map<String, dynamic>) {
      return null;
    }

    final scoped = decoded[scope];
    if (scoped is! Map<String, dynamic>) {
      return null;
    }

    final mode = scoped[page];
    if (mode is! String || mode.trim().isEmpty) {
      return null;
    }

    return mode;
  }

  Future<void> writeScanMode({
    required String scope,
    required String page,
    required String mode,
  }) async {
    if (scope.trim().isEmpty || page.trim().isEmpty || mode.trim().isEmpty) {
      return;
    }

    final prefs = await _prefs();
    final raw = prefs.getString(_scanPreferencesKey);
    final Map<String, dynamic> root;
    if (raw == null || raw.isEmpty) {
      root = <String, dynamic>{};
    } else {
      final decoded = jsonDecode(raw);
      root = decoded is Map<String, dynamic>
          ? Map<String, dynamic>.from(decoded)
          : <String, dynamic>{};
    }

    final scopedRaw = root[scope];
    final Map<String, dynamic> scoped = scopedRaw is Map<String, dynamic>
        ? Map<String, dynamic>.from(scopedRaw)
        : <String, dynamic>{};
    scoped[page] = mode;
    root[scope] = scoped;

    await prefs.setString(_scanPreferencesKey, jsonEncode(root));
  }

  Future<void> clearScanModesForScope(String scope) async {
    if (scope.trim().isEmpty) {
      return;
    }

    final prefs = await _prefs();
    final raw = prefs.getString(_scanPreferencesKey);
    if (raw == null || raw.isEmpty) {
      return;
    }

    final decoded = jsonDecode(raw);
    if (decoded is! Map<String, dynamic>) {
      return;
    }

    final root = Map<String, dynamic>.from(decoded);
    if (!root.containsKey(scope)) {
      return;
    }

    root.remove(scope);
    if (root.isEmpty) {
      await prefs.remove(_scanPreferencesKey);
      return;
    }

    await prefs.setString(_scanPreferencesKey, jsonEncode(root));
  }
}
