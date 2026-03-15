import 'package:flutter/foundation.dart';

import '../../../network/exception_utils.dart';
import '../models/batch_models.dart';

class BatchController extends ChangeNotifier {
  BatchController({required BatchRepository repository})
      : _repository = repository;

  final BatchRepository _repository;

  bool _loading = false;
  bool _submitting = false;
  String? _errorMessage;

  // ── 批次列表（按商品）──────────────────────────────────────────────────────────
  List<BatchData> _batches = <BatchData>[];

  // ── 临期预警列表（仪表盘用）────────────────────────────────────────────────────
  List<ExpiringBatchData> _expiring = <ExpiringBatchData>[];

  bool get loading => _loading;
  bool get submitting => _submitting;
  String? get errorMessage => _errorMessage;
  List<BatchData> get batches => List<BatchData>.unmodifiable(_batches);
  List<ExpiringBatchData> get expiring =>
      List<ExpiringBatchData>.unmodifiable(_expiring);

  int get expiringCount => _expiring.length;

  void clearError() {
    _errorMessage = null;
    notifyListeners();
  }

  // ── 加载某商品的批次 ───────────────────────────────────────────────────────

  Future<void> loadBatchesForProduct(int productId,
      {bool onlyActive = false}) async {
    if (_loading) return;
    _loading = true;
    _errorMessage = null;
    notifyListeners();

    try {
      _batches = await _repository.listBatches(
        productId: productId,
        onlyActive: onlyActive,
      );
    } catch (e) {
      _errorMessage = _humanizeError(e);
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  // ── 加载临期预警 ───────────────────────────────────────────────────────────

  Future<void> loadExpiring({int withinDays = 30}) async {
    if (_loading) return;
    _loading = true;
    _errorMessage = null;
    notifyListeners();

    try {
      _expiring = await _repository.listExpiringBatches(withinDays: withinDays);
    } catch (e) {
      _errorMessage = _humanizeError(e);
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  // ── 新建批次 ───────────────────────────────────────────────────────────────

  Future<BatchData?> createBatch(CreateBatchRequest req) async {
    if (_submitting) return null;
    _submitting = true;
    _errorMessage = null;
    notifyListeners();

    try {
      final batch = await _repository.createBatch(req);
      return batch;
    } catch (e) {
      _errorMessage = _humanizeError(e);
      return null;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  // ── 编辑批次 ───────────────────────────────────────────────────────────────

  Future<BatchData?> updateBatch(int id, UpdateBatchRequest req) async {
    if (_submitting) return null;
    _submitting = true;
    _errorMessage = null;
    notifyListeners();

    try {
      final updated = await _repository.updateBatch(id, req);
      final idx = _batches.indexWhere((b) => b.id == id);
      if (idx >= 0) {
        _batches = List<BatchData>.from(_batches)..[idx] = updated;
      }
      return updated;
    } catch (e) {
      _errorMessage = _humanizeError(e);
      return null;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  // ── 标记售完 ───────────────────────────────────────────────────────────────

  Future<bool> markSoldOut(int id) async {
    if (_submitting) return false;
    _submitting = true;
    _errorMessage = null;
    notifyListeners();

    try {
      final updated = await _repository.markSoldOut(id);
      // 更新本地列表
      final bIdx = _batches.indexWhere((b) => b.id == id);
      if (bIdx >= 0) {
        _batches = List<BatchData>.from(_batches)..[bIdx] = updated;
      }
      // 从临期列表移除
      _expiring = _expiring.where((e) => e.batch.id != id).toList();
      return true;
    } catch (e) {
      _errorMessage = _humanizeError(e);
      return false;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  // ── 删除批次 ───────────────────────────────────────────────────────────────

  Future<bool> deleteBatch(int id) async {
    if (_submitting) return false;
    _submitting = true;
    _errorMessage = null;
    notifyListeners();

    try {
      await _repository.deleteBatch(id);
      _batches = _batches.where((b) => b.id != id).toList();
      _expiring = _expiring.where((e) => e.batch.id != id).toList();
      return true;
    } catch (e) {
      _errorMessage = _humanizeError(e);
      return false;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  String _humanizeError(Object error) => humanizeError(error);
}
