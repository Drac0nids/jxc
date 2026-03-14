import 'package:flutter/material.dart';

import '../application/category_controller.dart';
import '../models/category_models.dart';

/// 分类管理页（OWNER 专属）
/// 树形展示 + 增删改
class CategoryManagementPage extends StatefulWidget {
  const CategoryManagementPage({super.key, required this.controller});

  final CategoryController controller;

  @override
  State<CategoryManagementPage> createState() =>
      _CategoryManagementPageState();
}

class _CategoryManagementPageState extends State<CategoryManagementPage> {
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
        title: const Text('分类管理'),
        actions: <Widget>[
          IconButton(
            tooltip: '新增大类',
            icon: const Icon(Icons.add_rounded),
            onPressed: () => _showCreateDialog(context, parentId: null, level: 1),
          ),
        ],
      ),
      body: AnimatedBuilder(
        animation: widget.controller,
        builder: (BuildContext context, Widget? child) {
          final bool loading = widget.controller.loading;
          final String? error = widget.controller.error;
          final List<CategoryNode> tree = widget.controller.tree;

          if (loading && tree.isEmpty) {
            return const Center(child: CircularProgressIndicator());
          }

          if (error != null && tree.isEmpty) {
            return Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Icon(Icons.error_outline,
                      color: Theme.of(context).colorScheme.error, size: 48),
                  const SizedBox(height: 8),
                  Text(error,
                      style:
                          TextStyle(color: Theme.of(context).colorScheme.error),
                      textAlign: TextAlign.center),
                  const SizedBox(height: 12),
                  FilledButton(
                      onPressed: widget.controller.load,
                      child: const Text('重试')),
                ],
              ),
            );
          }

          if (tree.isEmpty) {
            return Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Icon(Icons.category_outlined,
                      size: 64, color: Colors.grey.shade300),
                  const SizedBox(height: 12),
                  const Text('暂无分类，点击右上角 + 新增大类',
                      style: TextStyle(color: Colors.grey)),
                ],
              ),
            );
          }

          return RefreshIndicator(
            onRefresh: widget.controller.load,
            child: ListView(
              padding: const EdgeInsets.all(12),
              children: tree
                  .map((CategoryNode l1) => _Level1Tile(
                        node: l1,
                        onAddChild: () => _showCreateDialog(context,
                            parentId: l1.id, level: 2),
                        onRename: () =>
                            _showRenameDialog(context, node: l1),
                        onDelete: () => _confirmDelete(context, node: l1),
                        onAddGrandchild: (CategoryNode l2) =>
                            _showCreateDialog(context,
                                parentId: l2.id, level: 3),
                        onRenameChild: (CategoryNode child) =>
                            _showRenameDialog(context, node: child),
                        onDeleteChild: (CategoryNode child) =>
                            _confirmDelete(context, node: child),
                      ))
                  .toList(),
            ),
          );
        },
      ),
    );
  }

  Future<void> _showCreateDialog(
    BuildContext context, {
    required int? parentId,
    required int level,
  }) async {
    final String levelLabel = level == 1 ? '大类' : level == 2 ? '中类' : '小类';
    final TextEditingController nameCtrl = TextEditingController();

    final String? name = await showDialog<String>(
      context: context,
      builder: (BuildContext ctx) => AlertDialog(
        title: Text('新增$levelLabel'),
        content: TextField(
          controller: nameCtrl,
          autofocus: true,
          decoration: InputDecoration(
            labelText: '分类名称',
            hintText: '例如：食品饮料',
            border: const OutlineInputBorder(),
            isDense: true,
          ),
        ),
        actions: <Widget>[
          TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: const Text('取消')),
          FilledButton(
            onPressed: () {
              final String n = nameCtrl.text.trim();
              if (n.isNotEmpty) Navigator.of(ctx).pop(n);
            },
            child: const Text('创建'),
          ),
        ],
      ),
    );

    nameCtrl.dispose();
    if (name == null || !mounted) return;

    final messenger = ScaffoldMessenger.of(context);
    final errorColor = Theme.of(context).colorScheme.error;

    await widget.controller.createCategory(
      CreateCategoryRequest(name: name, parentId: parentId),
    );

    if (mounted && widget.controller.error != null) {
      // ignore: use_build_context_synchronously
      messenger.showSnackBar(
        SnackBar(
          content: Text(widget.controller.error!),
          // ignore: use_build_context_synchronously
          backgroundColor: errorColor,
        ),
      );
    }
  }

  Future<void> _showRenameDialog(
    BuildContext context, {
    required CategoryNode node,
  }) async {
    final TextEditingController nameCtrl =
        TextEditingController(text: node.name);

    final String? newName = await showDialog<String>(
      context: context,
      builder: (BuildContext ctx) => AlertDialog(
        title: const Text('重命名分类'),
        content: TextField(
          controller: nameCtrl,
          autofocus: true,
          decoration: const InputDecoration(
            labelText: '分类名称',
            border: OutlineInputBorder(),
            isDense: true,
          ),
        ),
        actions: <Widget>[
          TextButton(
              onPressed: () => Navigator.of(ctx).pop(),
              child: const Text('取消')),
          FilledButton(
            onPressed: () {
              final String n = nameCtrl.text.trim();
              if (n.isNotEmpty) Navigator.of(ctx).pop(n);
            },
            child: const Text('保存'),
          ),
        ],
      ),
    );

    nameCtrl.dispose();
    if (newName == null || !mounted) return;
    await widget.controller.updateCategory(
      node.id,
      UpdateCategoryRequest(name: newName),
    );
  }

  Future<void> _confirmDelete(
    BuildContext context, {
    required CategoryNode node,
  }) async {
    final bool confirmed = await showDialog<bool>(
          context: context,
          builder: (BuildContext ctx) => AlertDialog(
            title: const Text('确认删除'),
            content: Text(
                '确定删除分类「${node.name}」吗？\n\n注：有子分类或在用商品时无法删除。'),
            actions: <Widget>[
              TextButton(
                  onPressed: () => Navigator.of(ctx).pop(false),
                  child: const Text('取消')),
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
    final messenger = ScaffoldMessenger.of(context);
    final errorColor = Theme.of(context).colorScheme.error;
    final bool ok = await widget.controller.deleteCategory(node.id);
    if (mounted && !ok) {
      // ignore: use_build_context_synchronously
      messenger.showSnackBar(
        SnackBar(
          content: Text(widget.controller.error ?? '删除失败'),
          // ignore: use_build_context_synchronously
          backgroundColor: errorColor,
        ),
      );
    }
  }
}

// ── 树形 Tile 组件 ──────────────────────────────────────────────────────────────

class _Level1Tile extends StatefulWidget {
  const _Level1Tile({
    required this.node,
    required this.onAddChild,
    required this.onRename,
    required this.onDelete,
    required this.onAddGrandchild,
    required this.onRenameChild,
    required this.onDeleteChild,
  });

  final CategoryNode node;
  final VoidCallback onAddChild;
  final VoidCallback onRename;
  final VoidCallback onDelete;
  final void Function(CategoryNode) onAddGrandchild;
  final void Function(CategoryNode) onRenameChild;
  final void Function(CategoryNode) onDeleteChild;

  @override
  State<_Level1Tile> createState() => _Level1TileState();
}

class _Level1TileState extends State<_Level1Tile> {
  bool _expanded = true;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final bool hasChildren = widget.node.children.isNotEmpty;

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: Column(
        children: <Widget>[
          // 大类行
          ListTile(
            leading: CircleAvatar(
              radius: 15,
              backgroundColor: colorScheme.primaryContainer,
              child: Icon(Icons.folder_rounded,
                  size: 16, color: colorScheme.primary),
            ),
            title: Text(widget.node.name,
                style: const TextStyle(fontWeight: FontWeight.w600)),
            subtitle: Text('${widget.node.children.length} 个中类',
                style: const TextStyle(fontSize: 12)),
            trailing: Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                if (hasChildren)
                  IconButton(
                    icon: Icon(
                        _expanded ? Icons.expand_less : Icons.expand_more),
                    onPressed: () =>
                        setState(() => _expanded = !_expanded),
                    tooltip: _expanded ? '收起' : '展开',
                  ),
                _PopupMenu(
                  onAddChild: widget.onAddChild,
                  onRename: widget.onRename,
                  onDelete: widget.onDelete,
                  addChildLabel: '添加中类',
                ),
              ],
            ),
          ),
          // 中类列表
          if (_expanded && hasChildren)
            Padding(
              padding: const EdgeInsets.only(left: 16, bottom: 8),
              child: Column(
                children: widget.node.children
                    .map((CategoryNode l2) => _Level2Tile(
                          node: l2,
                          onAddChild: () =>
                              widget.onAddGrandchild(l2),
                          onRename: () =>
                              widget.onRenameChild(l2),
                          onDelete: () =>
                              widget.onDeleteChild(l2),
                          onRenameChild: widget.onRenameChild,
                          onDeleteChild: widget.onDeleteChild,
                        ))
                    .toList(),
              ),
            ),
        ],
      ),
    );
  }
}

