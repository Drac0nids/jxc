import 'package:flutter/material.dart';

import '../application/batch_controller.dart';
import '../models/batch_models.dart';

/// 新建批次信息弹窗（入库后弹出询问）
class CreateBatchSheet extends StatefulWidget {
  const CreateBatchSheet({
    super.key,
    required this.productId,
    required this.productName,
    required this.controller,
  });

  final int productId;
  final String productName;
  final BatchController controller;

  static Future<BatchData?> show(
    BuildContext context, {
    required int productId,
    required String productName,
    required BatchController controller,
  }) {
    return showModalBottomSheet<BatchData>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (_) => CreateBatchSheet(
        productId: productId,
        productName: productName,
        controller: controller,
      ),
    );
  }

  @override
  State<CreateBatchSheet> createState() => _CreateBatchSheetState();
}

class _CreateBatchSheetState extends State<CreateBatchSheet> {
  final _formKey = GlobalKey<FormState>();
  final _lotCtrl = TextEditingController();
  final _supplierCtrl = TextEditingController();
  final _notesCtrl = TextEditingController();

  DateTime _inboundAt = DateTime.now();
  DateTime? _producedAt;
  DateTime? _expiresAt;

  bool _hasExpiry = false; // 是否填写过期日期

  @override
  void dispose() {
    _lotCtrl.dispose();
    _supplierCtrl.dispose();
    _notesCtrl.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (!(_formKey.currentState?.validate() ?? false)) return;
    if (_hasExpiry && _expiresAt == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('请选择过期日期')),
      );
      return;
    }

    final req = CreateBatchRequest(
      productId: widget.productId,
      lotNumber: _lotCtrl.text.trim().isEmpty ? null : _lotCtrl.text.trim(),
      supplier: _supplierCtrl.text.trim().isEmpty ? null : _supplierCtrl.text.trim(),
      inboundAt: _inboundAt,
      producedAt: _producedAt,
      expiresAt: _hasExpiry ? _expiresAt : null,
      notes: _notesCtrl.text.trim().isEmpty ? null : _notesCtrl.text.trim(),
    );

    final result = await widget.controller.createBatch(req);
    if (!mounted) return;

    if (result != null) {
      Navigator.of(context).pop(result);
    }
  }

  Future<void> _pickDate({
    required DateTime? current,
    required String label,
    required void Function(DateTime) onPicked,
  }) async {
    final picked = await showDatePicker(
      context: context,
      initialDate: current ?? DateTime.now(),
      firstDate: DateTime(2000),
      lastDate: DateTime(2099),
      helpText: '选择$label',
    );
    if (picked != null) onPicked(picked);
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final submitting = widget.controller.submitting;

    return Padding(
      padding: EdgeInsets.only(
        bottom: MediaQuery.of(context).viewInsets.bottom,
      ),
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            // 顶部把手
            const SizedBox(height: 8),
            Container(
              width: 40,
              height: 4,
              decoration: BoxDecoration(
                color: Colors.grey.shade300,
                borderRadius: BorderRadius.circular(2),
              ),
            ),
            const SizedBox(height: 16),
            // 标题
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              child: Row(
                children: <Widget>[
                  Container(
                    padding: const EdgeInsets.all(8),
                    decoration: BoxDecoration(
                      color: cs.primaryContainer,
                      borderRadius: BorderRadius.circular(10),
                    ),
                    child: Icon(Icons.inventory_2_outlined,
                        size: 20, color: cs.primary),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        const Text(
                          '记录批次信息',
                          style: TextStyle(
                              fontSize: 16, fontWeight: FontWeight.w600),
                        ),
                        Text(
                          widget.productName,
                          style: TextStyle(
                              fontSize: 12, color: cs.onSurfaceVariant),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 20),

            // 表单
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 20),
              child: Form(
                key: _formKey,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    // 错误提示
                    if (widget.controller.errorMessage != null)
                      Container(
                        margin: const EdgeInsets.only(bottom: 12),
                        padding: const EdgeInsets.all(10),
                        decoration: BoxDecoration(
                          color: cs.errorContainer,
                          borderRadius: BorderRadius.circular(8),
                        ),
                        child: Text(
                          widget.controller.errorMessage!,
                          style: TextStyle(
                              fontSize: 13, color: cs.onErrorContainer),
                        ),
                      ),

                    // 批次号（可选，默认自动生成 YYYYMMDD-NN）
                    TextFormField(
                      controller: _lotCtrl,
                      decoration: const InputDecoration(
                        labelText: '批次号（可选）',
                        hintText: '留空则自动生成 YYYYMMDD-01',
                        prefixIcon: Icon(Icons.tag_rounded),
                        border: OutlineInputBorder(),
                        helperText: '同天同商品第2批自动变 -02，以此类推',
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 供应商（可选）
                    TextFormField(
                      controller: _supplierCtrl,
                      decoration: const InputDecoration(
                        labelText: '供应商名称（可选）',
                        hintText: '如：XX食品有限公司',
                        prefixIcon: Icon(Icons.store_outlined),
                        border: OutlineInputBorder(),
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 入库日期
                    _DateTile(
                      label: '入库日期',
                      value: _inboundAt,
                      icon: Icons.calendar_month_outlined,
                      onTap: () => _pickDate(
                        current: _inboundAt,
                        label: '入库日期',
                        onPicked: (d) => setState(() => _inboundAt = d),
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 生产日期（可选）
                    _DateTile(
                      label: '生产日期（可选）',
                      value: _producedAt,
                      icon: Icons.factory_outlined,
                      placeholder: '未填写',
                      onTap: () => _pickDate(
                        current: _producedAt,
                        label: '生产日期',
                        onPicked: (d) => setState(() => _producedAt = d),
                      ),
                    ),
                    const SizedBox(height: 12),

                    // 是否有过期日期
                    SwitchListTile.adaptive(
                      value: _hasExpiry,
                      onChanged: (v) =>
                          setState(() {
                            _hasExpiry = v;
                            if (!v) _expiresAt = null;
                          }),
                      title: const Text('该批次有过期日期'),
                      subtitle: const Text('开启后系统将在临期时提醒'),
                      dense: true,
                      contentPadding: EdgeInsets.zero,
                    ),

                    if (_hasExpiry) ...<Widget>[
                      const SizedBox(height: 8),
                      _DateTile(
                        label: '过期日期 *',
                        value: _expiresAt,
                        icon: Icons.event_busy_outlined,
                        placeholder: '请选择',
                        accentColor:
                            _expiresAt == null ? cs.error : cs.primary,
                        onTap: () => _pickDate(
                          current: _expiresAt ??
                              DateTime.now().add(const Duration(days: 30)),
                          label: '过期日期',
                          onPicked: (d) => setState(() => _expiresAt = d),
                        ),
                      ),
                    ],
                    const SizedBox(height: 12),

                    // 备注
                    TextFormField(
                      controller: _notesCtrl,
                      maxLines: 2,
                      decoration: const InputDecoration(
                        labelText: '备注（可选）',
                        prefixIcon: Icon(Icons.notes_rounded),
                        border: OutlineInputBorder(),
                      ),
                    ),
                    const SizedBox(height: 20),

                    // 按钮区
                    Row(
                      children: <Widget>[
                        Expanded(
                          child: OutlinedButton(
                            onPressed: () => Navigator.of(context).pop(null),
                            child: const Text('跳过'),
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          flex: 2,
                          child: FilledButton.icon(
                            onPressed: submitting ? null : _submit,
                            icon: submitting
                                ? const SizedBox(
                                    width: 16,
                                    height: 16,
                                    child: CircularProgressIndicator(
                                        strokeWidth: 2),
                                  )
                                : const Icon(Icons.check_rounded),
                            label: const Text('保存批次'),
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 16),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ────────────────────────────────────────────────────────────────────────────

class _DateTile extends StatelessWidget {
  const _DateTile({
    required this.label,
    required this.value,
    required this.icon,
    required this.onTap,
    this.placeholder = '未选择',
    this.accentColor,
  });

  final String label;
  final DateTime? value;
  final IconData icon;
  final String placeholder;
  final Color? accentColor;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final color = accentColor ?? cs.primary;
    final text = value != null
        ? '${value!.year}-${value!.month.toString().padLeft(2, '0')}-${value!.day.toString().padLeft(2, '0')}'
        : placeholder;

    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(8),
      child: InputDecorator(
        decoration: InputDecoration(
          labelText: label,
          prefixIcon: Icon(icon, color: color),
          border: const OutlineInputBorder(),
          suffixIcon: Icon(Icons.chevron_right, color: cs.onSurfaceVariant),
        ),
        child: Text(
          text,
          style: TextStyle(
              color: value != null ? null : cs.onSurfaceVariant,
              fontSize: 14),
        ),
      ),
    );
  }
}
