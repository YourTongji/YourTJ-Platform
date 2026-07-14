import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../captcha/presentation/captcha_dialog.dart';

enum _LoginMethod { password, emailCode, registration, passwordReset }

class LoginPage extends ConsumerStatefulWidget {
  const LoginPage({super.key, this.returnLocation});

  final String? returnLocation;

  @override
  ConsumerState<LoginPage> createState() => _LoginPageState();
}

class _LoginPageState extends ConsumerState<LoginPage> {
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _emailController = TextEditingController();
  final TextEditingController _passwordController = TextEditingController();
  final TextEditingController _codeController = TextEditingController();
  final TextEditingController _handleController = TextEditingController();
  final TextEditingController _registrationPasswordController =
      TextEditingController();
  _LoginMethod _method = _LoginMethod.password;
  bool _isSubmitting = false;
  bool _isRequestingCode = false;
  bool _codeRequested = false;
  bool _obscurePassword = true;
  String? _error;
  String? _notice;

  @override
  void dispose() {
    _emailController.dispose();
    _passwordController.dispose();
    _codeController.dispose();
    _handleController.dispose();
    _registrationPasswordController.dispose();
    super.dispose();
  }

  EmailCodePurpose get _codePurpose => _method == _LoginMethod.registration
      ? EmailCodePurpose.registration
      : EmailCodePurpose.login;

  String? _validateEmail(String? value) {
    final String email = value?.trim() ?? '';
    if (!email.endsWith('@tongji.edu.cn') ||
        email.length <= '@tongji.edu.cn'.length) {
      return '请输入有效的同济校园邮箱';
    }
    return null;
  }

