import 'package:flutter/foundation.dart';

import '../../../core/models/user_session.dart';
import '../../../network/api_exception.dart';
import '../../../storage/session_storage.dart';
import '../models/auth_repository.dart';
import '../models/login_request.dart';
import '../models/register_request.dart';

class SessionController extends ChangeNotifier {
  SessionController({
    required AuthRepository authRepository,
    required SessionStorage sessionStorage,
  })  : _authRepository = authRepository,
        _sessionStorage = sessionStorage;

  final AuthRepository _authRepository;
  final SessionStorage _sessionStorage;

  UserSession? _session;
  bool _initializing = true;
  bool _submitting = false;
  String? _errorMessage;

  UserSession? get session => _session;
  bool get isAuthenticated => _session != null;
  bool get initializing => _initializing;
  bool get submitting => _submitting;
  String? get errorMessage => _errorMessage;

  Future<void> restoreSession() async {
    try {
      _session = await _sessionStorage.read();
    } catch (error) {
      _errorMessage = _humanizeError(error);
    } finally {
      _initializing = false;
      notifyListeners();
    }
  }

  Future<bool> login({required String username, required String password}) async {
    if (_submitting) {
      return false;
    }

    _setSubmitting(true);
    try {
      final UserSession nextSession = await _authRepository.login(
        LoginRequest(username: username, password: password),
      );
      await _sessionStorage.write(nextSession);
      _session = nextSession;
      _errorMessage = null;
      notifyListeners();
      return true;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      notifyListeners();
      return false;
    } finally {
      _setSubmitting(false);
    }
  }

  Future<bool> register({
    required String username,
    required String name,
    required String password,
    String? tenantName,
  }) async {
    if (_submitting) {
      return false;
    }

    _setSubmitting(true);
    try {
      final UserSession nextSession = await _authRepository.register(
        RegisterRequest(
          username: username,
          name: name,
          password: password,
          tenantName: tenantName,
        ),
      );
      await _sessionStorage.write(nextSession);
      _session = nextSession;
      _errorMessage = null;
      notifyListeners();
      return true;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      notifyListeners();
      return false;
    } finally {
      _setSubmitting(false);
    }
  }

  Future<void> logout() async {
    if (_submitting) {
      return;
    }

    _submitting = true;
    notifyListeners();
    try {
      if (_session != null) {
        await _authRepository.logout();
      }
    } catch (_) {
      // 忽略登出接口失败，始终执行本地会话清理。
    } finally {
      await _sessionStorage.clear();
      _session = null;
      _errorMessage = null;
      _submitting = false;
      notifyListeners();
    }
  }

  void clearError() {
    if (_errorMessage == null || _errorMessage!.isEmpty) {
      return;
    }

    _errorMessage = null;
    notifyListeners();
  }

  void _setSubmitting(bool value) {
    _submitting = value;
    notifyListeners();
  }

  String _humanizeError(Object error) {
    if (error is ApiException) {
      final String requestIdPart =
          (error.requestId == null || error.requestId!.isEmpty) ? '' : '，request_id=${error.requestId}';
      return '${error.message}（code=${error.code}$requestIdPart）';
    }
    if (error is FormatException) {
      return error.message;
    }
    if (error is Exception) {
      return error.toString().replaceFirst('Exception: ', '');
    }

    return '发生未知错误，请稍后重试';
  }
}
