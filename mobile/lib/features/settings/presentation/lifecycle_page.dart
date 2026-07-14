import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:uuid/uuid.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/design/app_theme.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../account/data/account_repository.dart';
import '../../account/presentation/account_page_layout.dart';
import 'recent_auth_dialog.dart';

enum _LifecycleAction { deactivate, delete }

class LifecyclePage extends ConsumerStatefulWidget {
  const LifecyclePage({super.key});

  @override
  ConsumerState<LifecyclePage> createState() => _LifecyclePageState();
}

class _LifecyclePageState extends ConsumerState<LifecyclePage> {
  AccountLifecycle? _lifecycle;
  ApiFailure? _failure;
  bool _isLoading = true;
  _LifecycleAction? _submittingAction;
  String? _deactivateIdempotencyKey;
  String? _deleteIdempotencyKey;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _isLoading = true;
      _failure = null;
    });
    try {
      final AccountLifecycle lifecycle = await ref
          .read(accountRepositoryProvider)
          .getLifecycle();
      if (mounted) {
        setState(() => _lifecycle = lifecycle);
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _submit(_LifecycleAction action) async {
    if (_submittingAction != null) {
      return;
    }
    final bool confirmed = await _confirmImpact(action);
    if (!confirmed || !mounted) {
      return;
    }
    final bool verified = await ensureRecentAuthentication(context, ref);
    if (!verified || !mounted) {
      return;
    }
    setState(() {
      _submittingAction = action;
      _failure = null;
      if (action == _LifecycleAction.deactivate) {
        _deactivateIdempotencyKey ??= const Uuid().v4();
      } else {
        _deleteIdempotencyKey ??= const Uuid().v4();
      }
    });
    try {
      final AccountRepository repository = ref.read(accountRepositoryProvider);
      final AccountLifecycleMutation mutation =
          action == _LifecycleAction.deactivate
          ? await repository.deactivateAccount(_deactivateIdempotencyKey!)
          : await repository.requestAccountDeletion(_deleteIdempotencyKey!);
      if (action == _LifecycleAction.deactivate) {
        _deactivateIdempotencyKey = null;
      } else {
        _deleteIdempotencyKey = null;
      }
      await ref.read(sessionManagerProvider).logout();
      if (!mounted) {
        return;
      }
      await _showClosureResult(action, mutation);
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _submittingAction = null);
      }
    }
  }

  Future<bool> _confirmImpact(_LifecycleAction action) async {
    final TextEditingController confirmationController =
        TextEditingController();
    final String expected = action == _LifecycleAction.deactivate ? '停用' : '删除';
    final bool? result = await showDialog<bool>(
      context: context,
      builder: (BuildContext dialogContext) => StatefulBuilder(
        builder: (BuildContext context, StateSetter setDialogState) => AlertDialog(
          title: Text(
            action == _LifecycleAction.deactivate ? '停用账号' : '请求删除账号',
          ),
          content: SingleChildScrollView(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Text(
                  action == _LifecycleAction.deactivate
                      ? '停用会立即撤销全部设备会话并停止账号交互。这不是处罚，之后可以用密码或 recovery-purpose 邮件验证恢复。'
                      : '删除请求会立即撤销所有会话、停止公开展示与新互动，并开始持久删除任务。在 30 天恢复窗口内可恢复；一旦 purge 开始就不可恢复。法律/完整性要求的公共内容、治理历史和积分账本不会被改写。',
                ),
                const SizedBox(height: 16),
                TextField(
                  controller: confirmationController,
                  decoration: InputDecoration(labelText: '输入“$expected”以确认'),
                  onChanged: (_) => setDialogState(() {}),
                ),
              ],
            ),
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.pop(dialogContext, false),
              child: const Text('取消'),
            ),
            FilledButton(
              onPressed: confirmationController.text.trim() == expected
                  ? () => Navigator.pop(dialogContext, true)
                  : null,
              child: Text(
                action == _LifecycleAction.deactivate ? '继续停用' : '继续删除',
              ),
            ),
          ],
        ),
      ),
    );
    confirmationController.dispose();
    return result ?? false;
  }

  Future<void> _showClosureResult(
    _LifecycleAction action,
    AccountLifecycleMutation mutation,
  ) async {
    final AccountRepository repository = ref.read(accountRepositoryProvider);
    final bool? recovered = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (BuildContext dialogContext) => _ClosureResultDialog(
        action: action,
        mutation: mutation,
        onRecover: () async {
          try {
            final AccountLifecycle recovered = await repository.recoverAccount(
              mutation.recovery.recoveryToken,
            );
            if (recovered.state != AccountLifecycleState.active) {
              throw const ApiFailure(
                kind: ApiFailureKind.conflict,
                message: '服务器未返回 active 恢复状态',
              );
            }
            return null;
          } on ApiFailure catch (failure) {
            return failure;
          }
        },
      ),
    );
    if (!mounted) {
      return;
    }
    if (recovered == true) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('账号已恢复，请正常登录')));
      context.go(AppRoutes.login);
    } else {
      context.go(AppRoutes.account);
    }
  }

  @override
  Widget build(BuildContext context) {
    final Widget child;
    if (_isLoading) {
      child = const AppLoadingState(
        title: '正在读取账号生命周期',
        description: '危险操作前必须以服务器当前状态和最近认证为准。',
      );
    } else if (_lifecycle == null && _failure != null) {
      child = AccountFailureView(failure: _failure!, onRetry: _load);
    } else {
      child = _buildLifecycle(_lifecycle!);
    }
    return AccountPageLayout(title: '停用或删除账号', child: child);
  }

  Widget _buildLifecycle(AccountLifecycle lifecycle) {
    final YourTjPalette palette = Theme.of(context).extension<YourTjPalette>()!;
    return ListView(
      padding: const EdgeInsets.all(24),
      children: <Widget>[
        Card(
          child: ListTile(
            leading: const Icon(Icons.verified_user_outlined),
            title: Text('服务器状态：${_lifecycleLabel(lifecycle.state)}'),
            subtitle: Text('生命周期版本 ${lifecycle.lifecycleVersion}'),
          ),
        ),
        const SizedBox(height: 16),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Text('停用账号', style: Theme.of(context).textTheme.titleLarge),
                const SizedBox(height: 8),
                const Text('适合暂时离开：立即退出全部设备，可用密码或邮件验证恢复。'),
                const SizedBox(height: 12),
                OutlinedButton.icon(
                  onPressed:
                      lifecycle.state == AccountLifecycleState.active &&
                          _submittingAction == null
                      ? () => _submit(_LifecycleAction.deactivate)
                      : null,
                  icon: const Icon(Icons.pause_circle_outline_rounded),
                  label: Text(
                    _submittingAction == _LifecycleAction.deactivate
                        ? '正在停用'
                        : '停用账号',
                  ),
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Text(
                  '请求删除账号',
                  style: Theme.of(
                    context,
                  ).textTheme.titleLarge?.copyWith(color: palette.destructive),
                ),
                const SizedBox(height: 8),
                const Text('立即关闭账号并进入 30 天恢复窗口，随后按数据保留规则执行不可逆 purge。'),
                const SizedBox(height: 12),
                FilledButton.icon(
                  style: FilledButton.styleFrom(
                    backgroundColor: palette.destructive,
                    foregroundColor: Colors.white,
                  ),
                  onPressed:
                      lifecycle.state == AccountLifecycleState.active &&
                          _submittingAction == null
                      ? () => _submit(_LifecycleAction.delete)
                      : null,
                  icon: const Icon(Icons.delete_forever_outlined),
                  label: Text(
                    _submittingAction == _LifecycleAction.delete
                        ? '正在提交删除请求'
                        : '请求删除账号',
                  ),
                ),
              ],
            ),
          ),
        ),
        if (_failure != null) ...<Widget>[
          const SizedBox(height: 16),
          AppErrorState(
            title: '账号生命周期操作失败',
            description: _failure!.message,
            onRetry: _load,
          ),
        ],
      ],
    );
  }
}

