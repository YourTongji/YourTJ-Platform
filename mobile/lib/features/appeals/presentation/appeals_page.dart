import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:uuid/uuid.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../auth/domain/session_state.dart';
import '../../captcha/presentation/captcha_dialog.dart';
import '../data/appeals_repository.dart';

enum _AppealAccessMethod { password, emailCode }

class AppealsPage extends ConsumerStatefulWidget {
  const AppealsPage({this.initialEventId, this.initialAppealId, super.key});

  final String? initialEventId;
  final String? initialAppealId;

  @override
  ConsumerState<AppealsPage> createState() => _AppealsPageState();
}

class _AppealsPageState extends ConsumerState<AppealsPage> {
  final GlobalKey<FormState> _accessFormKey = GlobalKey<FormState>();
  final GlobalKey<FormState> _submitFormKey = GlobalKey<FormState>();
  final TextEditingController _emailController = TextEditingController();
  final TextEditingController _credentialController = TextEditingController();
  late final TextEditingController _eventController = TextEditingController(
    text: widget.initialEventId ?? '',
  );
  final TextEditingController _reasonController = TextEditingController();
  _AppealAccessMethod _accessMethod = _AppealAccessMethod.password;
  AppealAccessToken? _restrictedAccess;
  List<Appeal> _appeals = <Appeal>[];
  List<GovernanceNotice> _notices = <GovernanceNotice>[];
  String? _nextAppealCursor;
  bool _appealHasMore = false;
  bool _isLoading = false;
  bool _isLoadingMore = false;
  bool _isSubmitting = false;
  bool _isRequestingCode = false;
  bool _codeRequested = false;
  bool _obscurePassword = true;
  String? _pendingSubmitFingerprint;
  String? _pendingSubmitKey;
  int _requestGeneration = 0;
  int? _sessionGeneration;
  ApiFailure? _failure;

  AppealsRepository get _repository => ref.read(appealsRepositoryProvider);

  @override
  void dispose() {
    _emailController.dispose();
    _credentialController.dispose();
    _eventController.dispose();
    _reasonController.dispose();
    super.dispose();
  }

  String? get _restrictedToken {
    final AppealAccessToken? access = _restrictedAccess;
    if (access == null) {
      return null;
    }
    final int now = DateTime.now().toUtc().millisecondsSinceEpoch ~/ 1000;
    return access.expiresAt > now ? access.accessToken : null;
  }

  Future<void> _load({String? appealToken}) async {
    final int generation = ++_requestGeneration;
    setState(() {
      _isLoading = true;
      _failure = null;
    });
    try {
      final List<Object> values = await Future.wait<Object>(<Future<Object>>[
        _repository.appeals(appealToken: appealToken),
        _repository.governanceNotices(appealToken: appealToken),
      ]);
      if (!mounted || generation != _requestGeneration) {
        return;
      }
      final AppealPage appeals = values[0] as AppealPage;
      final GovernanceNoticePage notices = values[1] as GovernanceNoticePage;
      setState(() {
        _appeals = appeals.items;
        _nextAppealCursor = appeals.nextCursor;
        _appealHasMore = appeals.hasMore;
        _notices = notices.items;
      });
      final String? appealId = widget.initialAppealId;
      if (appealId != null) {
        WidgetsBinding.instance.addPostFrameCallback((Duration _) {
          if (mounted) {
            Scrollable.ensureVisible(
              _appealKeys.putIfAbsent(appealId, GlobalKey.new).currentContext ??
                  context,
            );
          }
        });
      }
    } on ApiFailure catch (failure) {
      if (mounted && generation == _requestGeneration) {
        if (appealToken != null &&
            failure.kind == ApiFailureKind.unauthorized) {
          setState(() => _restrictedAccess = null);
        }
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted && generation == _requestGeneration) {
        setState(() => _isLoading = false);
      }
    }
  }

  final Map<String, GlobalKey> _appealKeys = <String, GlobalKey>{};

