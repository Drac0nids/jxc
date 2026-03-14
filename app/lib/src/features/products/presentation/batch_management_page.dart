import 'package:flutter/material.dart';

import '../application/batch_controller.dart';
import '../models/batch_models.dart';
import 'create_batch_sheet.dart';

/// 某个商品的批次列表页面
class BatchManagementPage extends StatefulWidget {
  const BatchManagementPage({
    super.key,
    required this.productId,
    required this.productName,
    required this.controller,
  });

  final int productId;
  final String productName;
  final BatchController controller;

  @override
  State<BatchManagementPage> createState() => _BatchManagementPageState();
}

class _BatchManagementPageState extends State<BatchManagementPage> {
  bool _onlyActive = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    await widget.controller.loadBatchesForProduct(
      widget.productId,
      onlyActive: _onlyActive,
    );
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.controller,
      builder: (BuildContext ctx, _) {
        final batches = widget.controller.batches;
        final loading = widget.controller.loading;
        final error = widget.controller.errorMessage;

        return Scaffold(
          appBar: AppBar(
            title: const Text('批次管理'),
            actions: <Widget>[
              // 只显示未售完切换
              FilterChip(
                label: Text(_onlyActive ? '未售完' : '全部'),
                selected: _onlyActive,
                onSelected: (v) {
                  setState(() => _onlyActive = v);
                  _load();
                },
                avatar: Icon(
                  _onlyActive
                      ? Icons.radio_button_checked
                      : Icons.radio_button_off,
                  size: 16,
                ),
              ),
              const SizedBox(width: 8),
            ],
          ),
          floatingActionButton: FloatingActionButton.extended(
            onPressed: () async {
              final result = await CreateBatchSheet.show(
                context,
                productId: widget.productId,
                productName: widget.productName,
                controller: widget.controller,
              );
              if (result != null) _load();
            },
            icon: const Icon(Icons.add),
            label: const Text('新建批次'),
          ),
          body: RefreshIndicator(
            onRefresh: _load,
            child: loading
                ? const Center(child: CircularProgressIndicator())
                : error != null
                    ? _ErrorView(message: error, onRetry: _load)
                    : batches.isEmpty
                        ? _EmptyView(
                            onlyActive: _onlyActive,
                            onAdd: () async {
                              final result = await CreateBatchSheet.show(
                                context,
                                productId: widget.productId,
                                productName: widget.productName,
                                controller: widget.controller,
                              );
                              if (result != null) _load();
                            },
                          )
                        : ListView.builder(
                            padding: const EdgeInsets.fromLTRB(16, 12, 16, 100),
                            itemCount: batches.length,
                            itemBuilder: (_, int i) => _BatchCard(
                              batch: batches[i],
                              controller: widget.controller,
                              onRefresh: _load,
                            ),
                          ),
          ),
        );
      },
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// 批次卡片
// ════════════════════════════════════════════════════════════════════════════

class _BatchCard extends StatelessWidget {
  const _BatchCard({
    required this.batch,
    required this.controller,
    required this.onRefresh,
  });

