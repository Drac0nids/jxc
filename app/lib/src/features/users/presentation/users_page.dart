import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../core/widgets/brand_ui.dart';
import '../application/users_controller.dart';
import '../models/users_models.dart';

// ════════════════════════════════════════════════════════════════════════════
// Role options
// ════════════════════════════════════════════════════════════════════════════

const List<_RoleOption> _kRoles = <_RoleOption>[
  _RoleOption(value: 'OWNER', label: '老板', color: Color(0xFF8B5CF6)),
  _RoleOption(value: 'ADMIN', label: '副管理员', color: Color(0xFF3B82F6)),
  _RoleOption(value: 'PURCHASER', label: '采购员', color: Color(0xFF10B981)),
  _RoleOption(value: 'SALES', label: '销售员', color: Color(0xFFF59E0B)),
];

class _RoleOption {
  const _RoleOption(
      {required this.value, required this.label, required this.color});
  final String value;
  final String label;
  final Color color;
}

// ════════════════════════════════════════════════════════════════════════════
// UsersPage
// ════════════════════════════════════════════════════════════════════════════

class UsersPage extends StatefulWidget {
  const UsersPage({
    super.key,
    required this.controller,
    required this.currentUserId,
    required this.currentUserRole,
  });
  final UsersController controller;
  final String currentUserId;
  /// 当前登录者的角色（用于过滤可操作用户和可分配角色）
  final String currentUserRole;

  @override
  State<UsersPage> createState() => _UsersPageState();
}

