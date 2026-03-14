import 'package:flutter/material.dart';
import '../models/category_models.dart';

/// 三级级联分类选择器（BottomSheet）
/// 使用：
/// ```dart
/// final int? categoryId = await CategoryPickerSheet.show(context, tree: tree, initialId: product.categoryId);
/// ```
class CategoryPickerSheet extends StatefulWidget {
  const CategoryPickerSheet({
    super.key,
    required this.tree,
    this.initialId,
  });

  final List<CategoryNode> tree;
  final int? initialId;

  static Future<int?> show(
    BuildContext context, {
    required List<CategoryNode> tree,
    int? initialId,
  }) {
    return showModalBottomSheet<int?>(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (_) => CategoryPickerSheet(tree: tree, initialId: initialId),
    );
  }

  @override
  State<CategoryPickerSheet> createState() => _CategoryPickerSheetState();
}

class _CategoryPickerSheetState extends State<CategoryPickerSheet> {
  // 三列选中索引 (-1=未选中)
  int _level1Index = -1;
  int _level2Index = -1;
  int _level3Index = -1;

  @override
  void initState() {
    super.initState();
    _initFromId(widget.initialId);
  }

  /// 根据 categoryId 反查并初始化三列状态
  void _initFromId(int? id) {
    if (id == null) return;
    for (int i = 0; i < widget.tree.length; i++) {
      final l1 = widget.tree[i];
      if (l1.id == id) {
        _level1Index = i;
        return;
      }
      for (int j = 0; j < l1.children.length; j++) {
        final l2 = l1.children[j];
        if (l2.id == id) {
          _level1Index = i;
          _level2Index = j;
          return;
        }
        for (int k = 0; k < l2.children.length; k++) {
          final l3 = l2.children[k];
          if (l3.id == id) {
            _level1Index = i;
            _level2Index = j;
            _level3Index = k;
            return;
          }
        }
      }
    }
  }

  List<CategoryNode> get _level1Items => widget.tree;

  List<CategoryNode> get _level2Items =>
      _level1Index >= 0 && _level1Index < _level1Items.length
          ? _level1Items[_level1Index].children
          : <CategoryNode>[];

  List<CategoryNode> get _level3Items =>
      _level2Index >= 0 && _level2Index < _level2Items.length
          ? _level2Items[_level2Index].children
          : <CategoryNode>[];

  /// 当前选中的最末层 categoryId
  int? get _selectedId {
    if (_level1Index < 0) return null;
    if (_level2Index < 0) return _level1Items[_level1Index].id;
    if (_level3Index < 0) {
      return _level2Items[_level2Index].id;
    }
    return _level3Items[_level3Index].id;
  }

