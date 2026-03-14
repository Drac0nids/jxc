import 'package:flutter/foundation.dart';

import '../../../network/api_exception.dart';
import '../models/users_models.dart';
import '../models/users_repository.dart';

class UsersController extends ChangeNotifier {
  UsersController({required UsersRepository repository})
      : _repository = repository;

  final UsersRepository _repository;

  bool _loading = false;
  bool _submitting = false;
  String? _errorMessage;
  String? _successMessage;
  List<UserData> _list = <UserData>[];

  bool get loading => _loading;
  bool get submitting => _submitting;
  String? get errorMessage => _errorMessage;
  String? get successMessage => _successMessage;
  List<UserData> get list => List<UserData>.unmodifiable(_list);

  void clearMessages() {
    _errorMessage = null;
    _successMessage = null;
    notifyListeners();
  }

  // ── Load ──────────────────────────────────────────────────────────────────

  Future<void> load() async {
    if (_loading) return;
    _loading = true;
    _errorMessage = null;
    notifyListeners();

    try {
      _list = await _repository.listUsers();
    } catch (e) {
      _errorMessage = _humanize(e);
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  // ── Create ────────────────────────────────────────────────────────────────

  Future<bool> createUser(CreateUserRequest request) async {
    if (_submitting) return false;
    _submitting = true;
    _errorMessage = null;
    _successMessage = null;
    notifyListeners();

    try {
      final created = await _repository.createUser(request);
      _successMessage = '员工「${created.name}」创建成功';
      await load();
      return true;
    } catch (e) {
      _errorMessage = _humanize(e);
      notifyListeners();
      return false;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  // ── Update Role ───────────────────────────────────────────────────────────

  Future<bool> updateRole(String userId, String newRole) async {
    if (_submitting) return false;
    _submitting = true;
    _errorMessage = null;
    _successMessage = null;
    notifyListeners();

    try {
      final updated = await _repository.updateRole(
          userId, UpdateUserRoleRequest(role: newRole));
      _successMessage = '「${updated.name}」角色已更新为 ${updated.chineseRole}';
      await load();
      return true;
    } catch (e) {
      _errorMessage = _humanize(e);
      notifyListeners();
      return false;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  // ── Reset Password ────────────────────────────────────────────────────────

  Future<bool> resetPassword(String userId, String newPassword) async {
    if (_submitting) return false;
    _submitting = true;
    _errorMessage = null;
    _successMessage = null;
    notifyListeners();

    try {
      await _repository.resetPassword(
          userId, ResetPasswordRequest(newPassword: newPassword));
      _successMessage = '密码重置成功';
      notifyListeners();
      return true;
    } catch (e) {
      _errorMessage = _humanize(e);
      notifyListeners();
      return false;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  // ── Error humanizer ───────────────────────────────────────────────────────

  String _humanize(Object error) {
    if (error is ApiException) {
      if (error.code == 4090) return '用户名已存在（code=4090）';
      if (error.code == 4030) return '无权限执行该操作（code=4030）';
      return '${error.message}（code=${error.code}）';
    }
    if (error is FormatException) return error.message;
    if (error is Exception) {
      return error.toString().replaceFirst('Exception: ', '');
    }
    return '操作失败，请稍后重试';
  }
}