class _UsersPageState extends State<UsersPage> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance
        .addPostFrameCallback((_) => widget.controller.load());
  }

  void _showSnack(String msg) {
    if (!mounted) return;
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(msg)));
  }

  // ── Dialogs ───────────────────────────────────────────────────────────────

  Future<void> _showCreateDialog() async {
    await showDialog<void>(
      context: context,
      builder: (_) => _CreateUserDialog(controller: widget.controller, currentUserRole: widget.currentUserRole),
    );
    final msg = widget.controller.successMessage ??
        widget.controller.errorMessage;
    if (msg != null) {
      _showSnack(msg);
      widget.controller.clearMessages();
    }
  }

  Future<void> _showRoleDialog(UserData user) async {
    await showDialog<void>(
      context: context,
      builder: (_) => _UpdateRoleDialog(controller: widget.controller, user: user, currentUserRole: widget.currentUserRole),
    );
    final msg = widget.controller.successMessage ??
        widget.controller.errorMessage;
    if (msg != null) {
      _showSnack(msg);
      widget.controller.clearMessages();
    }
  }

  Future<void> _showResetPwdDialog(UserData user) async {
    await showDialog<void>(
      context: context,
      builder: (_) =>
          _ResetPasswordDialog(controller: widget.controller, user: user),
    );
    final msg = widget.controller.successMessage ??
        widget.controller.errorMessage;
    if (msg != null) {
      _showSnack(msg);
      widget.controller.clearMessages();
    }
  }

  Future<void> _showDeleteDialog(UserData user) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('删除「${user.name}」'),
        content: Text('确定要删除员工「${user.name}」（@${user.username}）吗？此操作不可撤销。'),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('取消'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: Colors.red),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('确认删除'),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    final ok = await widget.controller.deleteUser(user.id, user.name);
    final msg = ok
        ? widget.controller.successMessage
        : widget.controller.errorMessage;
    if (msg != null && mounted) {
      _showSnack(msg);
      widget.controller.clearMessages();
    }
  }

  // ── Build ─────────────────────────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.controller,
      builder: (BuildContext context, Widget? child) {
        final ctrl = widget.controller;

        return Scaffold(
          appBar: AppBar(
            title: const Text('人员管理'),
            actions: <Widget>[
              IconButton(
                tooltip: '添加员工',
                onPressed: ctrl.submitting ? null : _showCreateDialog,
                icon: const Icon(Icons.person_add_rounded),
              ),
            ],
          ),
          body: RefreshIndicator(
            onRefresh: ctrl.load,
            child: ListView(
              padding: const EdgeInsets.fromLTRB(16, 16, 16, 24),
              children: <Widget>[
                const BrandHeroBanner(
                  title: '人员与权限管理',
                  subtitle: '管理租户成员，分配老板/采购/销售角色',
                  icon: Icons.manage_accounts_rounded,
                  gradientSeedColor: Color(0xFF8B5CF6),
                ),
                const SizedBox(height: 12),

                // ── Role legend ──
                _RoleLegend(),
                const SizedBox(height: 12),

                // ── Error / Loading / List ──
                if (ctrl.loading)
                  ..._buildSkeletons()
                else if (ctrl.list.isEmpty)
                  _EmptyState(onAdd: _showCreateDialog)
                else
                  ...ctrl.list.map(
                    (UserData u) => _UserCard(
                      user: u,
                      isSelf: u.id == widget.currentUserId,
                      currentUserRole: widget.currentUserRole,
                      onRoleTap: () => _showRoleDialog(u),
                      onPwdTap: () => _showResetPwdDialog(u),
                      onDeleteTap: () => _showDeleteDialog(u),
                    ),
                  ),
              ],
            ),
          ),
        );
      },
    );
  }

  List<Widget> _buildSkeletons() {
    final base = Theme.of(context).colorScheme.surfaceContainerHighest;
    return List<Widget>.generate(
      3,
      (_) => Card(
        margin: const EdgeInsets.only(bottom: 10),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: <Widget>[
              Container(
                width: 44,
                height: 44,
                decoration: BoxDecoration(
                  color: base.withValues(alpha: 0.5),
                  shape: BoxShape.circle,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Container(
                      width: 100,
                      height: 13,
                      decoration: BoxDecoration(
                          color: base.withValues(alpha: 0.5),
                          borderRadius: BorderRadius.circular(4)),
                    ),
                    const SizedBox(height: 8),
                    Container(
                      width: 60,
                      height: 11,
                      decoration: BoxDecoration(
                          color: base.withValues(alpha: 0.4),
                          borderRadius: BorderRadius.circular(4)),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Role legend
// ════════════════════════════════════════════════════════════════════════════

class _RoleLegend extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
      decoration: BoxDecoration(
        color: Theme.of(context)
            .colorScheme
            .surfaceContainerHighest
            .withValues(alpha: 0.4),
        borderRadius: BorderRadius.circular(12),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text('角色权限说明',
              style: Theme.of(context)
                  .textTheme
                  .labelMedium
                  ?.copyWith(fontWeight: FontWeight.w700)),
          const SizedBox(height: 8),
          ..._kRoles.map(
            (r) => Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Row(
                children: <Widget>[
                  Container(
                    width: 8,
                    height: 8,
                    decoration: BoxDecoration(
                        color: r.color, shape: BoxShape.circle),
                  ),
                  const SizedBox(width: 8),
                  Text('${r.label}：',
                      style: const TextStyle(
                          fontSize: 13, fontWeight: FontWeight.w600)),
                  Expanded(
                    child: Text(
                      _roleDesc(r.value),
                      style: TextStyle(
                          fontSize: 12,
                          color: Theme.of(context)
                              .colorScheme
                              .onSurfaceVariant),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  String _roleDesc(String role) {
    switch (role) {
      case 'OWNER':
        return '全部功能，包含人员管理、经营看板、进销存所有模块';
      case 'PURCHASER':
        return '采购入库、库存盘点、商品管理、入库记录';
      case 'SALES':
        return '销售出库、销售趋势、订单下钻、商品查询';
      default:
        return '';
    }
  }
}

// ════════════════════════════════════════════════════════════════════════════
// User Card
// ════════════════════════════════════════════════════════════════════════════

class _UserCard extends StatelessWidget {
  const _UserCard({
    required this.user,
    required this.isSelf,
    required this.currentUserRole,
    required this.onRoleTap,
    required this.onPwdTap,
    required this.onDeleteTap,
  });

  final UserData user;
  final bool isSelf;
  final String currentUserRole;
  final VoidCallback onRoleTap;
  final VoidCallback onPwdTap;
  final VoidCallback onDeleteTap;

  /// 角色层级（数值越大权限越高）
  int _roleRank(String role) {
    switch (role.toUpperCase()) {
      case 'OWNER': return 100;
      case 'ADMIN': return 50;
      default: return 10;
    }
  }

  /// 操作者是否有权管理目标用户（层级必须高于目标）
  bool _canManage(String targetRole) =>
      _roleRank(currentUserRole) > _roleRank(targetRole);

  Color _roleColor() {
    for (final r in _kRoles) {
      if (r.value == user.role) return r.color;
    }
    return Colors.grey;
  }

  @override
  Widget build(BuildContext context) {
    final color = _roleColor();
    final cs = Theme.of(context).colorScheme;

    return Card(
      margin: const EdgeInsets.only(bottom: 10),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
        child: Row(
          children: <Widget>[
            // Avatar
            CircleAvatar(
              radius: 22,
              backgroundColor: color.withValues(alpha: 0.15),
              child: Text(
                user.name.isNotEmpty ? user.name[0] : '?',
                style: TextStyle(
                    fontSize: 17,
                    fontWeight: FontWeight.w700,
                    color: color),
              ),
            ),
            const SizedBox(width: 12),
            // Name & username
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Row(
                    children: <Widget>[
                      Text(user.name,
                          style: const TextStyle(
                              fontWeight: FontWeight.w700, fontSize: 15)),
                      if (isSelf) ...<Widget>[
                        const SizedBox(width: 6),
                        Container(
                          padding: const EdgeInsets.symmetric(
                              horizontal: 6, vertical: 2),
                          decoration: BoxDecoration(
                            color: Colors.blue.withValues(alpha: 0.12),
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: const Text('你',
                              style: TextStyle(
                                  fontSize: 11,
                                  color: Colors.blue,
                                  fontWeight: FontWeight.w700)),
                        ),
                      ],
                    ],
                  ),
                  const SizedBox(height: 2),
                  GestureDetector(
                    onTap: () {
                      Clipboard.setData(
                          ClipboardData(text: user.username));
                      ScaffoldMessenger.of(context).showSnackBar(
                        SnackBar(
                          content: Text(
                              '已复制用户名：${user.username}'),
                          duration: const Duration(seconds: 2),
                        ),
                      );
                    },
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Text('@${user.username}',
                            style: TextStyle(
                                fontSize: 12,
                                color: cs.onSurfaceVariant)),
                        const SizedBox(width: 3),
                        Icon(Icons.copy_rounded,
                            size: 12,
                            color:
                                cs.onSurfaceVariant.withValues(alpha: 0.5)),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            // Role Badge
            _RoleBadge(role: user.role, color: color),
            const SizedBox(width: 8),
            // Actions menu
            PopupMenuButton<String>(
              icon: const Icon(Icons.more_vert, size: 20),
              tooltip: '操作',
              itemBuilder: (_) => <PopupMenuEntry<String>>[
                // 修改角色：非自己 且 操作者层级 > 目标层级
                if (!isSelf && _canManage(user.role))
                  const PopupMenuItem<String>(
                    value: 'role',
                    child: Row(
                      children: <Widget>[
                        Icon(Icons.badge_rounded, size: 18),
                        SizedBox(width: 8),
                        Text('修改角色'),
                      ],
                    ),
                  ),
                // 重置密码：非自己 且 操作者层级 > 目标层级
                if (!isSelf && _canManage(user.role))
                  const PopupMenuItem<String>(
                    value: 'pwd',
                    child: Row(
                      children: <Widget>[
                        Icon(Icons.lock_reset_rounded, size: 18),
                        SizedBox(width: 8),
                        Text('重置密码'),
                      ],
                    ),
                  ),
                // 删除：非自己 且 操作者层级 > 目标层级
                if (!isSelf && _canManage(user.role))
                  const PopupMenuItem<String>(
                    value: 'delete',
                    child: Row(
                      children: <Widget>[
                        Icon(Icons.person_remove_rounded, size: 18, color: Colors.red),
                        SizedBox(width: 8),
                        Text('删除员工', style: TextStyle(color: Colors.red)),
                      ],
                    ),
                  ),
                // 自己只能看，没有操作项时显示提示
                if (isSelf || !_canManage(user.role))
                  const PopupMenuItem<String>(
                    value: '',
                    enabled: false,
                    child: Text('无可用操作',
                        style: TextStyle(fontSize: 13)),
                  ),
              ],
              onSelected: (v) {
                if (v == 'role') onRoleTap();
                if (v == 'pwd') onPwdTap();
                if (v == 'delete') onDeleteTap();
              },
            ),
          ],
        ),
      ),
    );
  }
}

class _RoleBadge extends StatelessWidget {
  const _RoleBadge({required this.role, required this.color});
  final String role;
  final Color color;

  String _label() {
    for (final r in _kRoles) {
      if (r.value == role) return r.label;
    }
    return role;
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.12),
        border: Border.all(color: color.withValues(alpha: 0.35)),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        _label(),
        style: TextStyle(
            fontSize: 12, fontWeight: FontWeight.w700, color: color),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Empty State
// ════════════════════════════════════════════════════════════════════════════

class _EmptyState extends StatelessWidget {
  const _EmptyState({required this.onAdd});
  final VoidCallback onAdd;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 48),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(Icons.group_off_rounded,
                size: 56,
                color: Theme.of(context)
                    .colorScheme
                    .onSurfaceVariant
                    .withValues(alpha: 0.4)),
            const SizedBox(height: 16),
            const Text('暂无员工',
                style:
                    TextStyle(fontWeight: FontWeight.w600, fontSize: 15)),
            const SizedBox(height: 6),
            Text('点击下方按钮添加第一名员工',
                style: TextStyle(
                    fontSize: 13,
                    color: Theme.of(context)
                        .colorScheme
                        .onSurfaceVariant)),
            const SizedBox(height: 20),
            FilledButton.icon(
              onPressed: onAdd,
              icon: const Icon(Icons.person_add_rounded),
              label: const Text('添加员工'),
            ),
          ],
        ),
      ),
    );
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Create User Dialog
// ════════════════════════════════════════════════════════════════════════════

class _CreateUserDialog extends StatefulWidget {
  const _CreateUserDialog({required this.controller, required this.currentUserRole});
  final UsersController controller;
  final String currentUserRole;

  @override
  State<_CreateUserDialog> createState() => _CreateUserDialogState();
}

class _CreateUserDialogState extends State<_CreateUserDialog> {
  final _usernameCtrl = TextEditingController();
  final _nameCtrl = TextEditingController();
  final _pwdCtrl = TextEditingController();
  final _confirmPwdCtrl = TextEditingController();
  String _selectedRole = 'PURCHASER';
  bool _showPwd = false;

  @override
  void dispose() {
    _usernameCtrl.dispose();
    _nameCtrl.dispose();
    _pwdCtrl.dispose();
    _confirmPwdCtrl.dispose();
    super.dispose();
  }

  String? _validate() {
    if (_usernameCtrl.text.trim().isEmpty) return '请填写登录用户名';
    if (_nameCtrl.text.trim().isEmpty) return '请填写真实姓名';
    if (_pwdCtrl.text.isEmpty) return '请填写密码';
    if (_pwdCtrl.text.length < 6) return '密码至少 6 位';
    if (_pwdCtrl.text != _confirmPwdCtrl.text) return '两次密码不一致';
    return null;
  }

  Future<void> _submit() async {
    final err = _validate();
    if (err != null) {
      ScaffoldMessenger.of(context)
          .showSnackBar(SnackBar(content: Text(err)));
      return;
    }

    final ok = await widget.controller.createUser(
      CreateUserRequest(
        username: _usernameCtrl.text.trim(),
        name: _nameCtrl.text.trim(),
        password: _pwdCtrl.text,
        role: _selectedRole,
      ),
    );

    if (ok && mounted) Navigator.of(context).pop();
  }

  @override
  Widget build(BuildContext context) {
    final ctrl = widget.controller;

    return AlertDialog(
      title: const Text('添加员工'),
      contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 0),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            // Login username
            TextField(
              controller: _usernameCtrl,
              decoration: const InputDecoration(
                labelText: '登录用户名',
                hintText: 'zhangsan',
                prefixIcon: Icon(Icons.person_outline),
              ),
              autofocus: true,
            ),
            const SizedBox(height: 12),
            // Display name
            TextField(
              controller: _nameCtrl,
              decoration: const InputDecoration(
                labelText: '姓名（显示名）',
                hintText: '张三',
                prefixIcon: Icon(Icons.badge_outlined),
              ),
            ),
            const SizedBox(height: 12),
            // Password
            TextField(
              controller: _pwdCtrl,
              obscureText: !_showPwd,
              decoration: InputDecoration(
                labelText: '密码（至少6位）',
                prefixIcon: const Icon(Icons.lock_outline),
                suffixIcon: IconButton(
                  icon: Icon(
                      _showPwd ? Icons.visibility_off : Icons.visibility),
                  onPressed: () =>
                      setState(() => _showPwd = !_showPwd),
                ),
              ),
            ),
            const SizedBox(height: 12),
            // Confirm password
            TextField(
              controller: _confirmPwdCtrl,
              obscureText: !_showPwd,
              decoration: const InputDecoration(
                labelText: '确认密码',
                prefixIcon: Icon(Icons.lock_outline),
              ),
            ),
            const SizedBox(height: 16),
            // Role selector
            Text('角色',
                style: Theme.of(context)
                    .textTheme
                    .labelMedium
                    ?.copyWith(fontWeight: FontWeight.w600)),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              children: _kRoles.where((r) {
                // OWNER 不可由普通操作者分配
                int rank(String role) {
                  switch (role.toUpperCase()) {
                    case 'OWNER': return 100;
                    case 'ADMIN': return 50;
                    default: return 10;
                  }
                }
                return rank(widget.currentUserRole) > rank(r.value);
              }).map((r) {
                final selected = _selectedRole == r.value;
                return ChoiceChip(
                  label: Text(r.label),
                  selected: selected,
                  selectedColor: r.color.withValues(alpha: 0.2),
                  onSelected: (_) =>
                      setState(() => _selectedRole = r.value),
                );
              }).toList(),
            ),
            const SizedBox(height: 8),
            // Role description
            Text(
              _roleDesc(_selectedRole),
              style: TextStyle(
                  fontSize: 12,
                  color:
                      Theme.of(context).colorScheme.onSurfaceVariant),
            ),
            const SizedBox(height: 8),
            if (ctrl.errorMessage != null)
              StatusNotice(
                  message: ctrl.errorMessage!, tone: NoticeTone.error),
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: ctrl.submitting ? null : () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: ctrl.submitting ? null : _submit,
          child: ctrl.submitting
              ? const SizedBox(
                  width: 18,
                  height: 18,
                  child: CircularProgressIndicator(strokeWidth: 2))
              : const Text('创建'),
        ),
      ],
    );
  }

  String _roleDesc(String role) {
    switch (role) {
      case 'OWNER':
        return '老板：全部功能，包含人员管理';
      case 'PURCHASER':
        return '采购员：采购入库、库存盘点、商品管理';
      case 'SALES':
        return '销售员：销售出库、销售趋势、订单下钻';
      default:
        return '';
    }
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Update Role Dialog
// ════════════════════════════════════════════════════════════════════════════

class _UpdateRoleDialog extends StatefulWidget {
  const _UpdateRoleDialog(
      {required this.controller, required this.user, required this.currentUserRole});
  final UsersController controller;
  final UserData user;
  /// 操作者角色，用于过滤可分配的角色选项
  final String currentUserRole;

  @override
  State<_UpdateRoleDialog> createState() => _UpdateRoleDialogState();
}

class _UpdateRoleDialogState extends State<_UpdateRoleDialog> {
  late String _selectedRole;

  @override
  void initState() {
    super.initState();
    _selectedRole = widget.user.role;
  }

  Future<void> _submit() async {
    if (_selectedRole == widget.user.role) {
      Navigator.pop(context);
      return;
    }
    final ok = await widget.controller.updateRole(
        widget.user.id, _selectedRole);
    if (ok && mounted) Navigator.pop(context);
  }

  int _roleRank(String role) {
    switch (role.toUpperCase()) {
      case 'OWNER': return 100;
      case 'ADMIN': return 50;
      default: return 10;
    }
  }

  @override
  Widget build(BuildContext context) {
    final ctrl = widget.controller;
    // 过滤：操作者只能分配比自己层级低的角色
    final assignable = _kRoles
        .where((r) => _roleRank(widget.currentUserRole) > _roleRank(r.value))
        .toList();

    return AlertDialog(
      title: Text('修改「${widget.user.name}」角色'),
      contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 0),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          ...assignable.map((r) {
            return Row(
              children: <Widget>[
                Radio<String>(
                  value: r.value,
                  groupValue: _selectedRole,
                  onChanged: (v) => setState(() => _selectedRole = v!),
                  activeColor: r.color,
                ),
                Expanded(
                  child: GestureDetector(
                    onTap: () => setState(() => _selectedRole = r.value),
                    child: Padding(
                      padding: const EdgeInsets.symmetric(vertical: 8),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          _RoleBadge(role: r.value, color: r.color),
                          const SizedBox(height: 4),
                          Text(_roleDesc(r.value),
                              style: const TextStyle(fontSize: 12)),
                        ],
                      ),
                    ),
                  ),
                ),
              ],
            );
          }),
          if (ctrl.errorMessage != null) ...<Widget>[
            const SizedBox(height: 8),
            StatusNotice(
                message: ctrl.errorMessage!, tone: NoticeTone.error),
          ],
        ],
      ),
      actions: <Widget>[
        TextButton(
          onPressed: ctrl.submitting ? null : () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: ctrl.submitting ? null : _submit,
          child: ctrl.submitting
              ? const SizedBox(
                  width: 18,
                  height: 18,
                  child: CircularProgressIndicator(strokeWidth: 2))
              : const Text('确认'),
        ),
      ],
    );
  }

  String _roleDesc(String role) {
    switch (role) {
      case 'OWNER':
        return '老板：全部功能，包含人员管理（唯一）';
      case 'ADMIN':
        return '副管理员：全部业务功能，可管理采购员/销售员';
      case 'PURCHASER':
        return '采购员：采购入库、库存盘点、商品管理';
      case 'SALES':
        return '销售员：销售出库、销售趋势、订单下钻';
      default:
        return '';
    }
  }
}

// ════════════════════════════════════════════════════════════════════════════
// Reset Password Dialog
// ════════════════════════════════════════════════════════════════════════════

class _ResetPasswordDialog extends StatefulWidget {
  const _ResetPasswordDialog(
      {required this.controller, required this.user});
  final UsersController controller;
  final UserData user;

  @override
  State<_ResetPasswordDialog> createState() => _ResetPasswordDialogState();
}

class _ResetPasswordDialogState extends State<_ResetPasswordDialog> {
  final _pwdCtrl = TextEditingController();
  final _confirmCtrl = TextEditingController();
  bool _showPwd = false;

  @override
  void dispose() {
    _pwdCtrl.dispose();
    _confirmCtrl.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (_pwdCtrl.text.length < 6) {
      ScaffoldMessenger.of(context)
          .showSnackBar(const SnackBar(content: Text('密码至少 6 位')));
      return;
    }
    if (_pwdCtrl.text != _confirmCtrl.text) {
      ScaffoldMessenger.of(context)
          .showSnackBar(const SnackBar(content: Text('两次密码不一致')));
      return;
    }
    final ok = await widget.controller
        .resetPassword(widget.user.id, _pwdCtrl.text);
    if (ok && mounted) Navigator.pop(context);
  }

  @override
  Widget build(BuildContext context) {
    final ctrl = widget.controller;
    return AlertDialog(
      title: Text('重置「${widget.user.name}」密码'),
      contentPadding: const EdgeInsets.fromLTRB(24, 16, 24, 0),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          TextField(
            controller: _pwdCtrl,
            obscureText: !_showPwd,
            autofocus: true,
            decoration: InputDecoration(
              labelText: '新密码（至少6位）',
              prefixIcon: const Icon(Icons.lock_outline),
              suffixIcon: IconButton(
                icon: Icon(
                    _showPwd ? Icons.visibility_off : Icons.visibility),
                onPressed: () => setState(() => _showPwd = !_showPwd),
              ),
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _confirmCtrl,
            obscureText: !_showPwd,
            decoration: const InputDecoration(
              labelText: '确认新密码',
              prefixIcon: Icon(Icons.lock_outline),
            ),
          ),
          if (ctrl.errorMessage != null) ...<Widget>[
            const SizedBox(height: 8),
            StatusNotice(
                message: ctrl.errorMessage!, tone: NoticeTone.error),
          ],
          const SizedBox(height: 8),
        ],
      ),
      actions: <Widget>[
        TextButton(
          onPressed: ctrl.submitting ? null : () => Navigator.pop(context),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: ctrl.submitting ? null : _submit,
          child: ctrl.submitting
              ? const SizedBox(
                  width: 18,
                  height: 18,
                  child: CircularProgressIndicator(strokeWidth: 2))
              : const Text('重置密码'),
        ),
      ],
    );
  }
}
