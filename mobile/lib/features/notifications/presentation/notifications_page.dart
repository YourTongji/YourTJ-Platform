import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart' hide Notification;
import 'package:yourtj_api/yourtj_api.dart' as api show Notification;

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../auth/domain/session_state.dart';
import '../../messages/domain/message_badge_counts.dart';
import '../data/notifications_repository.dart';

class NotificationsPage extends ConsumerStatefulWidget {
  const NotificationsPage({this.embedded = false, super.key});

  final bool embedded;

  @override
  ConsumerState<NotificationsPage> createState() => _NotificationsPageState();
}

class _NotificationsPageState extends ConsumerState<NotificationsPage>
    with WidgetsBindingObserver, AutomaticKeepAliveClientMixin {
  List<api.Notification> _notifications = <api.Notification>[];
  List<GovernanceNotice> _governance = <GovernanceNotice>[];
  String? _notificationCursor;
  String? _governanceCursor;
  bool _notificationHasMore = false;
  bool _governanceHasMore = false;
  bool _unreadOnly = false;
  bool _isLoading = true;
  bool _isLoadingMore = false;
  bool _isMutating = false;
  int _unreadCount = 0;
  int _governanceUnreadCount = 0;
  int _requestGeneration = 0;
  int? _sessionGeneration;
  ApiFailure? _failure;

  NotificationsRepository get _repository =>
      ref.read(notificationsRepositoryProvider);

  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed && _isAuthenticated) {
      unawaited(_refresh());
    }
  }

  bool get _isAuthenticated =>
      ref.read(sessionStateProvider).value?.isAuthenticated ?? false;

  Future<void> _refresh() async {
    final int generation = ++_requestGeneration;
    setState(() {
      _isLoading = true;
      _failure = null;
    });
    try {
      final List<Object> values = await Future.wait<Object>(<Future<Object>>[
        _repository.notifications(unreadOnly: _unreadOnly),
        _repository.governanceNotices(unreadOnly: _unreadOnly),
        _repository.unreadCount(),
        _repository.governanceUnreadCount(),
      ]);
      if (!mounted || generation != _requestGeneration) {
        return;
      }
      final NotificationPage notifications = values[0] as NotificationPage;
      final GovernanceNoticePage governance = values[1] as GovernanceNoticePage;
      setState(() {
        _notifications = notifications.items;
        _notificationCursor = notifications.nextCursor;
        _notificationHasMore = notifications.hasMore;
        _governance = governance.items;
        _governanceCursor = governance.nextCursor;
        _governanceHasMore = governance.hasMore;
        _unreadCount = values[2] as int;
        _governanceUnreadCount = values[3] as int;
      });
      ref.invalidate(messageBadgeCountsProvider);
    } on ApiFailure catch (failure) {
      if (mounted && generation == _requestGeneration) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted && generation == _requestGeneration) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _loadMore() async {
    if (_isLoadingMore || (!_notificationHasMore && !_governanceHasMore)) {
      return;
    }
    setState(() => _isLoadingMore = true);
    try {
      final Future<NotificationPage?> notificationRequest =
          _notificationHasMore && _notificationCursor != null
          ? _repository.notifications(
              unreadOnly: _unreadOnly,
              cursor: _notificationCursor,
            )
          : Future<NotificationPage?>.value();
      final Future<GovernanceNoticePage?> governanceRequest =
          _governanceHasMore && _governanceCursor != null
          ? _repository.governanceNotices(
              unreadOnly: _unreadOnly,
              cursor: _governanceCursor,
            )
          : Future<GovernanceNoticePage?>.value();
      final List<Object?> values = await Future.wait<Object?>(<Future<Object?>>[
        notificationRequest,
        governanceRequest,
      ]);
      if (!mounted) {
        return;
      }
      final NotificationPage? notificationPage = values[0] as NotificationPage?;
      final GovernanceNoticePage? governancePage =
          values[1] as GovernanceNoticePage?;
      setState(() {
        if (notificationPage != null) {
          final Set<String> ids = _notifications
              .map((api.Notification item) => item.id)
              .toSet();
          _notifications.addAll(
            notificationPage.items.where(
              (api.Notification item) => ids.add(item.id),
            ),
          );
          _notificationCursor = notificationPage.nextCursor;
          _notificationHasMore = notificationPage.hasMore;
        }
        if (governancePage != null) {
          final Set<String> ids = _governance
              .map((GovernanceNotice item) => item.id)
              .toSet();
          _governance.addAll(
            governancePage.items.where(
              (GovernanceNotice item) => ids.add(item.id),
            ),
          );
          _governanceCursor = governancePage.nextCursor;
          _governanceHasMore = governancePage.hasMore;
        }
      });
    } on ApiFailure catch (failure) {
      _showMessage(failure.message);
    } finally {
      if (mounted) {
        setState(() => _isLoadingMore = false);
      }
    }
  }

  Future<void> _markAllRead() async {
    if (_isMutating || _unreadCount + _governanceUnreadCount == 0) {
      return;
    }
    setState(() => _isMutating = true);
    try {
      await Future.wait<void>(<Future<void>>[
        _repository.markAllNotificationsRead(),
        _repository.markAllGovernanceNoticesRead(),
      ]);
      if (mounted) {
        _showMessage('全部通知已标记为已读');
        await _refresh();
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        _showMessage('${failure.message}；请刷新确认两类通知的服务器状态');
        await _refresh();
      }
    } finally {
      if (mounted) {
        setState(() => _isMutating = false);
      }
    }
  }

  Future<void> _openNotification(api.Notification item) async {
    if (!item.read) {
      await _runMutation(
        () => _repository.markNotificationsRead(<String>[item.id]),
      );
    }
    if (!mounted) {
      return;
    }
    final String? target = NotificationTarget.resolve(item.targetUrl);
    if (target == null) {
      _showMessage('此通知没有可安全打开的移动端目标');
      return;
    }
    await context.push(target);
  }

  Future<void> _openGovernance(GovernanceNotice item) async {
    if (!item.read) {
      await _runMutation(
        () => _repository.markGovernanceNoticesRead(<String>[item.id]),
      );
    }
    if (!mounted) {
      return;
    }
    final String? target = NotificationTarget.resolve(item.targetUrl);
    if (target == null) {
      _showMessage('此平台通知没有可安全打开的移动端目标');
      return;
    }
    await context.push(target);
  }

  Future<void> _runMutation(Future<void> Function() operation) async {
    if (_isMutating) {
      return;
    }
    setState(() => _isMutating = true);
    try {
      await operation();
      if (mounted) {
        await _refresh();
      }
    } on ApiFailure catch (failure) {
      _showMessage(failure.message);
    } finally {
      if (mounted) {
        setState(() => _isMutating = false);
      }
    }
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
    super.build(context);
    final AsyncValue<SessionState> session = ref.watch(sessionStateProvider);
    final SessionState? state = session.value;
    if (state != null && state.generation != _sessionGeneration) {
      _sessionGeneration = state.generation;
      ++_requestGeneration;
      _notifications = <api.Notification>[];
      _governance = <GovernanceNotice>[];
      _notificationCursor = null;
      _governanceCursor = null;
      _notificationHasMore = false;
      _governanceHasMore = false;
      _unreadCount = 0;
      _governanceUnreadCount = 0;
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
          unawaited(_refresh());
        }
      });
    }
    final Widget content = _content(state);
    if (widget.embedded) {
      return content;
    }
    return Scaffold(
      appBar: AppBar(title: const Text('通知')),
      body: SafeArea(top: false, child: content),
    );
  }

  Widget _content(SessionState? session) {
    if (session == null || session.phase == SessionPhase.restoring) {
      return const AppLoadingState(title: '正在恢复账号');
    }
    if (!session.isAuthenticated) {
      return AppEmptyState(
        title: '登录后查看通知',
        description: '回复、提及、点赞、私信和不可关闭的治理通知只对当前账号可见。',
        action: FilledButton.icon(
          onPressed: () => context.push(AppRoutes.login),
          icon: const Icon(Icons.login_rounded),
          label: const Text('登录'),
        ),
      );
    }
    if (_isLoading && _notifications.isEmpty && _governance.isEmpty) {
      return const AppLoadingState(title: '正在加载通知');
    }
    final ApiFailure? failure = _failure;
    if (failure != null && _notifications.isEmpty && _governance.isEmpty) {
      if (failure.kind == ApiFailureKind.forbidden) {
        return const AppPermissionState(description: '当前凭据不能访问普通账号通知。');
      }
      return AppErrorState(description: failure.message, onRetry: _refresh);
    }
    return RefreshIndicator(
      onRefresh: _refresh,
      child: CustomScrollView(
        key: const PageStorageKey<String>('notifications-scroll'),
        slivers: <Widget>[
          SliverToBoxAdapter(child: _header()),
          if (_governance.isNotEmpty)
            SliverToBoxAdapter(child: _governanceSection()),
          if (_notifications.isEmpty && _governance.isEmpty)
            SliverFillRemaining(
              hasScrollBody: false,
              child: AppEmptyState(
                title: _unreadOnly ? '没有未读通知' : '没有通知',
                description: _unreadOnly ? '当前账号的两类通知都已读。' : '新的平台与互动消息会显示在这里。',
              ),
            )
          else
            SliverPadding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
              sliver: SliverList.separated(
                itemCount: _notifications.length,
                separatorBuilder: (_, _) => const SizedBox(height: 10),
                itemBuilder: (BuildContext context, int index) {
                  return _NotificationCard(
                    item: _notifications[index],
                    isBusy: _isMutating,
                    onOpen: () => _openNotification(_notifications[index]),
                    onMarkRead: () => _runMutation(
                      () => _repository.markNotificationsRead(<String>[
                        _notifications[index].id,
                      ]),
                    ),
                  );
                },
              ),
            ),
          if (_notificationHasMore || _governanceHasMore)
            SliverToBoxAdapter(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(16, 0, 16, 24),
                child: Center(
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
              ),
            ),
        ],
      ),
    );
  }

  Widget _header() {
    final int total = _unreadCount + _governanceUnreadCount;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      '通知',
                      style: Theme.of(context).textTheme.headlineSmall,
                    ),
                    const SizedBox(height: 4),
                    Text(
                      total == 0 ? '当前没有未读消息' : '共 $total 条未读消息',
                      style: Theme.of(context).textTheme.bodyMedium,
                    ),
                  ],
                ),
              ),
              OutlinedButton.icon(
                onPressed: total == 0 || _isMutating ? null : _markAllRead,
                icon: const Icon(Icons.done_all_rounded),
                label: const Text('全部已读'),
              ),
            ],
          ),
          const SizedBox(height: 12),
          SegmentedButton<bool>(
            segments: <ButtonSegment<bool>>[
              const ButtonSegment<bool>(value: false, label: Text('全部')),
              ButtonSegment<bool>(
                value: true,
                label: Text(total == 0 ? '未读' : '未读 $total'),
              ),
            ],
            selected: <bool>{_unreadOnly},
            onSelectionChanged: (Set<bool> selection) {
              setState(() => _unreadOnly = selection.single);
              unawaited(_refresh());
            },
          ),
          const SizedBox(height: 12),
          const Card(
            child: ListTile(
              dense: true,
              leading: Icon(Icons.sync_rounded),
              title: Text('回到前台或下拉时会同步服务器事实'),
              subtitle: Text('实时流目前不由生成客户端安全消费；后台系统推送尚未开放。'),
            ),
          ),
        ],
      ),
    );
  }

  Widget _governanceSection() {
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Text('平台通知', style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 4),
          const Text('账号安全、内容处置和申诉消息不可通过互动偏好关闭。'),
          const SizedBox(height: 10),
          ..._governance.map(
            (GovernanceNotice item) => Padding(
              padding: const EdgeInsets.only(bottom: 10),
              child: _GovernanceNoticeCard(
                item: item,
                isBusy: _isMutating,
                onOpen: () => _openGovernance(item),
                onMarkRead: () => _runMutation(
                  () =>
                      _repository.markGovernanceNoticesRead(<String>[item.id]),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _NotificationCard extends StatelessWidget {
  const _NotificationCard({
    required this.item,
    required this.isBusy,
    required this.onOpen,
    required this.onMarkRead,
  });

  final api.Notification item;
  final bool isBusy;
  final VoidCallback onOpen;
  final VoidCallback onMarkRead;

  @override
  Widget build(BuildContext context) {
    final String? excerpt =
        _payloadText(item.payload, 'bodyExcerpt') ??
        _payloadText(item.payload, 'reason');
    return Card(
      color: item.read
          ? null
          : Theme.of(
              context,
            ).colorScheme.primaryContainer.withValues(alpha: 0.3),
      child: ListTile(
        contentPadding: const EdgeInsets.fromLTRB(16, 10, 8, 10),
        leading: const CircleAvatar(child: Icon(Icons.notifications_outlined)),
        title: Text(_payloadTitle(item.payload)),
        subtitle: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            if (excerpt != null)
              Text(excerpt, maxLines: 2, overflow: TextOverflow.ellipsis),
            const SizedBox(height: 4),
            Text(
              '${_notificationLabel(item.type)} · ${_formatUnix(item.createdAt)}',
            ),
          ],
        ),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            if (!item.read)
              IconButton(
                tooltip: '标记为已读',
                onPressed: isBusy ? null : onMarkRead,
                icon: const Icon(Icons.done_rounded),
              ),
            if (item.targetUrl != null)
              IconButton(
                tooltip: '查看详情',
                onPressed: isBusy ? null : onOpen,
                icon: const Icon(Icons.chevron_right_rounded),
              ),
          ],
        ),
        onTap: item.targetUrl == null || isBusy ? null : onOpen,
      ),
    );
  }
}

class _GovernanceNoticeCard extends StatelessWidget {
  const _GovernanceNoticeCard({
    required this.item,
    required this.isBusy,
    required this.onOpen,
    required this.onMarkRead,
  });

  final GovernanceNotice item;
  final bool isBusy;
  final VoidCallback onOpen;
  final VoidCallback onMarkRead;

  @override
  Widget build(BuildContext context) {
    return Card(
      color: item.read
          ? null
          : Theme.of(
              context,
            ).colorScheme.primaryContainer.withValues(alpha: 0.3),
      child: ListTile(
        contentPadding: const EdgeInsets.fromLTRB(16, 10, 8, 10),
        leading: const CircleAvatar(child: Icon(Icons.shield_outlined)),
        title: Text(item.summary),
        subtitle: Text('平台通知 · ${_formatUnix(item.createdAt)}'),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            if (!item.read)
              IconButton(
                tooltip: '标记平台通知为已读',
                onPressed: isBusy ? null : onMarkRead,
                icon: const Icon(Icons.done_rounded),
              ),
            IconButton(
              tooltip: '前往申诉中心',
              onPressed: isBusy ? null : onOpen,
              icon: const Icon(Icons.chevron_right_rounded),
            ),
          ],
        ),
        onTap: isBusy ? null : onOpen,
      ),
    );
  }
}

String _payloadTitle(Map<String, Object> payload) =>
    _payloadText(payload, 'title') ??
    _payloadText(payload, 'threadTitle') ??
    _payloadText(payload, 'badgeName') ??
    _payloadText(payload, 'body') ??
    '系统通知';

String? _payloadText(Map<String, Object> payload, String key) {
  final Object? value = payload[key];
  return value is String && value.trim().isNotEmpty ? value.trim() : null;
}

String _notificationLabel(String type) => switch (type) {
  'badge' => '徽章',
  'achievement_awarded' => '成就',
  'achievement_revoked' => '成就变更',
  'dm' => '私信',
  'dm_request' => '消息请求',
  'dm_request_accepted' => '请求已接受',
  'flag_auto_hide' => '内容处理',
  'follow' => '新关注',
  'mention' => '提及',
  'mod_action' => '管理通知',
  'quote' => '引用回复',
  'reply' => '回复',
  'vote' => '点赞',
  'watching' => '订阅更新',
  'verification_expired' => '认证到期',
  'verification_granted' => '认证',
  'verification_revoked' => '认证变更',
  _ => '系统通知',
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
