import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/validation/identity_inputs.dart';
import '../../account/data/account_repository.dart';
import '../../account/presentation/account_page_layout.dart';
import '../../captcha/presentation/captcha_dialog.dart';

enum _RecoveryMethod { password, emailCode }

class RecoveryPage extends ConsumerStatefulWidget {
  const RecoveryPage({super.key});

  @override
  ConsumerState<RecoveryPage> createState() => _RecoveryPageState();
}

class _RecoveryPageState extends ConsumerState<RecoveryPage> {
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _emailController = TextEditingController();
  final TextEditingController _credentialController = TextEditingController();
  _RecoveryMethod _method = _RecoveryMethod.password;
  RecoveryCredential? _credential;
  ApiFailure? _failure;
  bool _isSubmitting = false;
  bool _isRequestingCode = false;
  bool _codeRequested = false;
  bool _obscurePassword = true;

  @override
  void dispose() {
    _emailController.dispose();
    _credentialController.dispose();
    super.dispose();
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
      _failure = null;
    });
    try {
      await ref
          .read(sessionManagerProvider)
          .requestEmailCode(
            email: _emailController.text,
            captchaToken: captchaToken,
            purpose: EmailCodePurpose.recovery,
          );
      if (mounted) {
        setState(() => _codeRequested = true);
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isRequestingCode = false);
      }
    }
  }

  Future<void> _prove() async {
    if (_isSubmitting || !(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    if (_method == _RecoveryMethod.emailCode && !_codeRequested) {
      setState(() {
        _failure = const ApiFailure(
          kind: ApiFailureKind.invalidInput,
          message: '请先完成验证码发送流程',
        );
      });
      return;
    }
    setState(() {
      _isSubmitting = true;
      _failure = null;
      _credential = null;
    });
    try {
      final AccountRepository repository = ref.read(accountRepositoryProvider);
      final RecoveryCredential credential = _method == _RecoveryMethod.password
          ? await repository.proveRecoveryWithPassword(
              email: _emailController.text,
              password: _credentialController.text,
            )
          : await repository.proveRecoveryWithEmailCode(
              email: _emailController.text,
              code: _credentialController.text,
            );
      if (!mounted) {
        return;
      }
      setState(() => _credential = credential);
      _credentialController.clear();
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

  Future<void> _recover() async {
    final RecoveryCredential? credential = _credential;
    if (credential == null || _isSubmitting) {
      return;
    }
    setState(() {
      _isSubmitting = true;
      _failure = null;
    });
    try {
      final AccountLifecycle lifecycle = await ref
          .read(accountRepositoryProvider)
          .recoverAccount(credential.recoveryToken);
      if (lifecycle.state != AccountLifecycleState.active) {
        throw const ApiFailure(
          kind: ApiFailureKind.conflict,
          message: '服务器未确认账号已恢复为 active',
        );
      }
      if (!mounted) {
        return;
      }
      setState(() => _credential = null);
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('账号已恢复，旧会话仍失效，请正常登录')));
      context.go(AppRoutes.login);
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
    return AccountPageLayout(
      title: '恢复已停用或待删除账号',
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            const Card(
              child: Padding(
                padding: EdgeInsets.all(16),
                child: Text(
                  '恢复证明只生成短期 recovery credential，不会创建普通 access/refresh session。恢复成功后必须重新登录；已开始 purge 或超过恢复窗口的账号会 fail closed。',
                ),
              ),
            ),
            const SizedBox(height: 16),
            if (_credential == null)
              Form(
                key: _formKey,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    SegmentedButton<_RecoveryMethod>(
                      segments: const <ButtonSegment<_RecoveryMethod>>[
                        ButtonSegment<_RecoveryMethod>(
                          value: _RecoveryMethod.password,
                          icon: Icon(Icons.password_rounded),
                          label: Text('密码'),
                        ),
                        ButtonSegment<_RecoveryMethod>(
                          value: _RecoveryMethod.emailCode,
                          icon: Icon(Icons.mark_email_read_outlined),
                          label: Text('邮箱验证码'),
                        ),
                      ],
                      selected: <_RecoveryMethod>{_method},
                      onSelectionChanged: _isSubmitting
                          ? null
                          : (Set<_RecoveryMethod> selected) {
                              setState(() {
                                _method = selected.single;
                                _credentialController.clear();
                                _failure = null;
                                _codeRequested = false;
                              });
                            },
                    ),
                    const SizedBox(height: 16),
                    TextFormField(
                      controller: _emailController,
                      enabled: !_isSubmitting,
                      keyboardType: TextInputType.emailAddress,
                      autofillHints: const <String>[AutofillHints.email],
                      decoration: const InputDecoration(
                        labelText: '校园邮箱',
                        hintText: 'name@tongji.edu.cn',
                      ),
                      validator: _validateEmail,
                    ),
                    const SizedBox(height: 16),
                    TextFormField(
                      controller: _credentialController,
                      enabled: !_isSubmitting,
                      obscureText:
                          _method == _RecoveryMethod.password &&
                          _obscurePassword,
                      keyboardType: _method == _RecoveryMethod.emailCode
                          ? TextInputType.number
                          : TextInputType.visiblePassword,
                      decoration: InputDecoration(
                        labelText: _method == _RecoveryMethod.password
                            ? '密码'
                            : '恢复验证码',
                        suffixIcon: _method == _RecoveryMethod.password
                            ? IconButton(
                                tooltip: _obscurePassword ? '显示密码' : '隐藏密码',
                                onPressed: () => setState(
                                  () => _obscurePassword = !_obscurePassword,
                                ),
                                icon: Icon(
                                  _obscurePassword
                                      ? Icons.visibility_outlined
                                      : Icons.visibility_off_outlined,
                                ),
                              )
                            : null,
                      ),
                      validator: (String? value) {
                        final String credential = value ?? '';
                        if (_method == _RecoveryMethod.password &&
                            credential.isEmpty) {
                          return '请输入密码';
                        }
                        if (_method == _RecoveryMethod.emailCode &&
                            !IdentityInputs.isValidEmailVerificationCode(
                              credential.trim(),
                            )) {
                          return '请输入 6 位数字验证码';
                        }
                        return null;
                      },
                    ),
                    if (_method == _RecoveryMethod.emailCode) ...<Widget>[
                      const SizedBox(height: 8),
                      OutlinedButton(
                        onPressed: _isRequestingCode ? null : _requestCode,
                        child: Text(
                          _isRequestingCode
                              ? '正在发送'
                              : _codeRequested
                              ? '重新发送恢复验证码'
                              : '发送恢复验证码',
                        ),
                      ),
                    ],
                    const SizedBox(height: 20),
                    FilledButton(
                      onPressed: _isSubmitting ? null : _prove,
                      child: Text(_isSubmitting ? '正在验证' : '检查是否可恢复'),
                    ),
                  ],
                ),
              )
            else
              _buildRecoveryConfirmation(_credential!),
            if (_failure != null) ...<Widget>[
              const SizedBox(height: 16),
              Semantics(
                liveRegion: true,
                child: Text(
                  _failure!.message,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildRecoveryConfirmation(RecoveryCredential credential) {
    final AccountLifecycle lifecycle = credential.lifecycle;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text('可恢复账号', style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 8),
            Text('当前状态：${_recoveryStateLabel(lifecycle.state)}'),
            if (lifecycle.recoverUntil != null)
              Text('恢复窗口截止：${formatAccountTime(lifecycle.recoverUntil!)}'),
            Text('本次恢复凭据到期：${formatAccountTime(credential.expiresAt)}'),
            const SizedBox(height: 12),
            const Text('确认后会原子消费该凭据，恢复 active，但不会自动登录。'),
            const SizedBox(height: 16),
            FilledButton.icon(
              onPressed: _isSubmitting ? null : _recover,
              icon: const Icon(Icons.restore_rounded),
              label: Text(_isSubmitting ? '正在恢复' : '确认恢复账号'),
            ),
            TextButton(
              onPressed: _isSubmitting
                  ? null
                  : () => setState(() {
                      _credential = null;
                      _failure = null;
                    }),
              child: const Text('返回重新验证'),
            ),
          ],
        ),
      ),
    );
  }
}

String? _validateEmail(String? value) {
  final String email = value?.trim() ?? '';
  if (!email.endsWith('@tongji.edu.cn') ||
      email.length <= '@tongji.edu.cn'.length) {
    return '请输入有效的同济校园邮箱';
  }
  return null;
}

String _recoveryStateLabel(AccountLifecycleState state) {
  return switch (state) {
    AccountLifecycleState.deactivated => '已停用',
    AccountLifecycleState.deletionRequested => '已请求删除',
    AccountLifecycleState.deleted => '已删除（恢复窗口内）',
    AccountLifecycleState.active => '已活跃',
    AccountLifecycleState.purged => '已不可逆清理',
    AccountLifecycleState.unknownDefaultOpenApi => '未知状态',
  };
}