  final BatchData batch;
  final BatchController controller;
  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final expiryColor = _expiryColor(batch.expiryLevel, cs);

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            // 批次号 + 状态标签
            Row(
              children: <Widget>[
                Icon(Icons.inventory_rounded, size: 16, color: cs.primary),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    '批次：${batch.lotNumber}',
                    style: const TextStyle(
                        fontWeight: FontWeight.w600, fontSize: 14),
                  ),
                ),
                if (batch.isSoldOut)
                  _Chip(label: '已售完', color: Colors.grey.shade600)
                else if (batch.expiryLevel != null)
                  _Chip(
                    label: _expiryLabel(batch),
                    color: expiryColor,
                  ),
              ],
            ),
            const SizedBox(height: 8),

            // 日期信息
            _InfoRow(
              icon: Icons.calendar_today_outlined,
              label: '入库日期',
              value: _fmtDate(batch.inboundAt),
            ),
            if (batch.producedAt != null)
              _InfoRow(
                icon: Icons.factory_outlined,
                label: '生产日期',
                value: _fmtDate(batch.producedAt!),
              ),
            if (batch.expiresAt != null)
              _InfoRow(
                icon: Icons.event_busy_outlined,
                label: '过期日期',
                value: _fmtDate(batch.expiresAt!),
                valueColor: expiryColor,
                bold: batch.expiryLevel != null && !batch.isSoldOut,
              ),
            if (batch.notes != null && batch.notes!.isNotEmpty)
              _InfoRow(
                icon: Icons.notes_rounded,
                label: '备注',
                value: batch.notes!,
              ),

            // 操作按钮
            if (!batch.isSoldOut) ...<Widget>[
              const Divider(height: 20),
              _ActionRow(
                batch: batch,
                controller: controller,
                onRefresh: onRefresh,
              ),
            ],
          ],
        ),
      ),
    );
  }

  Color _expiryColor(BatchExpiryLevel? level, ColorScheme cs) {
    if (level == null) return cs.primary;
    return switch (level) {
      BatchExpiryLevel.expired => Colors.grey,
      BatchExpiryLevel.critical => Colors.red.shade600,
      BatchExpiryLevel.warning => Colors.orange.shade600,
      BatchExpiryLevel.notice => Colors.amber.shade700,
      BatchExpiryLevel.ok => cs.primary,
    };
  }

  String _expiryLabel(BatchData b) {
    if (b.expiryLevel == BatchExpiryLevel.expired) return '已过期';
    final d = b.daysUntilExpiry;
    if (d == null) return '';
    if (d == 0) return '今天到期';
    return '还剩 $d 天';
  }

  String _fmtDate(DateTime d) =>
      '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';
}

// ────────────────────────────────────────────────────────────────────────────

class _ActionRow extends StatelessWidget {
  const _ActionRow({
    required this.batch,
    required this.controller,
    required this.onRefresh,
  });

  final BatchData batch;
  final BatchController controller;
  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Row(
      mainAxisAlignment: MainAxisAlignment.end,
      children: <Widget>[
        // 删除
        TextButton.icon(
          onPressed: () => _confirmDelete(context),
          icon: Icon(Icons.delete_outline, size: 16, color: cs.error),
          label: Text('删除', style: TextStyle(color: cs.error, fontSize: 12)),
          style: TextButton.styleFrom(
              padding: const EdgeInsets.symmetric(horizontal: 8)),
        ),
        const SizedBox(width: 4),
        // 编辑
        TextButton.icon(
          onPressed: () => _edit(context),
          icon: const Icon(Icons.edit_outlined, size: 16),
          label: const Text('编辑', style: TextStyle(fontSize: 12)),
          style: TextButton.styleFrom(
              padding: const EdgeInsets.symmetric(horizontal: 8)),
        ),
        const SizedBox(width: 4),
        // 标记售完
        FilledButton.icon(
          onPressed: () => _markSoldOut(context),
          icon: const Icon(Icons.check_circle_outline, size: 16),
          label: const Text('标记售完', style: TextStyle(fontSize: 12)),
          style: FilledButton.styleFrom(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6)),
        ),
      ],
    );
  }

  Future<void> _markSoldOut(BuildContext context) async {
    final confirm = await showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        title: const Text('确认售完'),
        content: Text('将批次「${batch.lotNumber}」标记为已售完？'),
        actions: <Widget>[
          TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('取消')),
          FilledButton(
              onPressed: () => Navigator.pop(context, true),
              child: const Text('确认')),
        ],
      ),
    );
    if (confirm == true) {
      await controller.markSoldOut(batch.id);
      onRefresh();
    }
  }

  Future<void> _confirmDelete(BuildContext context) async {
    final confirm = await showDialog<bool>(
      context: context,
      builder: (_) => AlertDialog(
        title: const Text('删除批次'),
        content: Text('确认删除批次「${batch.lotNumber}」？此操作不可恢复。'),
        actions: <Widget>[
          TextButton(
              onPressed: () => Navigator.pop(context, false),
              child: const Text('取消')),
          TextButton(
            onPressed: () => Navigator.pop(context, true),
            style:
                TextButton.styleFrom(foregroundColor: Colors.red),
            child: const Text('删除'),
          ),
        ],
      ),
    );
    if (confirm == true) {
      await controller.deleteBatch(batch.id);
      onRefresh();
    }
  }

  Future<void> _edit(BuildContext context) async {
    await showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (_) => _EditBatchSheet(batch: batch, controller: controller),
    );
    onRefresh();
  }
}

