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
import '../../../core/widgets/platform_avatar.dart';
import '../../auth/domain/session_state.dart';
import '../data/messages_repository.dart';
import '../domain/message_badge_counts.dart';
import 'message_dialogs.dart';

class DirectMessagesPage extends ConsumerStatefulWidget {
  const DirectMessagesPage({
    required this.initialView,
    this.initialConversationId,
    super.key,
  });

  final ConversationView initialView;
  final String? initialConversationId;

  @override
  ConsumerState<DirectMessagesPage> createState() => _DirectMessagesPageState();
}

class _DirectMessagesPageState extends ConsumerState<DirectMessagesPage>
    with WidgetsBindingObserver, AutomaticKeepAliveClientMixin {
  final TextEditingController _searchController = TextEditingController();
  late ConversationView _view = widget.initialView;
  late String? _pendingDeepLinkId = widget.initialConversationId;
  List<DmConversation> _conversations = <DmConversation>[];
  DmConversation? _selected;
  DmCounts? _counts;
  String? _nextCursor;
  bool _hasMore = false;
  bool _isLoading = true;
  bool _isLoadingMore = false;
  bool _isMutating = false;
  bool _missingDeepLink = false;
  NewConversationDraft? _pendingStartDraft;
  String? _pendingStartKey;
  int _requestGeneration = 0;
  int? _sessionGeneration;
  ApiFailure? _failure;

  MessagesRepository get _repository => ref.read(messagesRepositoryProvider);

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
    _searchController.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed && _isAuthenticated) {
      unawaited(_load());
    }
  }

  bool get _isAuthenticated =>
      ref.read(sessionStateProvider).value?.isAuthenticated ?? false;

  Future<void> _load() async {
    final int generation = ++_requestGeneration;
    setState(() {
      _isLoading = true;
      _failure = null;
      _missingDeepLink = false;
    });
    try {
      DmConversationPage page = await _repository.conversations(
        view: _view,
        query: _searchController.text,
      );
      final List<DmConversation> items = <DmConversation>[...page.items];
      String? cursor = page.nextCursor;
      bool hasMore = page.hasMore;
      final String? deepLinkId = _pendingDeepLinkId;
      int chasedPages = 0;
      while (deepLinkId != null &&
          !items.any((DmConversation item) => item.id == deepLinkId) &&
          hasMore &&
          cursor != null &&
          chasedPages < 5) {
        page = await _repository.conversations(
          view: _view,
          query: _searchController.text,
          cursor: cursor,
        );
        items.addAll(page.items);
        cursor = page.nextCursor;
        hasMore = page.hasMore;
        chasedPages += 1;
      }
      final DmCounts counts = await _repository.counts();
      if (!mounted || generation != _requestGeneration) {
        return;
      }
      DmConversation? selected = _selected;
      if (deepLinkId != null) {
        selected = _findById(items, deepLinkId);
      } else if (selected != null) {
        selected = _findById(items, selected.id);
      }
      setState(() {
        _conversations = _deduplicate(items);
        _nextCursor = cursor;
        _hasMore = hasMore;
        _counts = counts;
        _selected = selected;
        _missingDeepLink = deepLinkId != null && selected == null;
        _pendingDeepLinkId = null;
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
    final String? cursor = _nextCursor;
    if (_isLoadingMore || !_hasMore || cursor == null) {
      return;
    }
    setState(() => _isLoadingMore = true);
    try {
      final DmConversationPage page = await _repository.conversations(
        view: _view,
        query: _searchController.text,
        cursor: cursor,
      );
      if (mounted) {
        setState(() {
          _conversations = _deduplicate(<DmConversation>[
            ..._conversations,
            ...page.items,
          ]);
          _nextCursor = page.nextCursor;
          _hasMore = page.hasMore;
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

  Future<void> _selectView(ConversationView view) async {
    if (_view == view) {
      return;
    }
    setState(() {
      _view = view;
      _selected = null;
    });
    await _load();
  }

  Future<void> _startConversation({bool retry = false}) async {
    final NewConversationDraft? draft = retry
        ? _pendingStartDraft
        : await showNewConversationDialog(context);
    if (draft == null || !mounted) {
      return;
    }
    if (!retry) {
      _pendingStartDraft = draft;
      _pendingStartKey = const Uuid().v4();
    }
    final String idempotencyKey = _pendingStartKey ??= const Uuid().v4();
    setState(() => _isMutating = true);
    try {
      final DmConversation conversation = await _repository.start(
        recipientHandle: draft.handle,
        requestMessage: draft.requestMessage,
        idempotencyKey: idempotencyKey,
      );
      if (!mounted) {
        return;
      }
      final ConversationView nextView =
          conversation.requestStatus == DmConversationRequestStatusEnum.pending
          ? ConversationView.sent
          : ConversationView.inbox;
      setState(() {
        _view = nextView;
        _selected = conversation;
        _pendingStartDraft = null;
        _pendingStartKey = null;
      });
      _showMessage(
        conversation.canSend
            ? '已打开与 @${conversation.participantHandle} 的对话'
            : '消息请求已发送；对方接受前不能继续发送',
      );
      await _load();
    } on ApiFailure catch (failure) {
      _showMessage('${failure.message}；发送结果可能不确定，可使用页面中的“按原请求重试”保留同一幂等键');
    } finally {
      if (mounted) {
        setState(() => _isMutating = false);
      }
    }
  }

  Future<void> _conversationChanged({
    DmConversation? replacement,
    ConversationView? view,
  }) async {
    if (view != null) {
      _view = view;
    }
    if (replacement != null) {
      _selected = replacement;
    }
    await _load();
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
      _conversations = <DmConversation>[];
      _selected = null;
      _counts = null;
      _pendingStartDraft = null;
      _pendingStartKey = null;
      _nextCursor = null;
      _hasMore = false;
      _failure = null;
      _isLoading = state.isAuthenticated;
      final int expectedGeneration = state.generation;
      WidgetsBinding.instance.addPostFrameCallback((Duration _) {
        if (!mounted ||
            ref.read(sessionStateProvider).value?.generation !=
                expectedGeneration) {
          return;
        }
        _searchController.clear();
        if (state.isAuthenticated) {
          unawaited(_load());
        }
      });
    }
    if (state == null || state.phase == SessionPhase.restoring) {
      return const AppLoadingState(title: '正在恢复私信账号');
    }
    if (!state.isAuthenticated) {
      return AppEmptyState(
        title: '登录后查看私信',
        description: '私信正文不会写入普通本地缓存；登录后只从当前账号的服务器事实读取。',
        action: FilledButton.icon(
          onPressed: () => context.push(AppRoutes.login),
          icon: const Icon(Icons.login_rounded),
          label: const Text('登录'),
        ),
      );
    }
    if (_isLoading && _conversations.isEmpty) {
      return const AppLoadingState(title: '正在加载私信');
    }
    final ApiFailure? failure = _failure;
    if (failure != null && _conversations.isEmpty) {
      if (failure.kind == ApiFailureKind.forbidden) {
        return const AppPermissionState(description: '当前账号状态不允许访问私信。');
      }
      return AppErrorState(description: failure.message, onRetry: _load);
    }
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final bool split = constraints.maxWidth >= 840;
        if (!split && _selected != null) {
          return _ConversationPane(
            key: ValueKey<String>(_selected!.id),
            conversation: _selected!,
            currentAccountId: state.account!.id,
            repository: _repository,
            showBack: true,
            onBack: () => setState(() => _selected = null),
            onChanged: _conversationChanged,
          );
        }
        final Widget list = _conversationList();
        if (!split) {
          return list;
        }
        return Row(
          children: <Widget>[
            SizedBox(width: 360, child: list),
            const VerticalDivider(width: 1),
            Expanded(
              child: _selected == null
                  ? const AppEmptyState(
                      title: '选择一个对话',
                      description: '平板和宽屏会在右侧显示消息记录。',
                    )
                  : _ConversationPane(
                      key: ValueKey<String>(_selected!.id),
                      conversation: _selected!,
                      currentAccountId: state.account!.id,
                      repository: _repository,
                      showBack: false,
                      onChanged: _conversationChanged,
                    ),
            ),
          ],
        );
      },
    );
  }

  Widget _conversationList() {
    return RefreshIndicator(
      onRefresh: _load,
      child: CustomScrollView(
        key: const PageStorageKey<String>('dm-conversation-list'),
        slivers: <Widget>[
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
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
                              '私信',
                              style: Theme.of(context).textTheme.headlineSmall,
                            ),
                            Text(
                              '未读 ${_counts?.unreadCount ?? 0} · 请求 ${_counts?.requestCount ?? 0}',
                            ),
                          ],
                        ),
                      ),
                      FilledButton.icon(
                        onPressed: _isMutating ? null : _startConversation,
                        icon: const Icon(Icons.edit_outlined),
                        label: const Text('新私信'),
                      ),
                    ],
                  ),
                  const SizedBox(height: 12),
                  SearchBar(
                    controller: _searchController,
                    hintText: '搜索公开用户名或消息摘要',
                    leading: const Icon(Icons.search_rounded),
                    trailing: <Widget>[
                      if (_searchController.text.isNotEmpty)
                        IconButton(
                          tooltip: '清空搜索',
                          onPressed: () {
                            _searchController.clear();
                            setState(() {});
                            unawaited(_load());
                          },
                          icon: const Icon(Icons.clear_rounded),
                        ),
                    ],
                    onChanged: (_) => setState(() {}),
                    onSubmitted: (_) => _load(),
                  ),
                  const SizedBox(height: 10),
                  SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: Row(
                      children: ConversationView.values
                          .map(
                            (ConversationView view) => Padding(
                              padding: const EdgeInsets.only(right: 8),
                              child: ChoiceChip(
                                selected: _view == view,
                                onSelected: (_) => _selectView(view),
                                label: Text(_viewLabel(view, _counts)),
                              ),
                            ),
                          )
                          .toList(growable: false),
                    ),
                  ),
                  if (_missingDeepLink) ...<Widget>[
                    const SizedBox(height: 10),
                    const Card(
                      child: ListTile(
                        leading: Icon(Icons.search_off_rounded),
                        title: Text('未在前几页找到通知对应的对话'),
                        subtitle: Text('它可能已归档、删除，或超出当前筛选；切换列表并继续加载即可查找。'),
                      ),
                    ),
                  ],
                  if (_pendingStartDraft != null) ...<Widget>[
                    const SizedBox(height: 10),
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(12),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: <Widget>[
                            Row(
                              children: <Widget>[
                                const Icon(Icons.sync_problem_rounded),
                                const SizedBox(width: 10),
                                Expanded(
                                  child: Text(
                                    '无法确认发给 @${_pendingStartDraft!.handle} 的请求结果',
                                    style: Theme.of(
                                      context,
                                    ).textTheme.titleSmall,
                                  ),
                                ),
                              ],
                            ),
                            const SizedBox(height: 6),
                            const Text('重试会复用原幂等键；取消只清除本机待确认状态。'),
                            const SizedBox(height: 8),
                            Wrap(
                              spacing: 8,
                              alignment: WrapAlignment.end,
                              children: <Widget>[
                                TextButton(
                                  onPressed: _isMutating
                                      ? null
                                      : () => setState(() {
                                          _pendingStartDraft = null;
                                          _pendingStartKey = null;
                                        }),
                                  child: const Text('取消'),
                                ),
                                FilledButton.tonal(
                                  onPressed: _isMutating
                                      ? null
                                      : () => _startConversation(retry: true),
                                  child: const Text('按原请求重试'),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                    ),
                  ],
                  if (_failure != null &&
                      _conversations.isNotEmpty) ...<Widget>[
                    const SizedBox(height: 8),
                    Text(
                      _failure!.message,
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.error,
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
          if (_conversations.isEmpty)
            SliverFillRemaining(
              hasScrollBody: false,
              child: AppEmptyState(
                title: _searchController.text.trim().length >= 2
                    ? '没有匹配的对话'
                    : '这里还没有对话',
                description: _view == ConversationView.requests
                    ? '陌生联系请求会单独出现，接受前只能有一条附言。'
                    : '可以通过公开用户名发起私信。',
              ),
            )
          else
            SliverList.separated(
              itemCount: _conversations.length,
              separatorBuilder: (_, _) => const Divider(height: 1),
              itemBuilder: (BuildContext context, int index) {
                final DmConversation conversation = _conversations[index];
                return _ConversationTile(
                  conversation: conversation,
                  selected: _selected?.id == conversation.id,
                  onRefreshAvatar: () => unawaited(_load()),
                  onTap: () => setState(() => _selected = conversation),
                );
              },
            ),
          if (_hasMore)
            SliverToBoxAdapter(
              child: Padding(
                padding: const EdgeInsets.all(16),
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
}

class _ConversationTile extends StatelessWidget {
  const _ConversationTile({
    required this.conversation,
    required this.selected,
    required this.onRefreshAvatar,
    required this.onTap,
  });

  final DmConversation conversation;
  final bool selected;
  final VoidCallback onRefreshAvatar;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final String name =
        conversation.participantDisplayName?.trim().isNotEmpty == true
        ? conversation.participantDisplayName!
        : '@${conversation.participantHandle}';
    return ListTile(
      selected: selected,
      leading: PlatformAvatar(
        compatibilityUrl: conversation.participantAvatarUrl,
        fallbackText: conversation.participantHandle,
        semanticLabel: '${conversation.participantHandle} 的头像',
        onRefresh: onRefreshAvatar,
      ),
      title: Row(
        children: <Widget>[
          Expanded(
            child: Text(name, maxLines: 1, overflow: TextOverflow.ellipsis),
          ),
          if (conversation.isMuted)
            const Icon(Icons.notifications_off_outlined, size: 18),
        ],
      ),
      subtitle: Text(
        conversation.lastMessageExcerpt ?? _requestStateLabel(conversation),
        maxLines: 2,
        overflow: TextOverflow.ellipsis,
      ),
      trailing: conversation.unreadCount > 0
          ? Badge(label: Text('${conversation.unreadCount}'))
          : const Icon(Icons.chevron_right_rounded),
      onTap: onTap,
    );
  }
}

class _ConversationPane extends StatefulWidget {
  const _ConversationPane({
    required this.conversation,
    required this.currentAccountId,
    required this.repository,
    required this.showBack,
    required this.onChanged,
    this.onBack,
    super.key,
  });

  final DmConversation conversation;
  final String currentAccountId;
  final MessagesRepository repository;
  final bool showBack;
  final VoidCallback? onBack;
  final Future<void> Function({
    DmConversation? replacement,
    ConversationView? view,
  })
  onChanged;

  @override
  State<_ConversationPane> createState() => _ConversationPaneState();
}

class _ConversationPaneState extends State<_ConversationPane> {
  final TextEditingController _bodyController = TextEditingController();
  List<DmMessage> _messages = <DmMessage>[];
  String? _nextCursor;
  bool _hasMore = false;
  bool _isLoading = true;
  bool _isLoadingMore = false;
  bool _isMutating = false;
  ApiFailure? _failure;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _bodyController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _isLoading = true;
      _failure = null;
    });
    try {
      final DmMessagePage page = await widget.repository.messages(
        widget.conversation.id,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _messages = page.items;
        _nextCursor = page.nextCursor;
        _hasMore = page.hasMore;
      });
      final String? latestId = page.items.firstOrNull?.id;
      if (widget.conversation.unreadCount > 0) {
        await widget.repository.markRead(widget.conversation.id, latestId);
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

  Future<void> _loadMore() async {
    final String? cursor = _nextCursor;
    if (_isLoadingMore || !_hasMore || cursor == null) {
      return;
    }
    setState(() => _isLoadingMore = true);
    try {
      final DmMessagePage page = await widget.repository.messages(
        widget.conversation.id,
        cursor: cursor,
      );
      if (mounted) {
        final Set<String> ids = _messages
            .map((DmMessage item) => item.id)
            .toSet();
        setState(() {
          _messages.addAll(
            page.items.where((DmMessage item) => ids.add(item.id)),
          );
          _nextCursor = page.nextCursor;
          _hasMore = page.hasMore;
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

  Future<void> _send() async {
    final String body = _bodyController.text.trim();
    if (_isMutating || body.isEmpty || body.length > 16000) {
      return;
    }
    setState(() => _isMutating = true);
    try {
      await widget.repository.send(widget.conversation.id, body);
      if (mounted) {
        _bodyController.clear();
        await _load();
        await widget.onChanged();
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() {
          _failure = ApiFailure(
            kind: failure.kind,
            message: '${failure.message}；不会自动重发，请先刷新确认服务器是否已收到',
            code: failure.code,
            statusCode: failure.statusCode,
            retryAfter: failure.retryAfter,
          );
        });
        try {
          final DmMessagePage canonical = await widget.repository.messages(
            widget.conversation.id,
          );
          if (mounted) {
            setState(() {
              _messages = canonical.items;
              _nextCursor = canonical.nextCursor;
              _hasMore = canonical.hasMore;
            });
          }
        } on ApiFailure {
          // The original failure remains actionable; no send is replayed.
        }
      }
    } finally {
      if (mounted) {
        setState(() => _isMutating = false);
      }
    }
  }

  Future<void> _accept() async {
    await _runMutation(() async {
      final DmConversation accepted = await widget.repository.accept(
        widget.conversation.id,
      );
      await widget.onChanged(
        replacement: accepted,
        view: ConversationView.inbox,
      );
    }, '请求已接受');
  }

  Future<void> _declineOrWithdraw() async {
    final bool incoming =
        widget.conversation.requestDirection ==
        DmConversationRequestDirectionEnum.incoming;
    final bool? confirmed = await showDialog<bool>(
      context: context,
      builder: (BuildContext context) => AlertDialog(
        title: Text(incoming ? '拒绝消息请求？' : '撤回消息请求？'),
        content: Text(
          incoming ? '未举报的附言会立即删除；拒绝不会自动屏蔽或通知对方。' : '未举报的附言会立即删除；撤回不会通知对方。',
        ),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(incoming ? '确认拒绝' : '确认撤回'),
          ),
        ],
      ),
    );
    if (confirmed != true) {
      return;
    }
    await _runMutation(() async {
      await widget.repository.declineOrWithdraw(widget.conversation.id);
      await widget.onChanged();
      widget.onBack?.call();
    }, incoming ? '请求已拒绝' : '请求已撤回');
  }

  Future<void> _reportRequest() async {
    final DmReportDraft? report = await showDmReportDialog(
      context,
      isRequest: true,
    );
    if (report == null) {
      return;
    }
    await _runMutation(() async {
      await widget.repository.reportRequest(
        requestId: widget.conversation.id,
        reason: report.reason,
        note: report.note,
      );
      await widget.onChanged();
      widget.onBack?.call();
    }, '举报已提交，请求已移出收件箱');
  }

  Future<void> _reportMessage(DmMessage message) async {
    final DmReportDraft? report = await showDmReportDialog(
      context,
      isRequest: false,
    );
    if (report == null) {
      return;
    }
    await _runMutation(() async {
      await widget.repository.reportMessage(
        messageId: message.id,
        reason: report.reason,
        note: report.note,
      );
    }, '举报已提交；审核人员只会看到必要证据');
  }

  Future<void> _menuAction(_ConversationAction action) async {
    switch (action) {
      case _ConversationAction.mute:
        await _runMutation(
          () => widget.repository.mute(widget.conversation.id),
          '已静音；未读计数仍保持准确',
        );
      case _ConversationAction.unmute:
        await _runMutation(
          () => widget.repository.unmute(widget.conversation.id),
          '已恢复私信通知',
        );
      case _ConversationAction.archive:
        await _runMutation(() async {
          await widget.repository.archive(widget.conversation.id);
          await widget.onChanged(view: ConversationView.archived);
        }, '对话已归档');
      case _ConversationAction.unarchive:
        await _runMutation(() async {
          await widget.repository.unarchive(widget.conversation.id);
          await widget.onChanged(view: ConversationView.inbox);
        }, '对话已回到收件箱');
      case _ConversationAction.delete:
        final bool confirmed = await _confirmDelete();
        if (!confirmed) {
          return;
        }
        await _runMutation(() async {
          await widget.repository.delete(widget.conversation.id);
          await widget.onChanged(view: ConversationView.deleted);
        }, '对话已从本方收件箱隐藏，可在“已删除”恢复');
      case _ConversationAction.recover:
        await _runMutation(() async {
          await widget.repository.recover(widget.conversation.id);
          await widget.onChanged(view: ConversationView.inbox);
        }, '对话已恢复');
    }
  }

  Future<bool> _confirmDelete() async {
    return await showDialog<bool>(
          context: context,
          builder: (BuildContext context) => AlertDialog(
            title: const Text('从本方收件箱隐藏对话？'),
            content: const Text('这不会立即删除对方副本；你可以在“已删除”列表中恢复。新消息也会让对话重新出现。'),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(false),
                child: const Text('取消'),
              ),
              FilledButton(
                onPressed: () => Navigator.of(context).pop(true),
                child: const Text('确认隐藏'),
              ),
            ],
          ),
        ) ??
        false;
  }

  Future<void> _runMutation(
    Future<void> Function() operation,
    String successMessage,
  ) async {
    if (_isMutating) {
      return;
    }
    setState(() {
      _isMutating = true;
      _failure = null;
    });
    try {
      await operation();
      _showMessage(successMessage);
      if (mounted) {
        await widget.onChanged();
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
    final DmConversation conversation = widget.conversation;
    final bool pending =
        conversation.requestStatus == DmConversationRequestStatusEnum.pending;
    final bool incoming =
        conversation.requestDirection ==
        DmConversationRequestDirectionEnum.incoming;
    final List<DmMessage> chronological = _messages.reversed.toList(
      growable: false,
    );
    return Column(
      children: <Widget>[
        Material(
          color: Theme.of(context).colorScheme.surface,
          child: ListTile(
            leading: widget.showBack
                ? IconButton(
                    tooltip: '返回对话列表',
                    onPressed: widget.onBack,
                    icon: const Icon(Icons.arrow_back_rounded),
                  )
                : null,
            title: Row(
              children: <Widget>[
                PlatformAvatar(
                  radius: 17,
                  compatibilityUrl: conversation.participantAvatarUrl,
                  fallbackText: conversation.participantHandle,
                  semanticLabel: '${conversation.participantHandle} 的头像',
                  onRefresh: () => unawaited(widget.onChanged()),
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    conversation.participantDisplayName ??
                        '@${conversation.participantHandle}',
                  ),
                ),
              ],
            ),
            subtitle: Text(
              pending
                  ? incoming
                        ? '收到的消息请求'
                        : '等待对方接受'
                  : conversation.isMuted
                  ? '已静音；未读仍正常计数'
                  : '@${conversation.participantHandle}',
            ),
            trailing: pending
                ? null
                : PopupMenuButton<_ConversationAction>(
                    tooltip: '对话操作',
                    enabled: !_isMutating,
                    onSelected: _menuAction,
                    itemBuilder: (BuildContext context) =>
                        _conversationActions(conversation)
                            .map(
                              (_ConversationAction action) =>
                                  PopupMenuItem<_ConversationAction>(
                                    value: action,
                                    child: Text(action.label),
                                  ),
                            )
                            .toList(growable: false),
                  ),
          ),
        ),
        const Divider(height: 1),
        if (pending)
          Padding(
            padding: const EdgeInsets.all(12),
            child: Card(
              child: Padding(
                padding: const EdgeInsets.all(14),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    Text(
                      conversation.lastMessageExcerpt ?? '请求附言不可用',
                      style: Theme.of(context).textTheme.bodyLarge,
                    ),
                    const SizedBox(height: 10),
                    if (incoming)
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        children: <Widget>[
                          FilledButton.icon(
                            onPressed: _isMutating ? null : _accept,
                            icon: const Icon(Icons.check_rounded),
                            label: const Text('接受'),
                          ),
                          OutlinedButton(
                            onPressed: _isMutating ? null : _declineOrWithdraw,
                            child: const Text('拒绝'),
                          ),
                          TextButton.icon(
                            onPressed: _isMutating ? null : _reportRequest,
                            icon: const Icon(Icons.flag_outlined),
                            label: const Text('举报'),
                          ),
                        ],
                      )
                    else
                      OutlinedButton.icon(
                        onPressed: _isMutating ? null : _declineOrWithdraw,
                        icon: const Icon(Icons.undo_rounded),
                        label: const Text('撤回请求'),
                      ),
                  ],
                ),
              ),
            ),
          ),
        Expanded(
          child: _isLoading && _messages.isEmpty
              ? const AppLoadingState(title: '正在加载消息')
              : _failure != null && _messages.isEmpty
              ? AppErrorState(description: _failure!.message, onRetry: _load)
              : chronological.isEmpty
              ? const AppEmptyState(
                  title: '还没有普通消息',
                  description: '接受请求后，双方才能继续发送消息。',
                )
              : RefreshIndicator(
                  onRefresh: _load,
                  child: ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: chronological.length + (_hasMore ? 1 : 0),
                    itemBuilder: (BuildContext context, int index) {
                      if (_hasMore && index == 0) {
                        return Center(
                          child: TextButton(
                            onPressed: _isLoadingMore ? null : _loadMore,
                            child: Text(_isLoadingMore ? '加载中' : '加载更早消息'),
                          ),
                        );
                      }
                      final int messageIndex = index - (_hasMore ? 1 : 0);
                      final DmMessage message = chronological[messageIndex];
                      return _MessageBubble(
                        message: message,
                        isMine: message.senderId == widget.currentAccountId,
                        onReport: () => _reportMessage(message),
                      );
                    },
                  ),
                ),
        ),
        if (!pending)
          SafeArea(
            top: false,
            child: Padding(
              padding: const EdgeInsets.fromLTRB(12, 8, 12, 12),
              child: Column(
                children: <Widget>[
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.end,
                    children: <Widget>[
                      Expanded(
                        child: TextField(
                          controller: _bodyController,
                          enabled: conversation.canSend && !_isMutating,
                          minLines: 1,
                          maxLines: 5,
                          maxLength: 16000,
                          decoration: InputDecoration(
                            hintText: conversation.canSend
                                ? '输入消息'
                                : '当前账号或关系状态不允许发送',
                            counterText: '',
                          ),
                          onSubmitted: (_) => _send(),
                        ),
                      ),
                      const SizedBox(width: 8),
                      IconButton.filled(
                        tooltip: '发送消息',
                        onPressed: conversation.canSend && !_isMutating
                            ? _send
                            : null,
                        icon: const Icon(Icons.send_rounded),
                      ),
                    ],
                  ),
                  if (_failure != null) ...<Widget>[
                    const SizedBox(height: 4),
                    Align(
                      alignment: Alignment.centerLeft,
                      child: Text(
                        _failure!.message,
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.error,
                        ),
                      ),
                    ),
                  ],
                  const Align(
                    alignment: Alignment.centerLeft,
                    child: Text(
                      '请勿发送校园身份、联系方式等不必要的敏感信息；当前私信不是端到端加密。',
                      style: TextStyle(fontSize: 12),
                    ),
                  ),
                ],
              ),
            ),
          ),
      ],
    );
  }
}

class _MessageBubble extends StatelessWidget {
  const _MessageBubble({
    required this.message,
    required this.isMine,
    required this.onReport,
  });

  final DmMessage message;
  final bool isMine;
  final VoidCallback onReport;

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: isMine ? Alignment.centerRight : Alignment.centerLeft,
      child: Padding(
        padding: const EdgeInsets.only(bottom: 10),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: Semantics(
            label:
                '${isMine ? '你' : message.senderDisplayName ?? message.senderHandle}在${_formatUnix(message.createdAt)}发送：${message.body}',
            child: ExcludeSemantics(
              child: Card(
                color: isMine
                    ? Theme.of(context).colorScheme.primaryContainer
                    : null,
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(14, 10, 6, 8),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Flexible(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: <Widget>[
                            Text(message.body),
                            const SizedBox(height: 4),
                            Text(
                              _formatUnix(message.createdAt),
                              style: Theme.of(context).textTheme.bodySmall,
                            ),
                          ],
                        ),
                      ),
                      if (!isMine)
                        PopupMenuButton<String>(
                          tooltip: '消息操作',
                          onSelected: (_) => onReport(),
                          itemBuilder: (_) => const <PopupMenuEntry<String>>[
                            PopupMenuItem<String>(
                              value: 'report',
                              child: Text('举报此消息'),
                            ),
                          ],
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

enum _ConversationAction {
  mute('静音通知'),
  unmute('恢复通知'),
  archive('归档'),
  unarchive('移回收件箱'),
  delete('从本方隐藏'),
  recover('恢复对话');

  const _ConversationAction(this.label);

  final String label;
}

List<_ConversationAction> _conversationActions(
  DmConversation conversation,
) => <_ConversationAction>[
  conversation.isMuted ? _ConversationAction.unmute : _ConversationAction.mute,
  conversation.isArchived
      ? _ConversationAction.unarchive
      : _ConversationAction.archive,
  conversation.isDeleted
      ? _ConversationAction.recover
      : _ConversationAction.delete,
];

String _viewLabel(ConversationView view, DmCounts? counts) => switch (view) {
  ConversationView.requests => '${view.label} ${counts?.requestCount ?? 0}',
  _ => view.label,
};

String _requestStateLabel(DmConversation conversation) =>
    conversation.requestStatus == DmConversationRequestStatusEnum.pending
    ? conversation.requestDirection ==
              DmConversationRequestDirectionEnum.incoming
          ? '收到的消息请求'
          : '等待对方接受'
    : '暂无消息';

DmConversation? _findById(List<DmConversation> items, String id) {
  for (final DmConversation item in items) {
    if (item.id == id) {
      return item;
    }
  }
  return null;
}

List<DmConversation> _deduplicate(List<DmConversation> items) {
  final Set<String> ids = <String>{};
  return items.where((DmConversation item) => ids.add(item.id)).toList();
}

String _formatUnix(int seconds) {
  final DateTime value = DateTime.fromMillisecondsSinceEpoch(
    seconds * 1000,
    isUtc: true,
  ).toLocal();
  String two(int number) => number.toString().padLeft(2, '0');
  return '${value.year}-${two(value.month)}-${two(value.day)} '
      '${two(value.hour)}:${two(value.minute)}';
}