class _Level2Tile extends StatelessWidget {
  const _Level2Tile({
    required this.node,
    required this.onAddChild,
    required this.onRename,
    required this.onDelete,
    required this.onRenameChild,
    required this.onDeleteChild,
  });

  final CategoryNode node;
  final VoidCallback onAddChild;
  final VoidCallback onRename;
  final VoidCallback onDelete;
  final void Function(CategoryNode) onRenameChild;
  final void Function(CategoryNode) onDeleteChild;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        ListTile(
          leading: CircleAvatar(
            radius: 13,
            backgroundColor: colorScheme.secondaryContainer,
            child: Icon(Icons.subdirectory_arrow_right_rounded,
                size: 14, color: colorScheme.secondary),
          ),
          title: Text(node.name,
              style: const TextStyle(fontWeight: FontWeight.w500)),
          subtitle: Text('${node.children.length} 个小类',
              style: const TextStyle(fontSize: 11)),
          dense: true,
          trailing: _PopupMenu(
            onAddChild: onAddChild,
            onRename: onRename,
            onDelete: onDelete,
            addChildLabel: '添加小类',
          ),
        ),
        // 小类标签
        if (node.children.isNotEmpty)
          Padding(
            padding: const EdgeInsets.only(left: 16, bottom: 8),
            child: Wrap(
              spacing: 6,
              runSpacing: 4,
              children: node.children
                  .map((CategoryNode l3) => Chip(
                        materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                        label: Text(l3.name,
                            style: const TextStyle(fontSize: 12)),
                        deleteIcon:
                            const Icon(Icons.edit_rounded, size: 14),
                        onDeleted: () => onRenameChild(l3),
                      ))
                  .toList(),
            ),
          ),
      ],
    );
  }
}

