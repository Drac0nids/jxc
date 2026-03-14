import 'package:flutter/material.dart';

import '../application/batch_controller.dart';
import '../models/batch_models.dart';
import 'create_batch_sheet.dart';

/// 入库成功后弹出的批次关联弹窗
///
/// 用户可以：
///   1. 选择已有批次（关联本次入库）
///   2. 新建批次
///   3. 跳过（不关联）
///
/// 弹窗为模态，不可通过点击背景关闭。
class BatchLinkSheet extends StatefulWidget {
  const BatchLinkSheet({
    super.key,
    required this.productId,
    required this.productName,
    required this.controller,
  });

  final int productId;
  final String productName;
  final BatchController controller;

  /// 显示弹窗，返回值含义：
  ///   - `null`       → 用户跳过
  ///   - `BatchData`  → 选择或新建的批次
  static Future<BatchData?> show(
    BuildContext context, {
    required int productId,
    required String productName,
    required BatchController controller,
  }) {
    return showModalBottomSheet<BatchData>(
      context: context,
      isScrollControlled: true,
      isDismissible: false,          // 不允许点背景关闭
      enableDrag: false,             // 不允许下拉关闭
      useSafeArea: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (_) => BatchLinkSheet(
        productId: productId,
        productName: productName,
        controller: controller,
      ),
    );
  }

  @override
  State<BatchLinkSheet> createState() => _BatchLinkSheetState();
}

