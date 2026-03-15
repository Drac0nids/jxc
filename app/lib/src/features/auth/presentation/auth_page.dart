import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../core/widgets/brand_ui.dart';
import '../../../storage/session_storage.dart';
import '../application/session_controller.dart';

class AuthPage extends StatefulWidget {
  const AuthPage({
    super.key,
    required this.sessionController,
    required this.sessionStorage,
  });

  final SessionController sessionController;
  final SessionStorage sessionStorage;

  @override
  State<AuthPage> createState() => _AuthPageState();
}

class _AuthPageState extends State<AuthPage> {
  bool _isRegisterMode = false;
  bool _obscurePassword = true;

  final TextEditingController _tenantCodeController =
      TextEditingController(text: 'DEMO01');
  final TextEditingController _tenantNameController = TextEditingController();
  final TextEditingController _usernameController =
      TextEditingController(text: 'admin');
  final TextEditingController _nameController = TextEditingController();
  final TextEditingController _passwordController =
      TextEditingController(text: 'admin123');

  @override
  void initState() {
    super.initState();
    // 尝试从本地读取上次登录的租户码，自动填写
    widget.sessionStorage.readTenantCode().then((code) {
      if (code != null && mounted) {
        _tenantCodeController.text = code;
      }
    });
  }

  @override
  void dispose() {
    _tenantCodeController.dispose();
    _tenantNameController.dispose();
    _usernameController.dispose();
    _nameController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return AnimatedBuilder(
      animation: widget.sessionController,
      builder: (BuildContext context, Widget? child) {
        return Scaffold(
          body: SafeArea(
            child: Center(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 420),
                child: SingleChildScrollView(
                  padding: const EdgeInsets.all(20),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: <Widget>[
                      AnimatedSwitcher(
                        duration: const Duration(milliseconds: 220),
                        child: BrandHeroBanner(
                          key: ValueKey<bool>(_isRegisterMode),
                          title: _isRegisterMode ? '创建你的进销存空间' : '欢迎使用极速云进销存',
                          subtitle: _isRegisterMode
                              ? '30 秒完成租户初始化并创建管理员账号'
                              : '登录后继续今日经营与作业流程',
                          icon: _isRegisterMode
                              ? Icons.app_registration_rounded
                              : Icons.inventory_2_rounded,
                          gradientSeedColor: _isRegisterMode
                              ? const Color(0xFF8B5CF6)
                              : const Color(0xFF3B82F6),
                        ),
                      ),
                      const SizedBox(height: 16),
                      SectionCard(
                        title: _isRegisterMode ? '租户注册' : '账号登录',
                        subtitle: _isRegisterMode
                            ? '先创建租户，再自动登录到系统'
                            : '输入账号密码后进入经营看板',
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: <Widget>[
                            SegmentedButton<bool>(
                              segments: const <ButtonSegment<bool>>[
                                ButtonSegment<bool>(
                                    value: false, label: Text('登录')),
                                ButtonSegment<bool>(
                                    value: true, label: Text('注册')),
                              ],
                              selected: <bool>{_isRegisterMode},
                              onSelectionChanged:
                                  widget.sessionController.submitting
                                      ? null
                                      : (Set<bool> value) {
                                          setState(() {
                                            _isRegisterMode = value.first;
                                          });
                                          widget.sessionController.clearError();
                                        },
                            ),
                            const SizedBox(height: 16),
                            if (_isRegisterMode) ...<Widget>[
                              TextField(
                                controller: _tenantNameController,
                                decoration: const InputDecoration(
                                  labelText: '租户名称（可选）',
                                ),
                              ),
                              const SizedBox(height: 12),
                            ],
                            // 租户码：登录模式农显示，注册模式隐藏
                            if (!_isRegisterMode) ...<Widget>[
                              TextField(
                                controller: _tenantCodeController,
                                textCapitalization: TextCapitalization.characters,
                                inputFormatters: <TextInputFormatter>[
                                  FilteringTextInputFormatter.allow(
                                      RegExp('[A-Za-z0-9]')),
                                  LengthLimitingTextInputFormatter(8),
                                ],
                                decoration: InputDecoration(
                                  labelText: '租户码 *',
                                  hintText: '登录后自动记住，下次无需重填',
                                  prefixIcon: const Icon(Icons.domain_rounded),
                                  helperText: '注册后系统自动生成，可在「人员管理」页查看',
                                ),
                                onChanged: (_) => setState(() {}),
                              ),
                              const SizedBox(height: 12),
                            ],
                            TextField(
                              controller: _usernameController,
                              decoration: InputDecoration(
                                labelText:
                                    _isRegisterMode ? 'Owner 登录用户名' : '用户名',
                              ),
                            ),
                            if (_isRegisterMode) ...<Widget>[
                              const SizedBox(height: 12),
                              TextField(
                                controller: _nameController,
                                decoration: const InputDecoration(
                                  labelText: 'Owner 姓名',
                                ),
                              ),
                            ],
                            const SizedBox(height: 12),
                            TextField(
                              controller: _passwordController,
                              obscureText: _obscurePassword,
                              decoration: InputDecoration(
                                labelText: '密码',
                                suffixIcon: IconButton(
                                  tooltip: _obscurePassword ? '显示密码' : '隐藏密码',
                                  onPressed: () {
                                    setState(() {
                                      _obscurePassword = !_obscurePassword;
                                    });
                                  },
                                  icon: Icon(
                                    _obscurePassword
                                        ? Icons.visibility_outlined
                                        : Icons.visibility_off_outlined,
                                  ),
                                ),
                              ),
                            ),
                            const SizedBox(height: 16),
                            FilledButton(
                              onPressed: widget.sessionController.submitting
                                  ? null
                                  : _submit,
                              child: Text(
                                widget.sessionController.submitting
                                    ? (_isRegisterMode ? '注册中...' : '登录中...')
                                    : (_isRegisterMode ? '注册并登录' : '登录'),
                              ),
                            ),
                            if (widget.sessionController.errorMessage !=
                                null) ...<Widget>[
                              const SizedBox(height: 12),
                              StatusNotice(
                                message: widget.sessionController.errorMessage!,
                                tone: NoticeTone.error,
                              ),
                            ],
                            if (!_isRegisterMode) ...<Widget>[
                              const SizedBox(height: 10),
                              Text(
                                '演示账号：租户码 DEMO01 / admin / admin123',
                                style: theme.textTheme.bodySmall?.copyWith(
                                  color: colorScheme.onSurfaceVariant,
                                ),
                              ),
                            ],
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  Future<void> _submit() async {
    FocusScope.of(context).unfocus();
    widget.sessionController.clearError();

    if (_isRegisterMode) {
      await _submitRegister();
      return;
    }

    await _submitLogin();
  }

  Future<void> _submitLogin() async {
    final username = _usernameController.text.trim();
    final password = _passwordController.text;
    final tenantCode = _tenantCodeController.text.trim();

    if (tenantCode.isEmpty) {
      _showMessage('请输入租户码');
      return;
    }

    if (username.isEmpty || password.isEmpty) {
      _showMessage('请输入用户名和密码');
      return;
    }

    await widget.sessionController.login(
      username: username,
      password: password,
      tenantCode: tenantCode.toUpperCase(),
    );
  }

  Future<void> _submitRegister() async {
    final tenantName = _tenantNameController.text.trim();
    final username = _usernameController.text.trim();
    final name = _nameController.text.trim();
    final password = _passwordController.text;

    if (username.isEmpty || name.isEmpty || password.isEmpty) {
      _showMessage('请输入用户名、姓名和密码');
      return;
    }

    await widget.sessionController.register(
      tenantName: tenantName.isEmpty ? null : tenantName,
      username: username,
      name: name,
      password: password,
    );
  }

  void _showMessage(String text) {
    if (!mounted) {
      return;
    }

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(text)),
    );
  }
}
