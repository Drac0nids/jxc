import 'package:flutter/material.dart';

import '../application/supplier_controller.dart';
import '../models/supplier_models.dart';

/// 供应商管理页（列表 + 增删改）
class SupplierManagementPage extends StatefulWidget {
  const SupplierManagementPage({super.key, required this.controller});

  final SupplierController controller;

  @override
  State<SupplierManagementPage> createState() => _SupplierManagementPageState();
}

class _SupplierManagementPageState extends State<SupplierManagementPage> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      widget.controller.load();
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('供应商管理'),
        actions: <Widget>[
          IconButton(
            tooltip: '新增供应商',
            icon: const Icon(Icons.add_rounded),
            onPressed: () => _showEditSheet(context, existing: null),
          ),
        ],
      ),
      body: AnimatedBuilder(
        animation: widget.controller,
        builder: (BuildContext context, Widget? _) {
          final bool loading = widget.controller.loading;
          final String? error = widget.controller.error;
          final List<SupplierData> list = widget.controller.suppliers;

          if (loading && list.isEmpty) {
            return const Center(child: CircularProgressIndicator());
          }

          if (error != null && list.isEmpty) {
            return Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Icon(Icons.error_outline,
                      color: Theme.of(context).colorScheme.error, size: 48),
                  const SizedBox(height: 8),
                  Text(error, textAlign: TextAlign.center),
                  const SizedBox(height: 12),
                  FilledButton(
                    onPressed: widget.controller.load,
                    child: const Text('重试'),
                  ),
                ],
              ),
            );
          }

          if (list.isEmpty) {
            return Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Icon(Icons.store_outlined,
                      size: 64, color: Colors.grey.shade300),
                  const SizedBox(height: 12),
                  const Text('暂无供应商，点击右上角 + 新增',
                      style: TextStyle(color: Colors.grey)),
                ],
              ),
            );
          }

          return RefreshIndicator(
            onRefresh: widget.controller.load,
            child: ListView.separated(
              padding: const EdgeInsets.all(12),
              itemCount: list.length,
              separatorBuilder: (_, __) => const SizedBox(height: 6),
              itemBuilder: (BuildContext context, int index) {
                final SupplierData s = list[index];
                return Card(
                  child: ListTile(
                    leading: CircleAvatar(
                      backgroundColor:
                          Theme.of(context).colorScheme.primaryContainer,
                      child: Icon(Icons.store_rounded,
                          color: Theme.of(context).colorScheme.primary,
                          size: 20),
                    ),
                    title: Text(s.name,
                        style:
                            const TextStyle(fontWeight: FontWeight.w600)),
                    subtitle: s.phone != null
                        ? Text(s.phone!,
                            style: const TextStyle(fontSize: 12))
                        : null,
                    trailing: PopupMenuButton<String>(
                      itemBuilder: (_) => <PopupMenuEntry<String>>[
                        const PopupMenuItem<String>(
                          value: 'edit',
                          child: Row(children: <Widget>[
                            Icon(Icons.edit_outlined, size: 18),
                            SizedBox(width: 8),
                            Text('编辑'),
                          ]),
                        ),
                        PopupMenuItem<String>(
                          value: 'delete',
                          child: Row(children: <Widget>[
                            Icon(Icons.delete_outline,
                                size: 18,
                                color:
                                    Theme.of(context).colorScheme.error),
                            const SizedBox(width: 8),
                            Text('删除',
                                style: TextStyle(
                                    color: Theme.of(context)
                                        .colorScheme
                                        .error)),
                          ]),
                        ),
                      ],
                      onSelected: (String val) {
                        if (val == 'edit') {
                          _showEditSheet(context, existing: s);
                        } else if (val == 'delete') {
                          _confirmDelete(context, s);
                        }
                      },
                    ),
                  ),
                );
              },
            ),
          );
        },
      ),
    );
  }

  Future<void> _showEditSheet(
    BuildContext context, {
    required SupplierData? existing,
  }) async {
    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      builder: (BuildContext ctx) => _SupplierEditSheet(
        existing: existing,
        controller: widget.controller,
      ),
    );
  }

  Future<void> _confirmDelete(
    BuildContext context,
    SupplierData s,
  ) async {
    final bool confirmed =
        await showDialog<bool>(
              context: context,
              builder: (BuildContext ctx) => AlertDialog(
                title: const Text('确认删除'),
                content: Text('确定删除供应商「${s.name}」吗？'),
                actions: <Widget>[
                  TextButton(
                    onPressed: () => Navigator.of(ctx).pop(false),
                    child: const Text('取消'),
                  ),
                  FilledButton(
                    style: FilledButton.styleFrom(
                        backgroundColor:
                            Theme.of(context).colorScheme.error),
                    onPressed: () => Navigator.of(ctx).pop(true),
                    child: const Text('删除'),
                  ),
                ],
              ),
            ) ??
        false;

    if (!confirmed || !mounted) return;

    final bool ok = await widget.controller.deleteSupplier(s.id);
    if (mounted && !ok) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(widget.controller.error ?? '删除失败'),
          backgroundColor: Theme.of(context).colorScheme.error,
        ),
      );
    }
  }
}