class _BatchLinkSheetState extends State<BatchLinkSheet>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;

  // ── 选已有批次 ────────────────────────────────────────────────────────────
  List<BatchData> _batches = [];
  bool _loadingBatches = true;
  String? _loadError;
  BatchData? _selectedBatch;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    _loadBatches();
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  Future<void> _loadBatches() async {
    setState(() {
      _loadingBatches = true;
      _loadError = null;
    });
    try {
      final list = await widget.controller.repository.listBatches(
        productId: widget.productId,
        onlyActive: true,
      );
      if (mounted) {
        setState(() {
          _batches = list;
          _loadingBatches = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _loadError = '加载批次失败，请下拉重试';
          _loadingBatches = false;
        });
      }
    }
  }

  // ── 新建批次（委托给 CreateBatchSheet）──────────────────────────────────────

  Future<void> _openCreateBatch() async {
    // 先关闭当前 sheet，再弹出 CreateBatchSheet
    // 使用 Navigator.pop + then 串联，简单起见直接在此弹出新建表单
    final BatchData? created = await CreateBatchSheet.show(
      context,
      productId: widget.productId,
      productName: widget.productName,
      controller: widget.controller,
    );
    if (!mounted) return;
    // 无论用户是否在 CreateBatchSheet 跳过，都关闭本 sheet
    // created == null 表示在新建表单中跳过
    Navigator.of(context).pop(created);
  }

  // ── 确认关联已有批次 ──────────────────────────────────────────────────────

  void _confirmLink() {
    if (_selectedBatch == null) return;
    Navigator.of(context).pop(_selectedBatch);
  }

  // ── 跳过 ──────────────────────────────────────────────────────────────────

  void _skip() => Navigator.of(context).pop(null);

  // ── Build ─────────────────────────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;

    return Padding(
      padding: EdgeInsets.only(
        bottom: MediaQuery.of(context).viewInsets.bottom,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          // 把手
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
                        '关联批次',
                        style: TextStyle(
                            fontSize: 16, fontWeight: FontWeight.w700),
                      ),
                      Text(
                        widget.productName,
                        style:
                            TextStyle(fontSize: 12, color: cs.onSurfaceVariant),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
                // 跳过按钮（右上角）
                TextButton(
                  onPressed: _skip,
                  child: const Text('跳过'),
                ),
              ],
            ),
          ),

          // TabBar
          const SizedBox(height: 8),
          TabBar(
            controller: _tabController,
            tabs: const <Tab>[
              Tab(text: '选择已有批次'),
              Tab(text: '新建批次'),
            ],
          ),

          // Tab 内容（固定高度，避免 BottomSheet 高度不稳定）
          SizedBox(
            height: 320,
            child: TabBarView(
              controller: _tabController,
              children: <Widget>[
                _buildExistingTab(cs),
                _buildCreateTab(cs),
              ],
            ),
          ),

          const SizedBox(height: 8),
        ],
      ),
    );
  }

  // ── Tab 1: 选已有批次 ─────────────────────────────────────────────────────

  Widget _buildExistingTab(ColorScheme cs) {
    if (_loadingBatches) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_loadError != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Text(_loadError!, style: TextStyle(color: cs.error)),
            const SizedBox(height: 12),
            FilledButton.tonal(
              onPressed: _loadBatches,
              child: const Text('重新加载'),
            ),
          ],
        ),
      );
    }

    if (_batches.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(Icons.inventory_2_outlined,
                size: 40, color: cs.onSurfaceVariant),
            const SizedBox(height: 8),
            Text('暂无未售完的批次',
                style: TextStyle(color: cs.onSurfaceVariant)),
            const SizedBox(height: 4),
            Text('请切换到"新建批次"标签创建',
                style: TextStyle(fontSize: 12, color: cs.onSurfaceVariant)),
          ],
        ),
      );
    }

    return Column(
      children: <Widget>[
        Expanded(
          child: RefreshIndicator(
            onRefresh: _loadBatches,
            child: ListView.separated(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              itemCount: _batches.length,
              separatorBuilder: (_, __) => const SizedBox(height: 6),
              itemBuilder: (_, int i) {
                final BatchData b = _batches[i];
                final bool selected = _selectedBatch?.id == b.id;
                return _BatchSelectTile(
                  batch: b,
                  selected: selected,
                  onTap: () => setState(() => _selectedBatch = b),
                  cs: cs,
                );
              },
            ),
          ),
        ),
        // 确认按钮
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 4, 16, 12),
          child: SizedBox(
            width: double.infinity,
            child: FilledButton.icon(
              onPressed: _selectedBatch == null ? null : _confirmLink,
              icon: const Icon(Icons.link_rounded),
              label: Text(_selectedBatch == null ? '选择一条批次后确认' : '关联所选批次'),
            ),
          ),
        ),
      ],
    );
  }

  // ── Tab 2: 新建批次 ───────────────────────────────────────────────────────

  Widget _buildCreateTab(ColorScheme cs) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(Icons.add_box_outlined, size: 48, color: cs.primary),
            const SizedBox(height: 12),
            const Text(
              '为本次入库新建一个批次',
              style: TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
            ),
            const SizedBox(height: 6),
            Text(
              '填写批次号、生产日期、过期日期等信息，\n方便后续效期预警。',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 13, color: cs.onSurfaceVariant),
            ),
            const SizedBox(height: 24),
            SizedBox(
              width: double.infinity,
              child: FilledButton.icon(
                onPressed: _openCreateBatch,
                icon: const Icon(Icons.add_rounded),
                label: const Text('填写批次信息'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// ── BatchSelectTile ──────────────────────────────────────────────────────────

class _BatchSelectTile extends StatelessWidget {
  const _BatchSelectTile({
    required this.batch,
    required this.selected,
    required this.onTap,
    required this.cs,
  });

  final BatchData batch;
  final bool selected;
  final VoidCallback onTap;
  final ColorScheme cs;

  Color get _levelColor {
    return switch (batch.expiryLevel) {
      BatchExpiryLevel.expired => Colors.grey,
      BatchExpiryLevel.critical => Colors.red,
      BatchExpiryLevel.warning => Colors.orange,
      BatchExpiryLevel.notice => Colors.amber,
      _ => Colors.green,
    };
  }

  String _fmtDate(DateTime? d) {
    if (d == null) return '—';
    return '${d.year}-${d.month.toString().padLeft(2, '0')}-${d.day.toString().padLeft(2, '0')}';
  }

  @override
  Widget build(BuildContext context) {
    final Color borderColor = selected ? cs.primary : Colors.transparent;
    final Color bgColor =
        selected ? cs.primaryContainer.withOpacity(0.4) : cs.surfaceContainerLow;

    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(10),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 150),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        decoration: BoxDecoration(
          color: bgColor,
          borderRadius: BorderRadius.circular(10),
          border: Border.all(color: borderColor, width: selected ? 2 : 0),
        ),
        child: Row(
          children: <Widget>[
            // 效期色标
            Container(
              width: 6,
              height: 36,
              decoration: BoxDecoration(
                color: _levelColor,
                borderRadius: BorderRadius.circular(3),
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    batch.lotNumber.isEmpty ? '（无批次号）' : batch.lotNumber,
                    style: const TextStyle(
                        fontSize: 13, fontWeight: FontWeight.w600),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    '入库 ${_fmtDate(batch.inboundAt)}  ·  '
                    '过期 ${_fmtDate(batch.expiresAt)}',
                    style: TextStyle(
                        fontSize: 11, color: cs.onSurfaceVariant),
                  ),
                ],
              ),
            ),
            if (selected)
              Icon(Icons.check_circle_rounded, color: cs.primary, size: 20),
          ],
        ),
      ),
    );
  }
}
