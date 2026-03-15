import 'package:flutter/foundation.dart';
import '../../../network/exception_utils.dart';

import '../models/supplier_models.dart';
import '../models/supplier_repository.dart';

class SupplierController extends ChangeNotifier {
  SupplierController({required SupplierRepository repository})
      : _repo = repository;

  final SupplierRepository _repo;

  List<SupplierData> _suppliers = <SupplierData>[];
  bool _loading = false;
  String? _error;

  List<SupplierData> get suppliers => _suppliers;
  bool get loading => _loading;
  String? get error => _error;

  Future<void> load() async {
    _loading = true;
    _error = null;
    notifyListeners();
    try {
      _suppliers = await _repo.listSuppliers();
    } catch (e) {
      _error = e.toString();
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  Future<List<SupplierData>> search(String keyword) async {
    try {
      return await _repo.listSuppliers(keyword: keyword);
    } catch (_) {
      return <SupplierData>[];
    }
  }

  Future<SupplierData?> createSupplier(CreateSupplierRequest req) async {
    try {
      final s = await _repo.createSupplier(req);
      _suppliers = <SupplierData>[s, ..._suppliers];
      _error = null;
      notifyListeners();
      return s;
    } catch (e) {
      _error = e.toString();
      notifyListeners();
      return null;
    }
  }

  Future<bool> updateSupplier(int id, UpdateSupplierRequest req) async {
    try {
      final s = await _repo.updateSupplier(id, req);
      final int idx = _suppliers.indexWhere((sl) => sl.id == id);
      if (idx >= 0) {
        _suppliers = List<SupplierData>.from(_suppliers)..[idx] = s;
      }
      _error = null;
      notifyListeners();
      return true;
    } catch (e) {
      _error = e.toString();
      notifyListeners();
      return false;
    }
  }

  Future<bool> deleteSupplier(int id) async {
    try {
      await _repo.deleteSupplier(id);
      _suppliers = _suppliers.where((s) => s.id != id).toList();
      _error = null;
      notifyListeners();
      return true;
    } catch (e) {
      _error = e.toString();
      notifyListeners();
      return false;
    }
  }
}