// ════════════════════════════════════════════════════════════════════════════
// 编辑批次弹窗
// ════════════════════════════════════════════════════════════════════════════

class _EditBatchSheet extends StatefulWidget {
  const _EditBatchSheet({required this.batch, required this.controller});
  final BatchData batch;
  final BatchController controller;
  @override
  State<_EditBatchSheet> createState() => _EditBatchSheetState();
}

class _EditBatchSheetState extends State<_EditBatchSheet> {
  late final TextEditingController _lotCtrl;
  late final TextEditingController _notesCtrl;
  DateTime? _producedAt;
  DateTime? _expiresAt;
  bool _hasExpiry = false;

  @override
  void initState() {
    super.initState();
    _lotCtrl = TextEditingController(text: widget.batch.lotNumber);
    _notesCtrl = TextEditingController(text: widget.batch.notes ?? '');
    _producedAt = widget.batch.producedAt;
    _expiresAt = widget.batch.expiresAt;
    _hasExpiry = widget.batch.expiresAt != null;
  }

  @override
  void dispose() {
    _lotCtrl.dispose();
    _notesCtrl.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final req = UpdateBatchRequest(
      lotNumber: _lotCtrl.text.trim().isEmpty ? null : _lotCtrl.text.trim(),
      producedAt: _producedAt,
      expiresAt: _hasExpiry ? _expiresAt : null,
      notes: _notesCtrl.text.trim().isEmpty ? null : _notesCtrl.text.trim(),
    );
    final ok = await widget.controller.updateBatch(widget.batch.id, req);
    if (!mounted) return;
    if (ok != null) Navigator.of(context).pop();
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
    return Padding(
      padding: EdgeInsets.only(
          bottom: MediaQuery.of(context).viewInsets.bottom),
      child: SingleChildScrollView(
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 16, 20, 20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              const Text('编辑批次',
                  style: TextStyle(
                      fontSize: 16, fontWeight: FontWeight.w600)),
              const SizedBox(height: 16),
              TextFormField(
                controller: _lotCtrl,
                decoration: const InputDecoration(
                  labelText: '批次号',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              // 生产日期
              InkWell(
                onTap: () => _pickDate(
                  current: _producedAt,
                  label: '生产日期',
                  onPicked: (d) => setState(() => _producedAt = d),
                ),
                child: InputDecorator(
                  decoration: InputDecoration(
                    labelText: '生产日期（可选）',
                    border: const OutlineInputBorder(),
                    prefixIcon:
                        Icon(Icons.factory_outlined, color: cs.primary),
                    suffixIcon:
                        Icon(Icons.chevron_right, color: cs.onSurfaceVariant),
                  ),
                  child: Text(
                    _producedAt != null
                        ? '${_producedAt!.year}-${_producedAt!.month.toString().padLeft(2, '0')}-${_producedAt!.day.toString().padLeft(2, '0')}'
                        : '未填写',
                    style: TextStyle(
                        fontSize: 14,
                        color: _producedAt != null
                            ? null
                            : cs.onSurfaceVariant),
                  ),
                ),
              ),
              const SizedBox(height: 12),
              SwitchListTile.adaptive(
                value: _hasExpiry,
                onChanged: (v) => setState(() {
                  _hasExpiry = v;
                  if (!v) _expiresAt = null;
                }),
                title: const Text('有过期日期'),
                dense: true,
                contentPadding: EdgeInsets.zero,
              ),
              if (_hasExpiry) ...<Widget>[
                const SizedBox(height: 8),
                InkWell(
                  onTap: () => _pickDate(
                    current: _expiresAt ??
                        DateTime.now().add(const Duration(days: 30)),
                    label: '过期日期',
                    onPicked: (d) => setState(() => _expiresAt = d),
                  ),
                  child: InputDecorator(
                    decoration: InputDecoration(
                      labelText: '过期日期',
                      border: const OutlineInputBorder(),
                      prefixIcon: Icon(Icons.event_busy_outlined,
                          color: _expiresAt == null ? cs.error : cs.primary),
                      suffixIcon: Icon(Icons.chevron_right,
                          color: cs.onSurfaceVariant),
                    ),
                    child: Text(
                      _expiresAt != null
                          ? '${_expiresAt!.year}-${_expiresAt!.month.toString().padLeft(2, '0')}-${_expiresAt!.day.toString().padLeft(2, '0')}'
                          : '请选择',
                      style: TextStyle(
                          fontSize: 14,
                          color: _expiresAt != null
                              ? null
                              : cs.error),
                    ),
                  ),
                ),
              ],
              const SizedBox(height: 12),
              TextFormField(
                controller: _notesCtrl,
                maxLines: 2,
                decoration: const InputDecoration(
                  labelText: '备注',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 20),
              FilledButton(
                onPressed: widget.controller.submitting ? null : _save,
                child: const Text('保存更改'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// 辅助 Widget
// ════════════════════════════════════════════════════════════════════════════

class _InfoRow extends StatelessWidget {
  const _InfoRow({
    required this.icon,
    required this.label,
    required this.value,
    this.valueColor,
    this.bold = false,
  });
  final IconData icon;
  final String label;
  final String value;
  final Color? valueColor;
  final bool bold;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: <Widget>[
          Icon(icon, size: 14, color: cs.onSurfaceVariant),
          const SizedBox(width: 6),
          Text('$label：',
              style: TextStyle(fontSize: 12, color: cs.onSurfaceVariant)),
          Expanded(
            child: Text(
              value,
              style: TextStyle(
                fontSize: 12,
                fontWeight: bold ? FontWeight.w600 : FontWeight.normal,
                color: valueColor,
              ),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}

class _Chip extends StatelessWidget {
  const _Chip({required this.label, required this.color});
  final String label;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withAlpha(26),
        borderRadius: BorderRadius.circular(20),
        border: Border.all(color: color.withAlpha(77)),
      ),
      child: Text(
        label,
        style: TextStyle(fontSize: 11, color: color, fontWeight: FontWeight.w600),
      ),
    );
  }
}

class _EmptyView extends StatelessWidget {
  const _EmptyView({required this.onlyActive, required this.onAdd});
  final bool onlyActive;
  final VoidCallback onAdd;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Icon(Icons.inventory_2_outlined,
              size: 56, color: Colors.grey.shade300),
          const SizedBox(height: 12),
          Text(
            onlyActive ? '无未售完批次' : '暂无批次记录',
            style: const TextStyle(fontSize: 15, fontWeight: FontWeight.w500),
          ),
          const SizedBox(height: 4),
          const Text('每次入库时可为商品添加批次',
              style: TextStyle(fontSize: 13, color: Colors.grey)),
          const SizedBox(height: 16),
          FilledButton.icon(
            onPressed: onAdd,
            icon: const Icon(Icons.add, size: 16),
            label: const Text('新建批次'),
          ),
        ],
      ),
    );
  }
}

class _ErrorView extends StatelessWidget {
  const _ErrorView({required this.message, required this.onRetry});
  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          const Icon(Icons.error_outline, size: 48, color: Colors.red),
          const SizedBox(height: 8),
          Text(message, textAlign: TextAlign.center),
          const SizedBox(height: 12),
          OutlinedButton.icon(
            onPressed: onRetry,
            icon: const Icon(Icons.refresh),
            label: const Text('重试'),
          ),
        ],
      ),
    );
  }
}
