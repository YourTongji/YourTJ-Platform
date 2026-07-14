import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../../../core/validation/identity_inputs.dart';
import '../../account/data/account_repository.dart';

Future<bool> ensureRecentAuthentication(
  BuildContext context,
  WidgetRef ref,
) async {
  RecentAuthStatus? status;
  ApiFailure? failure;
  try {
    status = await ref.read(accountRepositoryProvider).getRecentAuthStatus();
    if (status.isFresh) {
      return true;
    }
  } on ApiFailure catch (error) {
    failure = error;
  }
  if (!context.mounted) {
    return false;
  }
  return await showDialog<bool>(
        context: context,
        barrierDismissible: false,
        builder: (BuildContext context) =>
            _RecentAuthDialog(initialStatus: status, initialFailure: failure),
      ) ??
      false;
}

class _RecentAuthDialog extends ConsumerStatefulWidget {
  const _RecentAuthDialog({this.initialStatus, this.initialFailure});

  final RecentAuthStatus? initialStatus;
  final ApiFailure? initialFailure;

  @override
  ConsumerState<_RecentAuthDialog> createState() => _RecentAuthDialogState();
}

class _RecentAuthDialogState extends ConsumerState<_RecentAuthDialog> {
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _credentialController = TextEditingController();
  late RecentAuthStatus? _status = widget.initialStatus;
  late ApiFailure? _failure = widget.initialFailure;
  RecentAuthMethod? _method;
  bool _isSubmitting = false;
  bool _isRequestingCode = false;
  bool _emailCodeRequested = false;
  bool _obscurePassword = true;

  @override
  void initState() {
    super.initState();
    _selectInitialMethod();
  }

  @override
  void dispose() {
    _credentialController.dispose();
    super.dispose();
  }

  void _selectInitialMethod() {
    final List<RecentAuthMethod> methods =
        _status?.availableMethods ?? <RecentAuthMethod>[];
    if (methods.contains(RecentAuthMethod.password)) {
      _method = RecentAuthMethod.password;
    } else if (methods.contains(RecentAuthMethod.emailCode)) {
      _method = RecentAuthMethod.emailCode;
    }
  }

  Future<void> _reload() async {
    setState(() {
      _isSubmitting = true;
      _failure = null;
    });
    try {
      final RecentAuthStatus status = await ref
          .read(accountRepositoryProvider)
          .getRecentAuthStatus();
      if (!mounted) {
        return;
      }
      if (status.isFresh) {
        Navigator.pop(context, true);
        return;
      }
      setState(() => _status = status);
      _selectInitialMethod();
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

  Future<void> _requestEmailCode() async {
    if (_isRequestingCode) {
      return;
    }
    setState(() {
      _isRequestingCode = true;
      _failure = null;
    });
    try {
      await ref.read(accountRepositoryProvider).requestRecentAuthEmailCode();
      if (mounted) {
        setState(() => _emailCodeRequested = true);
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

  Future<void> _verify() async {
    final RecentAuthMethod? method = _method;
    if (method == null ||
        _isSubmitting ||
        !(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    setState(() {
      _isSubmitting = true;
      _failure = null;
    });
    try {
      final RecentAuthStatus status = await ref
          .read(accountRepositoryProvider)
          .verifyRecentAuth(
            RecentAuthVerifyInput(
              method: method,
              password: method == RecentAuthMethod.password
                  ? _credentialController.text
                  : null,
              code: method == RecentAuthMethod.emailCode
                  ? _credentialController.text.trim()
                  : null,
            ),
          );
      if (!status.isFresh) {
        throw const ApiFailure(
          kind: ApiFailureKind.forbidden,
          message: '服务器未确认当前会话已完成最近认证',
        );
      }
      if (mounted) {
        Navigator.pop(context, true);
      }
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
    final RecentAuthStatus? status = _status;
    return AlertDialog(
      title: const Text('再次确认是你本人'),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 480),
        child: SingleChildScrollView(
          child: Form(
            key: _formKey,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                const Text('高风险操作只认服务器绑定到当前可撤销会话的 10 分钟最近认证。'),
                if (status != null && !status.sessionBound) ...<Widget>[
                  const SizedBox(height: 12),
                  const Text('当前是无法绑定到服务器 session 的旧凭据，请退出后重新登录。'),
                ] else if (status != null && _method != null) ...<Widget>[
                  const SizedBox(height: 16),
                  SegmentedButton<RecentAuthMethod>(
                    segments: <ButtonSegment<RecentAuthMethod>>[
                      if (status.availableMethods.contains(
                        RecentAuthMethod.password,
                      ))
                        const ButtonSegment<RecentAuthMethod>(
                          value: RecentAuthMethod.password,
                          icon: Icon(Icons.password_rounded),
                          label: Text('当前密码'),
                        ),
                      if (status.availableMethods.contains(
                        RecentAuthMethod.emailCode,
                      ))
                        const ButtonSegment<RecentAuthMethod>(
                          value: RecentAuthMethod.emailCode,
                          icon: Icon(Icons.mark_email_read_outlined),
                          label: Text('邮箱验证码'),
                        ),
                    ],
                    selected: <RecentAuthMethod>{_method!},
                    onSelectionChanged: _isSubmitting
                        ? null
                        : (Set<RecentAuthMethod> selected) {
                            setState(() {
                              _method = selected.single;
                              _credentialController.clear();
                              _failure = null;
                            });
                          },
                  ),
                  const SizedBox(height: 16),
                  TextFormField(
                    controller: _credentialController,
                    obscureText:
                        _method == RecentAuthMethod.password &&
                        _obscurePassword,
                    keyboardType: _method == RecentAuthMethod.emailCode
                        ? TextInputType.number
                        : TextInputType.visiblePassword,
                    autofillHints: _method == RecentAuthMethod.password
                        ? const <String>[AutofillHints.password]
                        : null,
                    decoration: InputDecoration(
                      labelText: _method == RecentAuthMethod.password
                          ? '当前密码'
                          : '邮箱验证码',
                      suffixIcon: _method == RecentAuthMethod.password
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
                      if (_method == RecentAuthMethod.password &&
                          credential.isEmpty) {
                        return '请输入当前密码';
                      }
                      if (_method == RecentAuthMethod.emailCode &&
                          !IdentityInputs.isValidEmailVerificationCode(
                            credential.trim(),
                          )) {
                        return '请输入 6 位数字验证码';
                      }
                      return null;
                    },
                  ),
                  if (_method == RecentAuthMethod.emailCode) ...<Widget>[
                    const SizedBox(height: 8),
                    OutlinedButton(
                      onPressed: _isRequestingCode ? null : _requestEmailCode,
                      child: Text(
                        _isRequestingCode
                            ? '正在发送'
                            : _emailCodeRequested
                            ? '重新发送验证码'
                            : '发送验证码到绑定邮箱',
                      ),
                    ),
                  ],
                ] else if (status != null) ...<Widget>[
                  const SizedBox(height: 12),
                  const Text('当前账号没有可用的最近认证方式，请重新登录或联系支持。'),
                ],
                if (_failure != null) ...<Widget>[
                  const SizedBox(height: 12),
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
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _isSubmitting ? null : () => Navigator.pop(context, false),
          child: const Text('取消'),
        ),
        if (status == null)
          FilledButton(
            onPressed: _isSubmitting ? null : _reload,
            child: const Text('重试'),
          )
        else if (status.sessionBound && _method != null)
          FilledButton(
            onPressed: _isSubmitting ? null : _verify,
            child: Text(_isSubmitting ? '正在验证' : '确认'),
          ),
      ],
    );
  }
}
