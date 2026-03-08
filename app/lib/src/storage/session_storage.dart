import 'dart:convert';

import 'package:shared_preferences/shared_preferences.dart';

import '../core/models/user_session.dart';

class SessionStorage {
  SessionStorage({SharedPreferences? preferences}) : _preferences = preferences;

  static const String _sessionKey = 'jxc.user_session';

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
}