class _PopupMenu extends StatelessWidget {
  const _PopupMenu({
    required this.onAddChild,
    required this.onRename,
    required this.onDelete,
    required this.addChildLabel,
  });

  final VoidCallback onAddChild;
  final VoidCallback onRename;
  final VoidCallback onDelete;
  final String addChildLabel;

  @override
  Widget build(BuildContext context) {
    return PopupMenuButton<String>(
      itemBuilder: (_) => <PopupMenuEntry<String>>[
        PopupMenuItem<String>(
          value: 'add',
          child: Row(children: <Widget>[
            const Icon(Icons.add_circle_outline, size: 18),
            const SizedBox(width: 8),
            Text(addChildLabel),
          ]),
        ),
        const PopupMenuDivider(),
        const PopupMenuItem<String>(
          value: 'rename',
          child: Row(children: <Widget>[
            Icon(Icons.drive_file_rename_outline, size: 18),
            SizedBox(width: 8),
            Text('重命名'),
          ]),
        ),
        PopupMenuItem<String>(
          value: 'delete',
          child: Row(children: <Widget>[
            Icon(Icons.delete_outline,
                size: 18, color: Theme.of(context).colorScheme.error),
            const SizedBox(width: 8),
            Text('删除',
                style: TextStyle(
                    color: Theme.of(context).colorScheme.error)),
          ]),
        ),
      ],
      onSelected: (String value) {
        if (value == 'add') onAddChild();
        if (value == 'rename') onRename();
        if (value == 'delete') onDelete();
      },
    );
  }
}
