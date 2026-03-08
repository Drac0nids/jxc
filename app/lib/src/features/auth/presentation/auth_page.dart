import 'package:flutter/material.dart';

import '../application/session_controller.dart';

class AuthPage extends StatefulWidget {
  const AuthPage({super.key, required this.sessionController});

  final SessionController sessionController;

  @override
  State<AuthPage> createState() => _AuthPageState();
}

class _AuthPageState extends State<AuthPage> {
  bool _isRegisterMode = false;

  final TextEditingController _tenantNameController = TextEditingController();
  final TextEditingController _usernameController = TextEditingController(text: 'admin');
  final TextEditingController _nameController = TextEditingController();
  final TextEditingController _passwordController = TextEditingController(text: 'admin123');

  @override
  void dispose() {
    _tenantNameController.dispose();
    _usernameController.dispose();
    _nameController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: widget.sessionController,
      builder: (BuildContext context, Widget? child) {
        return Scaffold(
          appBar: AppBar(title: Text(_isRegisterMode ? '租户注册' : '登录系统')),
          body: SafeArea(
            child: Center(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 420),
                child: SingleChildScrollView(
                  padding: const EdgeInsets.all(20),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: <Widget>[
                      SegmentedButton<bool>(
                        segments: const <ButtonSegment<bool>>[
                          ButtonSegment<bool>(value: false, label: Text('登录')),
                          ButtonSegment<bool>(value: true, label: Text('注册')),
                        ],
                        selected: <bool>{_isRegisterMode},
                        onSelectionChanged: widget.sessionController.submitting
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
                            border: OutlineInputBorder(),
                          ),
                        ),
                        const SizedBox(height: 12),
                      ],
                      TextField(
                        controller: _usernameController,
                        decoration: InputDecoration(
                          labelText: _isRegisterMode ? 'Owner 登录用户名' : '用户名',
                          border: const OutlineInputBorder(),
                        ),
                      ),
                      if (_isRegisterMode) ...<Widget>[
                        const SizedBox(height: 12),
                        TextField(
                          controller: _nameController,
                          decoration: const InputDecoration(
                            labelText: 'Owner 姓名',
                            border: OutlineInputBorder(),
                          ),
                        ),
                      ],
                      const SizedBox(height: 12),
                      TextField(
                        controller: _passwordController,
                        obscureText: true,
                        decoration: const InputDecoration(
                          labelText: '密码',
                          border: OutlineInputBorder(),
                        ),
                      ),
                      const SizedBox(height: 16),
                      FilledButton(
                        onPressed: widget.sessionController.submitting ? null : _submit,
                        child: Text(
                          widget.sessionController.submitting
                              ? (_isRegisterMode ? '注册中...' : '登录中...')
                              : (_isRegisterMode ? '注册并登录' : '登录'),
                        ),
                      ),
                      if (widget.sessionController.errorMessage != null) ...<Widget>[
                        const SizedBox(height: 12),
                        Text(
                          widget.sessionController.errorMessage!,
                          style: TextStyle(color: Theme.of(context).colorScheme.error),
                        ),
                      ],
                      if (!_isRegisterMode) ...<Widget>[
                        const SizedBox(height: 8),
                        const Text(
                          '演示账号：admin / admin123',
                          style: TextStyle(fontSize: 12, color: Colors.black54),
                        ),
                      ],
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

    if (username.isEmpty || password.isEmpty) {
      _showMessage('请输入用户名和密码');
      return;
    }

    await widget.sessionController.login(
      username: username,
      password: password,
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