class _ClosureResultDialog extends StatefulWidget {
  const _ClosureResultDialog({
    required this.action,
    required this.mutation,
    required this.onRecover,
  });

  final _LifecycleAction action;
  final AccountLifecycleMutation mutation;
  final Future<ApiFailure?> Function() onRecover;

  @override
  State<_ClosureResultDialog> createState() => _ClosureResultDialogState();
}

class _ClosureResultDialogState extends State<_ClosureResultDialog> {
  bool _isRecovering = false;
  ApiFailure? _failure;

  Future<void> _recover() async {
    setState(() {
      _isRecovering = true;
      _failure = null;
    });
    final ApiFailure? failure = await widget.onRecover();
    if (!mounted) {
      return;
    }
    if (failure == null) {
      Navigator.pop(context, true);
      return;
    }
    setState(() {
      _failure = failure;
      _isRecovering = false;
    });
  }

  @override
  Widget build(BuildContext context) {
    final AccountLifecycle lifecycle = widget.mutation.lifecycle;
    return AlertDialog(
      title: Text(
        widget.action == _LifecycleAction.deactivate ? '账号已停用' : '删除请求已接受',
      ),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            const Text('全部旧会话已撤销，本机也已清除 refresh token。'),
            if (lifecycle.recoverUntil != null) ...<Widget>[
              const SizedBox(height: 8),
              Text('恢复窗口截止：${formatAccountTime(lifecycle.recoverUntil!)}'),
            ],
            const SizedBox(height: 8),
            const Text(
              '这个 15 分钟恢复凭据只留在当前内存对话中，不显示、不复制、不写磁盘。关闭后仍可从登录页使用密码或 recovery-purpose 邮件验证恢复。',
            ),
            if (_failure != null) ...<Widget>[
              const SizedBox(height: 12),
              Text(
                _failure!.message,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
          ],
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _isRecovering ? null : () => Navigator.pop(context, false),
          child: const Text('保持当前状态'),
        ),
        FilledButton(
          onPressed: _isRecovering ? null : _recover,
          child: Text(_isRecovering ? '正在恢复' : '立即恢复账号'),
        ),
      ],
    );
  }
}

String _lifecycleLabel(AccountLifecycleState state) {
  return switch (state) {
    AccountLifecycleState.active => '活跃',
    AccountLifecycleState.deactivated => '已停用',
    AccountLifecycleState.deletionRequested => '已请求删除',
    AccountLifecycleState.deleted => '已删除（可能仍在恢复窗口）',
    AccountLifecycleState.purged => '已不可逆清理',
    AccountLifecycleState.unknownDefaultOpenApi => '未知（已禁用危险操作）',
  };
}