// ── 新建/编辑底部弹窗 ─────────────────────────────────────────────────────────

class _SupplierEditSheet extends StatefulWidget {
  const _SupplierEditSheet({
    required this.existing,
    required this.controller,
  });

  final SupplierData? existing;
  final SupplierController controller;

  @override
  State<_SupplierEditSheet> createState() => _SupplierEditSheetState();
}

class _SupplierEditSheetState extends State<_SupplierEditSheet> {
  final _formKey = GlobalKey<FormState>();
  late final TextEditingController _nameCtrl;
  late final TextEditingController _phoneCtrl;
  late final TextEditingController _notesCtrl;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _nameCtrl =
        TextEditingController(text: widget.existing?.name ?? '');
    _phoneCtrl =
        TextEditingController(text: widget.existing?.phone ?? '');
    _notesCtrl =
        TextEditingController(text: widget.existing?.notes ?? '');
  }

  @override
  void dispose() {
    _nameCtrl.dispose();
    _phoneCtrl.dispose();
    _notesCtrl.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (!_formKey.currentState!.validate()) return;
    setState(() => _saving = true);

    final bool isEdit = widget.existing != null;
    bool ok;

    if (isEdit) {
      ok = await widget.controller.updateSupplier(
        widget.existing!.id,
        UpdateSupplierRequest(
          name: _nameCtrl.text.trim(),
          phone: _phoneCtrl.text.trim().isEmpty
              ? null
              : _phoneCtrl.text.trim(),
          notes: _notesCtrl.text.trim().isEmpty
              ? null
              : _notesCtrl.text.trim(),
        ),
      );
    } else {
      final result = await widget.controller.createSupplier(
        CreateSupplierRequest(
          name: _nameCtrl.text.trim(),
          phone: _phoneCtrl.text.trim().isEmpty
              ? null
              : _phoneCtrl.text.trim(),
          notes: _notesCtrl.text.trim().isEmpty
              ? null
              : _notesCtrl.text.trim(),
        ),
      );
      ok = result != null;
    }

    if (!mounted) return;

    if (ok) {
      Navigator.of(context).pop();
    } else {
      setState(() => _saving = false);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(widget.controller.error ?? '保存失败'),
          backgroundColor: Theme.of(context).colorScheme.error,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final bool isEdit = widget.existing != null;

    return Padding(
      padding: EdgeInsets.only(
        bottom: MediaQuery.of(context).viewInsets.bottom,
      ),
      child: SafeArea(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 16, 16, 16),
          child: Form(
            key: _formKey,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  isEdit ? '编辑供应商' : '新建供应商',
                  style: const TextStyle(
                      fontSize: 16, fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 16),

                // 名称（必填）
                TextFormField(
                  controller: _nameCtrl,
                  autofocus: true,
                  decoration: const InputDecoration(
                    labelText: '供应商名称 *',
                    prefixIcon: Icon(Icons.store_rounded),
                    border: OutlineInputBorder(),
                  ),
                  validator: (String? v) =>
                      v == null || v.trim().isEmpty ? '名称不能为空' : null,
                ),
                const SizedBox(height: 12),

                // 联系电话（可选）
                TextFormField(
                  controller: _phoneCtrl,
                  keyboardType: TextInputType.phone,
                  decoration: const InputDecoration(
                    labelText: '联系电话（可选）',
                    prefixIcon: Icon(Icons.phone_outlined),
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 12),

                // 备注（可选）
                TextFormField(
                  controller: _notesCtrl,
                  maxLines: 2,
                  decoration: const InputDecoration(
                    labelText: '备注（可选）',
                    prefixIcon: Icon(Icons.notes_outlined),
                    border: OutlineInputBorder(),
                  ),
                ),
                const SizedBox(height: 20),

                Row(
                  children: <Widget>[
                    Expanded(
                      child: OutlinedButton(
                        onPressed: _saving
                            ? null
                            : () => Navigator.of(context).pop(),
                        child: const Text('取消'),
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: FilledButton(
                        onPressed: _saving ? null : _save,
                        child: _saving
                            ? const SizedBox(
                                width: 16,
                                height: 16,
                                child: CircularProgressIndicator(
                                    strokeWidth: 2, color: Colors.white),
                              )
                            : Text(isEdit ? '保存' : '创建'),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