  Future<void> _loadMore() async {
    final String? cursor = _nextAppealCursor;
    if (_isLoadingMore || !_appealHasMore || cursor == null) {
      return;
    }
    setState(() => _isLoadingMore = true);
    try {
      final AppealPage page = await _repository.appeals(
        cursor: cursor,
        appealToken: _restrictedToken,
      );
      if (mounted) {
        final Set<String> ids = _appeals.map((Appeal item) => item.id).toSet();
        setState(() {
          _appeals.addAll(page.items.where((Appeal item) => ids.add(item.id)));
          _nextAppealCursor = page.nextCursor;
          _appealHasMore = page.hasMore;
        });
      }
    } on ApiFailure catch (failure) {
      _showMessage(failure.message);
    } finally {
      if (mounted) {
        setState(() => _isLoadingMore = false);
      }
    }
  }

  Future<void> _requestCode() async {
    if (_isRequestingCode || _validateEmail(_emailController.text) != null) {
      _accessFormKey.currentState?.validate();
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
            purpose: EmailCodePurpose.appeal,
          );
      if (mounted) {
        setState(() => _codeRequested = true);
        _showMessage('申诉验证码已发送');
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

  Future<void> _proveRestrictedAccess() async {
    if (_isSubmitting || !(_accessFormKey.currentState?.validate() ?? false)) {
      return;
    }
    if (_accessMethod == _AppealAccessMethod.emailCode && !_codeRequested) {
      setState(() {
        _failure = const ApiFailure(
          kind: ApiFailureKind.invalidInput,
          message: '请先完成人机验证并发送申诉验证码',
        );
      });
      return;
    }
    setState(() {
      _isSubmitting = true;
      _failure = null;
    });
    try {
      final AppealAccessToken access =
          _accessMethod == _AppealAccessMethod.password
          ? await _repository.passwordAccess(
              email: _emailController.text,
              password: _credentialController.text,
            )
          : await _repository.emailCodeAccess(
              email: _emailController.text,
              code: _credentialController.text,
            );
      if (!mounted) {
        return;
      }
      setState(() {
        _restrictedAccess = access;
        _credentialController.clear();
      });
      await _load(appealToken: access.accessToken);
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

  Future<void> _submitAppeal() async {
    if (_isSubmitting || !(_submitFormKey.currentState?.validate() ?? false)) {
      return;
    }
    setState(() {
      _isSubmitting = true;
      _failure = null;
    });
    final String eventId = _eventController.text.trim();
    final String reason = _reasonController.text.trim();
    final String fingerprint = '$eventId\u0000$reason';
    if (_pendingSubmitFingerprint != fingerprint) {
      _pendingSubmitFingerprint = fingerprint;
      _pendingSubmitKey = const Uuid().v4();
    }
    try {
      await _repository.submit(
        governanceEventId: eventId,
        reason: reason,
        idempotencyKey: _pendingSubmitKey!,
        appealToken: _restrictedToken,
      );
      if (mounted) {
        _reasonController.clear();
        _pendingSubmitFingerprint = null;
        _pendingSubmitKey = null;
        _showMessage('申诉已提交，将由独立工作人员复核');
        await _load(appealToken: _restrictedToken);
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        if (failure.kind == ApiFailureKind.unauthorized &&
            _restrictedAccess != null) {
          _leaveRestrictedAccess();
          _showMessage('申诉短期凭据已失效，请重新验证');
        } else {
          setState(() => _failure = failure);
        }
      }
    } finally {
      if (mounted) {
        setState(() => _isSubmitting = false);
      }
    }
  }

  Future<void> _withdraw(Appeal appeal) async {
    final String? reason = await _showWithdrawDialog();
    if (reason == null || !mounted) {
      return;
    }
    setState(() => _isSubmitting = true);
    try {
      await _repository.withdraw(
        appeal: appeal,
        reason: reason,
        appealToken: _restrictedToken,
      );
      if (mounted) {
        _showMessage('申诉已撤回；历史记录仍会保留');
        await _load(appealToken: _restrictedToken);
      }
    } on ApiFailure catch (failure) {
      _showMessage(failure.message);
      if (mounted) {
        await _load(appealToken: _restrictedToken);
      }
    } finally {
      if (mounted) {
        setState(() => _isSubmitting = false);
      }
    }
  }

  Future<String?> _showWithdrawDialog() async {
    final TextEditingController controller = TextEditingController();
    final GlobalKey<FormState> formKey = GlobalKey<FormState>();
    final String? result = await showDialog<String>(
      context: context,
      builder: (BuildContext context) => AlertDialog(
        title: const Text('撤回申诉？'),
        content: Form(
          key: formKey,
          child: TextFormField(
            controller: controller,
            maxLength: 1000,
            minLines: 2,
            maxLines: 5,
            decoration: const InputDecoration(
              labelText: '撤回原因',
              helperText: '撤回不会删除原处理或已经产生的申诉历史。',
            ),
            validator: _validateReason,
          ),
        ),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () {
              if (formKey.currentState?.validate() ?? false) {
                Navigator.of(context).pop(controller.text.trim());
              }
            },
            child: const Text('确认撤回'),
          ),
        ],
      ),
    );
    controller.dispose();
    return result;
  }

  Future<void> _openNotice(GovernanceNotice notice) async {
    if (!notice.read) {
      try {
        await _repository.markGovernanceNoticeRead(
          id: notice.id,
          appealToken: _restrictedToken,
        );
      } on ApiFailure catch (failure) {
        _showMessage(failure.message);
      }
    }
    final String? eventId = _eventIdFromTarget(notice.targetUrl);
    if (eventId != null && mounted) {
      setState(() => _eventController.text = eventId);
    }
  }

  void _leaveRestrictedAccess() {
    ++_requestGeneration;
    setState(() {
      _restrictedAccess = null;
      _appeals = <Appeal>[];
      _notices = <GovernanceNotice>[];
      _failure = null;
    });
  }

  void _showMessage(String message) {
    if (mounted) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(message)));
    }
  }

  @override
  Widget build(BuildContext context) {
    final AsyncValue<SessionState> session = ref.watch(sessionStateProvider);
    final SessionState? state = session.value;
    if (state != null && state.generation != _sessionGeneration) {
      _sessionGeneration = state.generation;
      ++_requestGeneration;
      _appeals = <Appeal>[];
      _notices = <GovernanceNotice>[];
      _failure = null;
      _isLoading = state.isAuthenticated;
      final int expectedGeneration = state.generation;
      WidgetsBinding.instance.addPostFrameCallback((Duration _) {
        if (!mounted ||
            ref.read(sessionStateProvider).value?.generation !=
                expectedGeneration) {
          return;
        }
        if (state.isAuthenticated) {
          setState(() => _restrictedAccess = null);
          unawaited(_load());
        }
      });
    }
    return Scaffold(
      appBar: AppBar(
        title: const Text('申诉中心'),
        actions: <Widget>[
          if (_restrictedAccess != null && state?.isAuthenticated != true)
            TextButton.icon(
              onPressed: _leaveRestrictedAccess,
              icon: const Icon(Icons.logout_rounded),
              label: const Text('退出受限访问'),
            ),
        ],
      ),
      body: SafeArea(top: false, child: _content(state)),
    );
  }

  Widget _content(SessionState? session) {
    if (session == null || session.phase == SessionPhase.restoring) {
      return const AppLoadingState(title: '正在确认申诉访问权限');
    }
    final bool canAccess = session.isAuthenticated || _restrictedToken != null;
    if (!canAccess) {
      return _restrictedAccessCard();
    }
    if (_isLoading && _appeals.isEmpty && _notices.isEmpty) {
      return const AppLoadingState(title: '正在加载申诉记录');
    }
    final ApiFailure? failure = _failure;
    if (failure != null && _appeals.isEmpty && _notices.isEmpty) {
      if (failure.kind == ApiFailureKind.forbidden) {
        return const AppPermissionState(description: '当前凭据不能访问此账号的申诉。');
      }
      return AppErrorState(
        description: failure.message,
        onRetry: () => _load(appealToken: _restrictedToken),
      );
    }
    return RefreshIndicator(
      onRefresh: () => _load(appealToken: _restrictedToken),
      child: ListView(
        padding: const EdgeInsets.all(16),
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      '申诉中心',
                      style: Theme.of(context).textTheme.headlineSmall,
                    ),
                    const SizedBox(height: 4),
                    const Text('原处理记录会保留，并由其他工作人员独立复核。'),
                  ],
                ),
              ),
              if (!session.isAuthenticated)
                const Chip(
                  avatar: Icon(Icons.lock_clock_outlined, size: 18),
                  label: Text('短期受限访问'),
                ),
            ],
          ),
          const SizedBox(height: 16),
          _submitCard(),
          if (_failure case final ApiFailure failure) ...<Widget>[
            const SizedBox(height: 12),
            _InlineFailure(failure: failure),
          ],
          if (_notices.isNotEmpty) ...<Widget>[
            const SizedBox(height: 20),
            Text('治理通知', style: Theme.of(context).textTheme.titleLarge),
            const SizedBox(height: 8),
            ..._notices.map(
              (GovernanceNotice notice) => Card(
                child: ListTile(
                  leading: const Icon(Icons.shield_outlined),
                  title: Text(notice.summary),
                  subtitle: Text(_formatUnix(notice.createdAt)),
                  trailing: notice.read
                      ? const Icon(Icons.done_rounded)
                      : const Chip(label: Text('未读')),
                  onTap: () => _openNotice(notice),
                ),
              ),
            ),
          ],
          const SizedBox(height: 20),
          Text('申诉记录', style: Theme.of(context).textTheme.titleLarge),
          const SizedBox(height: 8),
          if (_appeals.isEmpty)
            const SizedBox(
              height: 220,
              child: AppEmptyState(
                title: '还没有申诉记录',
                description: '只有属于当前账号、仍在 30 天窗口且后端认定可申诉的治理事件才能提交。',
              ),
            )
          else
            ..._appeals.map(
              (Appeal appeal) => Padding(
                key: _appealKeys.putIfAbsent(appeal.id, GlobalKey.new),
                padding: const EdgeInsets.only(bottom: 12),
                child: _AppealCard(
                  appeal: appeal,
                  isBusy: _isSubmitting,
                  onWithdraw: () => _withdraw(appeal),
                ),
              ),
            ),
          if (_appealHasMore)
            Center(
              child: OutlinedButton.icon(
                onPressed: _isLoadingMore ? null : _loadMore,
                icon: _isLoadingMore
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.expand_more_rounded),
                label: Text(_isLoadingMore ? '加载中' : '加载更多'),
              ),
            ),
        ],
      ),
    );
  }

  Widget _restrictedAccessCard() {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: Card(
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Form(
                key: _accessFormKey,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    const Icon(Icons.balance_rounded, size: 40),
                    const SizedBox(height: 12),
                    Text(
                      '安全进入申诉中心',
                      textAlign: TextAlign.center,
                      style: Theme.of(context).textTheme.titleLarge,
                    ),
                    const SizedBox(height: 8),
                    const Text(
                      '即使账号处于暂停状态，也可以用校园邮箱证明身份。这里签发的一小时短期凭据只在内存中保存，不能访问资料、内容、私信或积分。',
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 20),
                    SegmentedButton<_AppealAccessMethod>(
                      segments: const <ButtonSegment<_AppealAccessMethod>>[
                        ButtonSegment<_AppealAccessMethod>(
                          value: _AppealAccessMethod.password,
                          icon: Icon(Icons.password_rounded),
                          label: Text('密码'),
                        ),
                        ButtonSegment<_AppealAccessMethod>(
                          value: _AppealAccessMethod.emailCode,
                          icon: Icon(Icons.mark_email_read_outlined),
                          label: Text('邮箱验证码'),
                        ),
                      ],
                      selected: <_AppealAccessMethod>{_accessMethod},
                      onSelectionChanged: _isSubmitting
                          ? null
                          : (Set<_AppealAccessMethod> selection) {
                              setState(() {
                                _accessMethod = selection.single;
                                _credentialController.clear();
                                _codeRequested = false;
                                _failure = null;
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
                        labelText: '同济校园邮箱',
                        hintText: 'name@tongji.edu.cn',
                      ),
                      validator: _validateEmail,
                    ),
                    const SizedBox(height: 16),
                    TextFormField(
                      controller: _credentialController,
                      enabled: !_isSubmitting,
                      obscureText:
                          _accessMethod == _AppealAccessMethod.password &&
                          _obscurePassword,
                      keyboardType:
                          _accessMethod == _AppealAccessMethod.emailCode
                          ? TextInputType.number
                          : TextInputType.visiblePassword,
                      decoration: InputDecoration(
                        labelText: _accessMethod == _AppealAccessMethod.password
                            ? '密码'
                            : '申诉验证码',
                        suffixIcon:
                            _accessMethod == _AppealAccessMethod.password
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
                        if (_accessMethod == _AppealAccessMethod.password &&
                            credential.isEmpty) {
                          return '请输入密码';
                        }
                        if (_accessMethod == _AppealAccessMethod.emailCode &&
                            !RegExp(r'^\d{6}$').hasMatch(credential.trim())) {
                          return '请输入 6 位申诉验证码';
                        }
                        return null;
                      },
                    ),
                    if (_accessMethod ==
                        _AppealAccessMethod.emailCode) ...<Widget>[
                      const SizedBox(height: 12),
                      OutlinedButton.icon(
                        onPressed: _isRequestingCode ? null : _requestCode,
                        icon: const Icon(Icons.send_outlined),
                        label: Text(
                          _isRequestingCode
                              ? '发送中'
                              : _codeRequested
                              ? '重新发送验证码'
                              : '完成人机验证并发送验证码',
                        ),
                      ),
                    ],
                    if (_failure case final ApiFailure failure) ...<Widget>[
                      const SizedBox(height: 12),
                      _InlineFailure(failure: failure),
                    ],
                    const SizedBox(height: 20),
                    FilledButton.icon(
                      onPressed: _isSubmitting ? null : _proveRestrictedAccess,
                      icon: _isSubmitting
                          ? const SizedBox.square(
                              dimension: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.lock_open_rounded),
                      label: Text(_isSubmitting ? '正在验证' : '验证并进入'),
                    ),
                    const SizedBox(height: 8),
                    TextButton(
                      onPressed: () => context.push(AppRoutes.login),
                      child: const Text('账号正常？使用完整登录'),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _submitCard() {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Form(
          key: _submitFormKey,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              Text('提交申诉', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 6),
              const Text('从治理通知进入时会自动带入事件编号；手工编号不存在或不属于当前账号时，服务器会统一拒绝而不泄露信息。'),
              const SizedBox(height: 12),
              TextFormField(
                controller: _eventController,
                enabled: !_isSubmitting,
                keyboardType: TextInputType.number,
                decoration: const InputDecoration(labelText: '治理事件编号'),
                validator: (String? value) {
                  final int? event = int.tryParse(value?.trim() ?? '');
                  return event == null || event <= 0 ? '请输入有效的治理事件编号' : null;
                },
              ),
              const SizedBox(height: 12),
              TextFormField(
                controller: _reasonController,
                enabled: !_isSubmitting,
                minLines: 3,
                maxLines: 7,
                maxLength: 1000,
                decoration: const InputDecoration(
                  labelText: '申诉理由',
                  helperText: '请说明事实与希望复核的部分；不要提交无关个人信息。',
                ),
                validator: _validateReason,
              ),
              Align(
                alignment: Alignment.centerRight,
                child: FilledButton.icon(
                  onPressed: _isSubmitting ? null : _submitAppeal,
                  icon: const Icon(Icons.send_outlined),
                  label: const Text('提交申诉'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _AppealCard extends StatelessWidget {
  const _AppealCard({
    required this.appeal,
    required this.isBusy,
    required this.onWithdraw,
  });

  final Appeal appeal;
  final bool isBusy;
  final VoidCallback onWithdraw;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Wrap(
              spacing: 8,
              runSpacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: <Widget>[
                Chip(label: Text(_statusLabel(appeal.status))),
                Chip(label: Text(_targetLabel(appeal.targetKind))),
                Text('事件 #${appeal.governanceEventId}'),
              ],
            ),
            const SizedBox(height: 10),
            Text(
              appeal.originalReason ?? '处理原因未提供公开摘要',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 4),
            Text('你的申诉：${appeal.submissionReason}'),
            if (appeal.decisionReason case final String reason) ...<Widget>[
              const SizedBox(height: 6),
              Text('公开决定：$reason'),
            ],
            if (appeal.status == AppealStatus.submitted) ...<Widget>[
              const SizedBox(height: 10),
              Align(
                alignment: Alignment.centerRight,
                child: OutlinedButton(
                  onPressed: isBusy ? null : onWithdraw,
                  child: const Text('撤回申诉'),
                ),
              ),
            ],
            const Divider(height: 24),
            Text('状态历史', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 8),
            ...appeal.history.map(
              (AppealHistory event) => ListTile(
                dense: true,
                contentPadding: EdgeInsets.zero,
                leading: const Icon(Icons.circle, size: 10),
                title: Text(_statusLabel(event.toStatus)),
                subtitle: Text(event.reason),
                trailing: Text(_formatUnix(event.createdAt)),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _InlineFailure extends StatelessWidget {
  const _InlineFailure({required this.failure});

  final ApiFailure failure;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      liveRegion: true,
      child: Text(
        failure.message,
        style: TextStyle(color: Theme.of(context).colorScheme.error),
      ),
    );
  }
}

String? _validateEmail(String? value) {
  final String email = value?.trim().toLowerCase() ?? '';
  if (!email.endsWith('@tongji.edu.cn') ||
      email.length <= '@tongji.edu.cn'.length) {
    return '请输入有效的同济校园邮箱';
  }
  return null;
}

String? _validateReason(String? value) {
  final int length = value?.trim().length ?? 0;
  if (length < 3 || length > 1000) {
    return '请输入 3–1000 字理由';
  }
  return null;
}

String? _eventIdFromTarget(String targetUrl) {
  final Uri? uri = Uri.tryParse(targetUrl);
  if (uri == null || uri.path != '/appeals') {
    return null;
  }
  final String? event = uri.queryParameters['event'];
  final int? parsed = event == null ? null : int.tryParse(event);
  return parsed != null && parsed > 0 ? event : null;
}

String _statusLabel(AppealStatus status) => switch (status) {
  AppealStatus.submitted => '已提交',
  AppealStatus.inReview => '复核中',
  AppealStatus.upheld => '维持原处理',
  AppealStatus.overturned => '已撤销原处置',
  AppealStatus.amended => '已调整原处理',
  AppealStatus.withdrawn => '已撤回',
  AppealStatus.unknownDefaultOpenApi => '未知状态',
};

String _targetLabel(AppealTargetKindEnum target) => switch (target) {
  AppealTargetKindEnum.sanction => '账号制裁',
  AppealTargetKindEnum.forumThread => '社区主题',
  AppealTargetKindEnum.forumComment => '社区评论',
  AppealTargetKindEnum.review => '课程评价',
  AppealTargetKindEnum.unknownDefaultOpenApi => '未知对象',
};

String _formatUnix(int seconds) {
  final DateTime value = DateTime.fromMillisecondsSinceEpoch(
    seconds * 1000,
    isUtc: true,
  ).toLocal();
  String two(int number) => number.toString().padLeft(2, '0');
  return '${value.year}-${two(value.month)}-${two(value.day)} '
      '${two(value.hour)}:${two(value.minute)}';
}