  Future<void> _submitPassword() async {
    if (_isSubmitting || !(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    _beginSubmit();
    try {
      await ref
          .read(sessionManagerProvider)
          .passwordLogin(
            email: _emailController.text,
            password: _passwordController.text,
          );
      _finishLogin();
    } on ApiFailure catch (failure) {
      _showFailure(failure.message);
    } finally {
      _endSubmit();
    }
  }

  Future<void> _requestCode() async {
    if (_isRequestingCode || _validateEmail(_emailController.text) != null) {
      _formKey.currentState?.validate();
      return;
    }
    final String? captchaToken = await showCaptchaDialog(
      context: context,
      client: ref.read(captchaClientProvider),
    );
    if (captchaToken == null || !mounted) {
      return;
    }
    setState(() {
      _isRequestingCode = true;
      _error = null;
      _notice = null;
    });
    try {
      await ref
          .read(sessionManagerProvider)
          .requestEmailCode(
            email: _emailController.text,
            captchaToken: captchaToken,
            purpose: _codePurpose,
          );
      if (mounted) {
        setState(() {
          _codeRequested = true;
          _notice = '验证码已发送；若账号符合条件，请检查校园邮箱。';
        });
      }
    } on ApiFailure catch (failure) {
      _showFailure(failure.message);
    } finally {
      if (mounted) {
        setState(() => _isRequestingCode = false);
      }
    }
  }

  Future<void> _submitCode() async {
    if (_isSubmitting ||
        !_codeRequested ||
        !(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    _beginSubmit();
    try {
      await ref
          .read(sessionManagerProvider)
          .verifyEmailCode(
            email: _emailController.text,
            code: _codeController.text,
            purpose: _codePurpose,
            handle: _method == _LoginMethod.registration
                ? _handleController.text
                : null,
            password: _method == _LoginMethod.registration
                ? _registrationPasswordController.text
                : null,
          );
      _finishLogin();
    } on ApiFailure catch (failure) {
      _showFailure(failure.message);
    } finally {
      _endSubmit();
    }
  }

  Future<void> _requestResetCode() async {
    if (_isRequestingCode || _validateEmail(_emailController.text) != null) {
      _formKey.currentState?.validate();
      return;
    }
    final String? captchaToken = await showCaptchaDialog(
      context: context,
      client: ref.read(captchaClientProvider),
    );
    if (captchaToken == null || !mounted) {
      return;
    }
    setState(() {
      _isRequestingCode = true;
      _error = null;
      _notice = null;
    });
    try {
      await ref
          .read(sessionManagerProvider)
          .requestPasswordReset(
            email: _emailController.text,
            captchaToken: captchaToken,
          );
      if (mounted) {
        setState(() {
          _codeRequested = true;
          _notice = '如果该账号可以重置密码，验证码将发送到校园邮箱。';
        });
      }
    } on ApiFailure catch (failure) {
      _showFailure(failure.message);
    } finally {
      if (mounted) {
        setState(() => _isRequestingCode = false);
      }
    }
  }

  Future<void> _submitPasswordReset() async {
    if (_isSubmitting ||
        !_codeRequested ||
        !(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    _beginSubmit();
    try {
      await ref
          .read(sessionManagerProvider)
          .resetPassword(
            email: _emailController.text,
            code: _codeController.text,
            newPassword: _registrationPasswordController.text,
          );
      _finishLogin();
    } on ApiFailure catch (failure) {
      _showFailure(failure.message);
    } finally {
      _endSubmit();
    }
  }

  void _beginSubmit() {
    setState(() {
      _isSubmitting = true;
      _error = null;
      _notice = null;
    });
  }

  void _endSubmit() {
    if (mounted) {
      setState(() => _isSubmitting = false);
    }
  }

  void _showFailure(String message) {
    if (mounted) {
      setState(() => _error = message);
    }
  }

  void _finishLogin() {
    if (!mounted) {
      return;
    }
    final String? returnLocation = widget.returnLocation;
    if (returnLocation != null) {
      context.go(returnLocation);
      return;
    }
    if (context.canPop()) {
      context.pop();
    } else {
      context.go(AppRoutes.account);
    }
  }

  void _selectMethod(Set<_LoginMethod> selection) {
    final _LoginMethod next = selection.single;
    setState(() {
      _method = next;
      _error = null;
      _notice = null;
      _codeRequested = false;
      _codeController.clear();
    });
  }

  void _showPasswordReset() {
    setState(() {
      _method = _LoginMethod.passwordReset;
      _error = null;
      _notice = null;
      _codeRequested = false;
      _codeController.clear();
      _registrationPasswordController.clear();
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('登录 YourTJ')),
      body: SafeArea(
        top: false,
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 520),
              child: Form(
                key: _formKey,
                child: AutofillGroup(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: <Widget>[
                      Semantics(
                        header: true,
                        child: Text(
                          _method == _LoginMethod.passwordReset
                              ? '重置密码'
                              : '欢迎来到 YourTJ',
                          style: Theme.of(context).textTheme.headlineMedium
                              ?.copyWith(fontWeight: FontWeight.w700),
                        ),
                      ),
                      const SizedBox(height: 8),
                      Text(
                        _method == _LoginMethod.passwordReset
                            ? '验证码请求使用中性响应；重置成功会撤销旧会话，并在本机安全保存新的 refresh token。'
                            : '登录、验证码登录与注册使用同一平台身份；成功后会回到原来的浏览位置。',
                      ),
                      const SizedBox(height: 24),
                      if (_method == _LoginMethod.passwordReset)
                        Align(
                          alignment: Alignment.centerLeft,
                          child: TextButton.icon(
                            onPressed: _isSubmitting
                                ? null
                                : () => _selectMethod(<_LoginMethod>{
                                    _LoginMethod.password,
                                  }),
                            icon: const Icon(Icons.arrow_back_rounded),
                            label: const Text('返回登录'),
                          ),
                        )
                      else
                        SegmentedButton<_LoginMethod>(
                          segments: const <ButtonSegment<_LoginMethod>>[
                            ButtonSegment<_LoginMethod>(
                              value: _LoginMethod.password,
                              icon: Icon(Icons.password_rounded),
                              label: Text('密码'),
                            ),
                            ButtonSegment<_LoginMethod>(
                              value: _LoginMethod.emailCode,
                              icon: Icon(Icons.mark_email_read_outlined),
                              label: Text('验证码'),
                            ),
                            ButtonSegment<_LoginMethod>(
                              value: _LoginMethod.registration,
                              icon: Icon(Icons.person_add_alt_1_rounded),
                              label: Text('注册'),
                            ),
                          ],
                          selected: <_LoginMethod>{_method},
                          onSelectionChanged: _isSubmitting
                              ? null
                              : _selectMethod,
                        ),
                      const SizedBox(height: 24),
                      TextFormField(
                        controller: _emailController,
                        autofillHints: const <String>[AutofillHints.email],
                        keyboardType: TextInputType.emailAddress,
                        textInputAction: TextInputAction.next,
                        decoration: const InputDecoration(
                          labelText: '校园邮箱',
                          hintText: 'name@tongji.edu.cn',
                          prefixIcon: Icon(Icons.mail_outline_rounded),
                        ),
                        validator: _validateEmail,
                      ),
                      const SizedBox(height: 16),
                      if (_method == _LoginMethod.password) ...<Widget>[
                        _PasswordField(
                          controller: _passwordController,
                          label: '密码',
                          isObscured: _obscurePassword,
                          onToggleVisibility: () {
                            setState(
                              () => _obscurePassword = !_obscurePassword,
                            );
                          },
                          onSubmitted: _submitPassword,
                        ),
                        Align(
                          alignment: Alignment.centerRight,
                          child: TextButton(
                            onPressed: _isSubmitting
                                ? null
                                : _showPasswordReset,
                            child: const Text('忘记密码？'),
                          ),
                        ),
                      ] else ...<Widget>[
                        Row(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            Expanded(
                              child: TextFormField(
                                controller: _codeController,
                                keyboardType: TextInputType.number,
                                textInputAction:
                                    _method == _LoginMethod.emailCode
                                    ? TextInputAction.done
                                    : TextInputAction.next,
                                decoration: const InputDecoration(
                                  labelText: '邮箱验证码',
                                  prefixIcon: Icon(
                                    Icons.verified_user_outlined,
                                  ),
                                ),
                                validator: (String? value) {
                                  final String code = value?.trim() ?? '';
                                  if (!RegExp(r'^[0-9]{6}$').hasMatch(code)) {
                                    return '请输入 6 位数字验证码';
                                  }
                                  return null;
                                },
                              ),
                            ),
                            const SizedBox(width: 12),
                            Padding(
                              padding: const EdgeInsets.only(top: 4),
                              child: OutlinedButton(
                                onPressed: _isRequestingCode
                                    ? null
                                    : _method == _LoginMethod.passwordReset
                                    ? _requestResetCode
                                    : _requestCode,
                                child: Text(
                                  _isRequestingCode
                                      ? '发送中'
                                      : _codeRequested
                                      ? '重新发送'
                                      : '获取验证码',
                                ),
                              ),
                            ),
                          ],
                        ),
                        if (_method == _LoginMethod.passwordReset) ...<Widget>[
                          const SizedBox(height: 16),
                          _PasswordField(
                            controller: _registrationPasswordController,
                            label: '新密码',
                            isObscured: _obscurePassword,
                            onToggleVisibility: () {
                              setState(
                                () => _obscurePassword = !_obscurePassword,
                              );
                            },
                            onSubmitted: _submitPasswordReset,
                            minimumLength: 8,
                          ),
                        ],
                        if (_method == _LoginMethod.registration) ...<Widget>[
                          const SizedBox(height: 16),
                          TextFormField(
                            controller: _handleController,
                            textInputAction: TextInputAction.next,
                            decoration: const InputDecoration(
                              labelText: '公开用户名',
                              prefixIcon: Icon(Icons.alternate_email_rounded),
                              helperText: '邮箱不会出现在公开资料中。',
                            ),
                            validator: (String? value) {
                              final String handle = value?.trim() ?? '';
                              if (!RegExp(
                                r'^[a-z0-9._-]{3,30}$',
                              ).hasMatch(handle)) {
                                return '使用 3–30 位小写字母、数字、点、下划线或短横线';
                              }
                              return null;
                            },
                          ),
                          const SizedBox(height: 16),
                          _PasswordField(
                            controller: _registrationPasswordController,
                            label: '设置密码',
                            isObscured: _obscurePassword,
                            onToggleVisibility: () {
                              setState(
                                () => _obscurePassword = !_obscurePassword,
                              );
                            },
                            onSubmitted: _submitCode,
                            minimumLength: 8,
                          ),
                        ],
                      ],
                      if (_notice case final String notice) ...<Widget>[
                        const SizedBox(height: 16),
                        Semantics(liveRegion: true, child: Text(notice)),
                      ],
                      if (_error case final String error) ...<Widget>[
                        const SizedBox(height: 16),
                        Semantics(
                          liveRegion: true,
                          child: Text(
                            error,
                            style: TextStyle(
                              color: Theme.of(context).colorScheme.error,
                            ),
                          ),
                        ),
                      ],
                      const SizedBox(height: 24),
                      FilledButton.icon(
                        onPressed: _isSubmitting
                            ? null
                            : _method == _LoginMethod.password
                            ? _submitPassword
                            : _method == _LoginMethod.passwordReset
                            ? _codeRequested
                                  ? _submitPasswordReset
                                  : null
                            : _codeRequested
                            ? _submitCode
                            : null,
                        icon: _isSubmitting
                            ? const SizedBox.square(
                                dimension: 18,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                ),
                              )
                            : const Icon(Icons.login_rounded),
                        label: Text(
                          _isSubmitting
                              ? '正在提交'
                              : _method == _LoginMethod.registration
                              ? '创建账号并登录'
                              : _method == _LoginMethod.passwordReset
                              ? '重置密码并登录'
                              : '登录',
                        ),
                      ),
                      const SizedBox(height: 12),
                      TextButton.icon(
                        onPressed: _isSubmitting
                            ? null
                            : () => context.push(AppRoutes.appeals),
                        icon: const Icon(Icons.gavel_outlined),
                        label: const Text('账号受限？进入申诉中心'),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _PasswordField extends StatelessWidget {
  const _PasswordField({
    required this.controller,
    required this.label,
    required this.isObscured,
    required this.onToggleVisibility,
    required this.onSubmitted,
    this.minimumLength = 1,
  });

  final TextEditingController controller;
  final String label;
  final bool isObscured;
  final VoidCallback onToggleVisibility;
  final VoidCallback onSubmitted;
  final int minimumLength;

  @override
  Widget build(BuildContext context) {
    return TextFormField(
      controller: controller,
      autofillHints: const <String>[AutofillHints.password],
      obscureText: isObscured,
      textInputAction: TextInputAction.done,
      onFieldSubmitted: (_) => onSubmitted(),
      decoration: InputDecoration(
        labelText: label,
        prefixIcon: const Icon(Icons.lock_outline_rounded),
        suffixIcon: IconButton(
          tooltip: isObscured ? '显示密码' : '隐藏密码',
          onPressed: onToggleVisibility,
          icon: Icon(
            isObscured
                ? Icons.visibility_outlined
                : Icons.visibility_off_outlined,
          ),
        ),
      ),
      validator: (String? value) {
        if (value == null || value.length < minimumLength) {
          return minimumLength > 1 ? '密码至少需要 $minimumLength 位' : '请输入密码';
        }
        return null;
      },
    );
  }
}