  String? get _selectedPath {
    if (_level1Index < 0) return null;
    final l1 = _level1Items[_level1Index].name;
    if (_level2Index < 0) return l1;
    final l2 = _level2Items[_level2Index].name;
    if (_level3Index < 0) return '$l1 > $l2';
    return '$l1 > $l2 > ${_level3Items[_level3Index].name}';
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final mediaHeight = MediaQuery.of(context).size.height;

    return SizedBox(
      height: mediaHeight * 0.65,
      child: Column(
        children: <Widget>[
          // 顶部拖拽条
          const SizedBox(height: 8),
          Container(
            width: 40,
            height: 4,
            decoration: BoxDecoration(
              color: Colors.grey.shade300,
              borderRadius: BorderRadius.circular(2),
            ),
          ),
          const SizedBox(height: 12),
          // 标题
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              children: <Widget>[
                Icon(Icons.category_outlined,
                    size: 20, color: colorScheme.primary),
                const SizedBox(width: 8),
                const Text('选择商品分类',
                    style:
                        TextStyle(fontSize: 17, fontWeight: FontWeight.w600)),
              ],
            ),
          ),
          const SizedBox(height: 8),
          if (widget.tree.isEmpty)
            Expanded(
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Icon(Icons.category_outlined,
                        size: 48, color: Colors.grey.shade300),
                    const SizedBox(height: 8),
                    const Text('暂无分类，请先在分类管理中添加',
                        style: TextStyle(color: Colors.grey)),
                  ],
                ),
              ),
            )
          else ...<Widget>[
            // 三列选择区
            Expanded(
              child: Row(
                children: <Widget>[
                  // 大类
                  _buildColumn(
                    label: '大类',
                    items: _level1Items,
                    selectedIndex: _level1Index,
                    onTap: (i) => setState(() {
                      _level1Index = i;
                      _level2Index = -1;
                      _level3Index = -1;
                    }),
                    colorScheme: colorScheme,
                  ),
                  // 中类
                  _buildColumn(
                    label: '中类',
                    items: _level2Items,
                    selectedIndex: _level2Index,
                    onTap: _level2Items.isEmpty
                        ? null
                        : (i) => setState(() {
                              _level2Index = i;
                              _level3Index = -1;
                            }),
                    colorScheme: colorScheme,
                    dimmed: _level1Index < 0,
                  ),
                  // 小类
                  _buildColumn(
                    label: '小类',
                    items: _level3Items,
                    selectedIndex: _level3Index,
                    onTap: _level3Items.isEmpty
                        ? null
                        : (i) => setState(() {
                              _level3Index = i;
                            }),
                    colorScheme: colorScheme,
                    dimmed: _level2Index < 0,
                  ),
                ],
              ),
            ),
            // 当前路径预览 + 确认按钮
            Container(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 16),
              decoration: BoxDecoration(
                color: colorScheme.surface,
                border: Border(
                    top: BorderSide(color: Colors.grey.shade200, width: 1)),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  if (_selectedPath != null)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: Text(
                        '已选：$_selectedPath',
                        style: TextStyle(
                          fontSize: 13,
                          color: colorScheme.primary,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                    ),
                  Row(
                    children: <Widget>[
                      OutlinedButton(
                        onPressed: () => Navigator.of(context).pop(null),
                        child: const Text('清除分类'),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: FilledButton(
                          onPressed: () =>
                              Navigator.of(context).pop(_selectedId),
                          child: const Text('确认选择'),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildColumn({
    required String label,
    required List<CategoryNode> items,
    required int selectedIndex,
    required void Function(int)? onTap,
    required ColorScheme colorScheme,
    bool dimmed = false,
  }) {
    return Expanded(
      child: Column(
        children: <Widget>[
          Container(
            color: colorScheme.surfaceContainerHighest,
            padding: const EdgeInsets.symmetric(vertical: 6),
            width: double.infinity,
            child: Text(
              label,
              textAlign: TextAlign.center,
              style: TextStyle(
                fontSize: 12,
                color: Colors.grey.shade600,
                fontWeight: FontWeight.w500,
              ),
            ),
          ),
          Expanded(
            child: dimmed
                ? Container(color: Colors.grey.shade50)
                : items.isEmpty
                    ? Center(
                        child: Text(
                          '无',
                          style: TextStyle(
                              color: Colors.grey.shade400, fontSize: 13),
                        ),
                      )
                    : ListView.builder(
                        itemCount: items.length,
                        itemBuilder: (_, i) {
                          final node = items[i];
                          final bool selected = i == selectedIndex;
                          return InkWell(
                            onTap: onTap == null ? null : () => onTap(i),
                            child: Container(
                              padding: const EdgeInsets.symmetric(
                                  horizontal: 10, vertical: 12),
                              decoration: BoxDecoration(
                                color: selected
                                    ? colorScheme.primaryContainer
                                    : null,
                                border: Border(
                                    bottom: BorderSide(
                                        color: Colors.grey.shade100)),
                              ),
                              child: Row(
                                children: <Widget>[
                                  Expanded(
                                    child: Text(
                                      node.name,
                                      style: TextStyle(
                                        fontSize: 14,
                                        fontWeight: selected
                                            ? FontWeight.w600
                                            : FontWeight.normal,
                                        color: selected
                                            ? colorScheme.primary
                                            : null,
                                      ),
                                    ),
                                  ),
                                  if (node.children.isNotEmpty)
                                    Icon(Icons.chevron_right,
                                        size: 16,
                                        color: Colors.grey.shade400),
                                ],
                              ),
                            ),
                          );
                        },
                      ),
          ),
        ],
      ),
    );
  }
}
