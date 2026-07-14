import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../account/presentation/account_page_layout.dart';
import '../../auth/domain/session_state.dart';
import 'recent_auth_dialog.dart';

class PasswordSettingsPage extends ConsumerStatefulWidget {
  const PasswordSettingsPage({super.key});

  @override
  ConsumerState<PasswordSettingsPage> createState() =>
      _PasswordSettingsPageState();
}

class _PasswordSettingsPageState extends ConsumerState<PasswordSettingsPage> {
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _currentPasswordController =
      TextEditingController();
  final TextEditingController _newPasswordController = TextEditingController();
  final TextEditingController _confirmPasswordController =
      TextEditingController();
  bool _isSubmitting = false;
  bool _obscureCurrent = true;
  bool _obscureNew = true;
  bool _hasPassword = false;
  ApiFailure? _failure;

  @override
  void initState() {
    super.initState();
    _hasPassword =
        ref.read(sessionManagerProvider).state.account?.hasPassword ?? false;
  }

  @override
  void dispose() {
    _currentPasswordController.dispose();
    _newPasswordController.dispose();
    _confirmPasswordController.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    if (_isSubmitting || !(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    if (!_hasPassword) {
      final bool verified = await ensureRecentAuthentication(context, ref);
      if (!verified || !mounted) {
        return;
      }
    }
    setState(() {
      _isSubmitting = true;
      _failure = null;
    });
    try {
      if (_hasPassword) {
        await ref
            .read(sessionManagerProvider)
            .changePassword(
              currentPassword: _currentPasswordController.text,
              newPassword: _newPasswordController.text,
            );
      } else {
        await ref
            .read(sessionManagerProvider)
            .setPassword(newPassword: _newPasswordController.text);
      }
      if (!mounted) {
        return;
      }
      setState(() => _hasPassword = true);
      _currentPasswordController.clear();
      _newPasswordController.clear();
      _confirmPasswordController.clear();
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('密码已更新，其他旧会话已由服务器撤销')));
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isSubmitting = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final SessionState session =
        ref.watch(sessionStateProvider).value ??
        ref.read(sessionManagerProvider).state;
    if (!session.isAuthenticated) {
      return const AccountPageLayout(
        title: '密码设置',
        child: AppPermissionState(
          title: '需要登录',
          description: '设置或修改密码必须绑定到当前可撤销会话。',
        ),
      );
    }
    return AccountPageLayout(
      title: _hasPassword ? '修改密码' : '设置密码',
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Form(
          key: _formKey,
          child: AutofillGroup(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Text(
                  _hasPassword
                      ? '修改成功后，服务器会撤销全部旧 refresh family，并为本机安全替换会话。'
                      : '首次设定需要当前 server-bound session 的最近认证；已有密码时不会被覆盖。',
                ),
                const SizedBox(height: 24),
                if (_hasPassword) ...<Widget>[
                  _PasswordField(
                    controller: _currentPasswordController,
                    label: '当前密码',
                    isObscured: _obscureCurrent,
                    onToggle: () =>
                        setState(() => _obscureCurrent = !_obscureCurrent),
                    autofillHint: AutofillHints.password,
                    validator: (String? value) =>
                        value == null || value.isEmpty ? '请输入当前密码' : null,
                  ),
                  const SizedBox(height: 16),
                ],
                _PasswordField(
                  controller: _newPasswordController,
                  label: '新密码',
                  isObscured: _obscureNew,
                  onToggle: () => setState(() => _obscureNew = !_obscureNew),
                  autofillHint: AutofillHints.newPassword,
                  validator: _validateNewPassword,
                ),
                const SizedBox(height: 16),
                TextFormField(
                  controller: _confirmPasswordController,
                  enabled: !_isSubmitting,
                  obscureText: _obscureNew,
                  autofillHints: const <String>[AutofillHints.newPassword],
                  decoration: const InputDecoration(labelText: '再次输入新密码'),
                  validator: (String? value) =>
                      value != _newPasswordController.text
                      ? '两次输入的新密码不一致'
                      : null,
                ),
                if (_failure != null) ...<Widget>[
                  const SizedBox(height: 16),
                  Semantics(
                    liveRegion: true,
                    child: Text(
                      _failure!.message,
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.error,
                      ),
                    ),
                  ),
                ],
                const SizedBox(height: 24),
                FilledButton.icon(
                  onPressed: _isSubmitting ? null : _submit,
                  icon: _isSubmitting
                      ? const SizedBox.square(
                          dimension: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.password_rounded),
                  label: Text(
                    _isSubmitting
                        ? '正在替换会话'
                        : _hasPassword
                        ? '修改密码'
                        : '设置密码',
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  String? _validateNewPassword(String? value) {
    final String password = value ?? '';
    if (password.length < 8 || password.length > 128) {
      return '新密码需为 8–128 个字符';
    }
    return null;
  }
}

class _PasswordField extends StatelessWidget {
  const _PasswordField({
    required this.controller,
    required this.label,
    required this.isObscured,
    required this.onToggle,
    required this.autofillHint,
    required this.validator,
  });

  final TextEditingController controller;
  final String label;
  final bool isObscured;
  final VoidCallback onToggle;
  final String autofillHint;
  final FormFieldValidator<String> validator;

  @override
  Widget build(BuildContext context) {
    return TextFormField(
      controller: controller,
      obscureText: isObscured,
      autofillHints: <String>[autofillHint],
      decoration: InputDecoration(
        labelText: label,
        suffixIcon: IconButton(
          tooltip: isObscured ? '显示密码' : '隐藏密码',
          onPressed: onToggle,
          icon: Icon(
            isObscured
                ? Icons.visibility_outlined
                : Icons.visibility_off_outlined,
          ),
        ),
      ),
      validator: validator,
    );
  }
}
