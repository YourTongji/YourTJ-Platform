import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../account/data/account_repository.dart';
import '../../account/presentation/account_page_layout.dart';

class SessionsPage extends ConsumerStatefulWidget {
  const SessionsPage({super.key});

  @override
  ConsumerState<SessionsPage> createState() => _SessionsPageState();
}

class _SessionsPageState extends ConsumerState<SessionsPage> {
  final List<Session> _sessions = <Session>[];
  ApiFailure? _failure;
  String? _nextCursor;
  bool _hasMore = false;
  bool _isLoading = true;
  bool _isLoadingMore = false;
  bool _isRevokingOthers = false;
  bool _isRevokingAll = false;
  String? _revokingId;

  @override
  void initState() {
    super.initState();
    _load(reset: true);
  }

  Future<void> _load({required bool reset}) async {
    if (reset) {
      setState(() {
        _isLoading = true;
        _failure = null;
      });
    } else {
      if (_isLoadingMore || !_hasMore) {
        return;
      }
      setState(() => _isLoadingMore = true);
    }
    try {
      final SessionPage page = await ref
          .read(accountRepositoryProvider)
          .getSessions(cursor: reset ? null : _nextCursor);
      if (!mounted) {
        return;
      }
      setState(() {
        if (reset) {
          _sessions.clear();
        }
        _sessions.addAll(page.items);
        _nextCursor = page.nextCursor;
        _hasMore = page.hasMore;
        _failure = null;
      });
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _isLoadingMore = false;
        });
      }
    }
  }

  Future<void> _revoke(Session session) async {
    final bool? confirmed = await showDialog<bool>(
      context: context,
      builder: (BuildContext dialogContext) => AlertDialog(
        title: Text(session.isCurrent ? '退出当前设备？' : '撤销该设备会话？'),
        content: Text(
          session.isCurrent
              ? '撤销后本机会立即清除安全存储中的 refresh token，需要重新登录。'
              : '该设备的 access/refresh 凭据将失效，当前设备不受影响。',
        ),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(dialogContext, true),
            child: Text(session.isCurrent ? '退出当前设备' : '撤销会话'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    setState(() {
      _revokingId = session.id;
      _failure = null;
    });
    try {
      await ref.read(accountRepositoryProvider).revokeSession(session.id);
      if (session.isCurrent) {
        await ref.read(sessionManagerProvider).logout();
        if (mounted) {
          context.go(AppRoutes.account);
        }
        return;
      }
      if (mounted) {
        setState(
          () => _sessions.removeWhere((Session item) => item.id == session.id),
        );
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _revokingId = null);
      }
    }
  }

  Future<void> _revokeOthers() async {
    final bool? confirmed = await showDialog<bool>(
      context: context,
      builder: (BuildContext dialogContext) => AlertDialog(
        title: const Text('退出其他所有设备？'),
        content: const Text('除当前 server-bound session 外的全部会话都会被撤销。'),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(dialogContext, true),
            child: const Text('退出其他设备'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    setState(() {
      _isRevokingOthers = true;
      _failure = null;
    });
    try {
      await ref.read(accountRepositoryProvider).revokeOtherSessions();
      await _load(reset: true);
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isRevokingOthers = false);
      }
    }
  }

  Future<void> _revokeAll() async {
    final bool? confirmed = await showDialog<bool>(
      context: context,
      builder: (BuildContext dialogContext) => AlertDialog(
        title: const Text('退出全部设备？'),
        content: const Text(
          '当前设备也会立即清除安全存储中的 refresh token，全部 server-side session family 将被撤销。',
        ),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(dialogContext, true),
            child: const Text('退出全部设备'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    setState(() {
      _isRevokingAll = true;
      _failure = null;
    });
    final bool revoked = await ref
        .read(sessionManagerProvider)
        .logout(revokeAll: true);
    if (!mounted) {
      return;
    }
    if (!revoked) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('已清除本机登录；服务器全部撤销结果暂时无法确认')));
    }
    context.go(AppRoutes.account);
  }

  @override
  Widget build(BuildContext context) {
    final Widget child;
    if (_isLoading) {
      child = const AppLoadingState(
        title: '正在读取设备会话',
        description: '设备标签只用于本人安全中心，不展示精确历史 IP。',
      );
    } else if (_sessions.isEmpty && _failure != null) {
      child = AccountFailureView(
        failure: _failure!,
        onRetry: () => _load(reset: true),
      );
    } else if (_sessions.isEmpty) {
      child = const AppEmptyState(
        title: '没有可见的活跃会话',
        description: '如果本机仍显示已登录，请刷新或重新登录。',
      );
    } else {
      child = _buildSessions();
    }
    return AccountPageLayout(title: '设备与会话', child: child);
  }

  Widget _buildSessions() {
    return RefreshIndicator(
      onRefresh: () => _load(reset: true),
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(16),
        children: <Widget>[
          Card(
            child: ListTile(
              leading: const Icon(Icons.devices_other_rounded),
              title: const Text('一键退出其他设备'),
              subtitle: const Text('保留当前设备，撤销其他全部会话。'),
              trailing: _isRevokingOthers
                  ? const CircularProgressIndicator()
                  : const Icon(Icons.chevron_right_rounded),
              onTap: _isRevokingOthers ? null : _revokeOthers,
            ),
          ),
          Card(
            child: ListTile(
              leading: const Icon(Icons.phonelink_erase_rounded),
              title: const Text('退出全部设备'),
              subtitle: const Text('包括本机在内撤销全部会话，完成后需重新登录。'),
              trailing: _isRevokingAll
                  ? const CircularProgressIndicator()
                  : const Icon(Icons.chevron_right_rounded),
              onTap: _isRevokingAll ? null : _revokeAll,
            ),
          ),
          const SizedBox(height: 8),
          ..._sessions.map((Session session) {
            final bool isRevoking = _revokingId == session.id;
            return Card(
              child: ListTile(
                leading: Icon(
                  session.isCurrent
                      ? Icons.smartphone_rounded
                      : Icons.devices_rounded,
                ),
                title: Text(
                  session.deviceLabel?.trim().isNotEmpty == true
                      ? session.deviceLabel!
                      : '未命名设备',
                ),
                subtitle: Text(
                  '${session.isCurrent ? '当前设备 · ' : ''}'
                  '最后使用 ${formatAccountTime(session.lastUsedAt)}\n'
                  '会话到期 ${formatAccountTime(session.expiresAt)}',
                ),
                isThreeLine: true,
                trailing: isRevoking
                    ? const CircularProgressIndicator()
                    : IconButton(
                        tooltip: session.isCurrent ? '退出当前设备' : '撤销该会话',
                        onPressed: () => _revoke(session),
                        icon: const Icon(Icons.logout_rounded),
                      ),
              ),
            );
          }),
          if (_failure != null) ...<Widget>[
            const SizedBox(height: 8),
            AppErrorState(
              title: '会话操作失败',
              description: _failure!.message,
              onRetry: () => _load(reset: true),
            ),
          ],
          if (_hasMore) ...<Widget>[
            const SizedBox(height: 8),
            OutlinedButton(
              onPressed: _isLoadingMore ? null : () => _load(reset: false),
              child: _isLoadingMore
                  ? const CircularProgressIndicator()
                  : const Text('加载更多会话'),
            ),
          ],
        ],
      ),
    );
  }
}
