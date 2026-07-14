import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../../core/widgets/platform_avatar.dart';
import '../../auth/domain/session_state.dart';
import '../../media/data/media_uploader.dart';
import '../../media/presentation/media_upload_button.dart';
import '../data/forum_repository.dart';
import 'forum_markdown_composer.dart';
import 'forum_widgets.dart';
import 'revision_history_sheet.dart';

class ThreadDetailPage extends ConsumerStatefulWidget {
  const ThreadDetailPage({required this.threadId, super.key});

  final String threadId;

  @override
  ConsumerState<ThreadDetailPage> createState() => _ThreadDetailPageState();
}

class _ThreadDetailPageState extends ConsumerState<ThreadDetailPage> {
  ThreadDetail? _thread;
  List<Board> _boards = <Board>[];
  List<Comment> _comments = <Comment>[];
  String? _nextCursor;
  bool _hasMoreComments = false;
  bool _isLoading = true;
  bool _isLoadingMore = false;
  final Set<String> _pendingActions = <String>{};
  ApiFailure? _error;
  int _generation = 0;
  (int, SessionPhase, String?)? _sessionIdentity;

  ForumRepository get _repository => ref.read(forumRepositoryProvider);

  bool get _isAuthenticated =>
      ref.read(sessionStateProvider).value?.isAuthenticated ?? false;

  @override
  void initState() {
    super.initState();
    final SessionState? session = ref.read(sessionStateProvider).value;
    if (session != null) {
      _sessionIdentity = (
        session.generation,
        session.phase,
        session.account?.id,
      );
    }
    ref.listenManual<AsyncValue<SessionState>>(
      sessionStateProvider,
      _handleSessionState,
    );
    unawaited(_load());
  }

  void _handleSessionState(
    AsyncValue<SessionState>? _,
    AsyncValue<SessionState> next,
  ) {
    final SessionState? session = next.value;
    if (session == null) {
      return;
    }
    final (int, SessionPhase, String?) identity = (
      session.generation,
      session.phase,
      session.account?.id,
    );
    if (_sessionIdentity == identity) {
      return;
    }
    _sessionIdentity = identity;
    unawaited(_load());
  }

  @override
  void didUpdateWidget(ThreadDetailPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.threadId != widget.threadId) {
      _pendingActions.clear();
      unawaited(_load());
    }
  }

  Future<void> _load() async {
    final int generation = ++_generation;
    final String threadId = widget.threadId;
    setState(() {
      _isLoading = true;
      _error = null;
    });
    try {
      final List<Object> results = await Future.wait<Object>(<Future<Object>>[
        _repository.thread(threadId),
        _repository.boards(),
        _repository.comments(threadId),
      ]);
      if (!mounted ||
          generation != _generation ||
          threadId != widget.threadId) {
        return;
      }
      final ForumPageSlice<Comment> commentPage =
          results[2] as ForumPageSlice<Comment>;
      setState(() {
        _thread = results[0] as ThreadDetail;
        _boards = results[1] as List<Board>;
        _comments = commentPage.items;
        _nextCursor = commentPage.nextCursor;
        _hasMoreComments = commentPage.hasMore;
      });
      if (_isAuthenticated) {
        unawaited(_repository.markRead(threadId));
      }
    } on ApiFailure catch (failure) {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() => _error = failure);
      }
    } finally {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _refreshThread() async {
    final int generation = _generation;
    final String threadId = widget.threadId;
    try {
      final ThreadDetail thread = await _repository.thread(threadId);
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() => _thread = thread);
      }
    } on ApiFailure catch (failure) {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        _showMessage(failure.message);
      }
    }
  }

  Future<void> _refreshComments() async {
    final int generation = _generation;
    final String threadId = widget.threadId;
    try {
      final ForumPageSlice<Comment> page = await _repository.comments(threadId);
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() {
          _comments = page.items;
          _nextCursor = page.nextCursor;
          _hasMoreComments = page.hasMore;
        });
      }
    } on ApiFailure catch (failure) {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        _showMessage(failure.message);
      }
    }
  }

  Future<void> _loadMoreComments() async {
    if (_isLoadingMore || !_hasMoreComments || _nextCursor == null) {
      return;
    }
    final int generation = _generation;
    final String threadId = widget.threadId;
    final String cursor = _nextCursor!;
    setState(() => _isLoadingMore = true);
    try {
      final ForumPageSlice<Comment> page = await _repository.comments(
        threadId,
        cursor: cursor,
      );
      if (mounted && generation == _generation && threadId == widget.threadId) {
        final Set<String> known = _comments
            .map((Comment comment) => comment.id)
            .toSet();
        setState(() {
          _comments.addAll(
            page.items.where((Comment comment) => known.add(comment.id)),
          );
          _nextCursor = page.nextCursor;
          _hasMoreComments = page.hasMore;
        });
      }
    } on ApiFailure catch (failure) {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        _showMessage(failure.message);
      }
    } finally {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() => _isLoadingMore = false);
      }
    }
  }

  Future<bool> _requireAuthentication() async {
    if (_isAuthenticated) {
      return true;
    }
    await context.push(
      publicInteractionLoginLocation(GoRouterState.of(context).uri),
    );
    return mounted && _isAuthenticated;
  }

  Future<void> _runAction({
    required String key,
    required Future<void> Function() operation,
    required String successMessage,
    bool refreshThread = true,
    bool refreshComments = false,
  }) async {
    if (_pendingActions.contains(key) || !await _requireAuthentication()) {
      return;
    }
    setState(() => _pendingActions.add(key));
    try {
      await operation();
      if (!mounted) {
        return;
      }
      _showMessage(successMessage);
      await Future.wait(<Future<void>>[
        if (refreshThread) _refreshThread(),
        if (refreshComments) _refreshComments(),
      ]);
    } on ApiFailure catch (failure) {
      if (mounted) {
        _showMessage(failure.message);
      }
    } finally {
      if (mounted) {
        setState(() => _pendingActions.remove(key));
      }
    }
  }

  Future<void> _voteThread(String value) async {
    final ThreadDetail? thread = _thread;
    if (thread == null) {
      return;
    }
    await _runAction(
      key: 'thread-vote',
      successMessage: '投票已更新',
      operation: () => _repository.vote(
        id: thread.id,
        postType: 'thread',
        value: value,
        remove: thread.viewerVote?.value == value,
      ),
    );
  }

  Future<void> _bookmarkThread() async {
    final ThreadDetail? thread = _thread;
    if (thread == null) {
      return;
    }
    await _runAction(
      key: 'thread-bookmark',
      successMessage: thread.isBookmarked ? '已取消收藏' : '已收藏',
      operation: () => _repository.setBookmark(
        id: thread.id,
        postType: 'thread',
        bookmarked: thread.isBookmarked,
      ),
    );
  }

  Future<void> _subscribe(String level) async {
    await _runAction(
      key: 'thread-subscribe',
      successMessage: '订阅已更新',
      operation: () => _repository.setThreadSubscription(
        threadId: widget.threadId,
        level: level,
      ),
    );
  }

  Future<void> _reportPost(String id, String postType) async {
    if (!await _requireAuthentication() || !mounted) {
      return;
    }
    final _ReportDraft? draft = await showDialog<_ReportDraft>(
      context: context,
      builder: (BuildContext context) => const _ReportDialog(),
    );
    if (draft == null) {
      return;
    }
    await _runAction(
      key: 'report-$postType-$id',
      successMessage: '举报已提交',
      refreshThread: false,
      operation: () => _repository.report(
        id: id,
        postType: postType,
        reason: draft.reason,
        note: draft.note,
      ),
    );
  }

  Future<void> _editThread() async {
    final ThreadDetail? thread = _thread;
    if (thread == null || !thread.canEdit || !await _requireAuthentication()) {
      return;
    }
    if (!mounted) {
      return;
    }
    final bool? updated = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (BuildContext context) =>
          _EditThreadDialog(thread: thread, repository: _repository),
    );
    if (updated == true) {
      await _refreshThread();
    }
  }

  Future<void> _showThreadRevisions() async {
    final ThreadDetail? thread = _thread;
    if (thread == null || !await _requireAuthentication() || !mounted) {
      return;
    }
    await showForumRevisionHistorySheet(
      context: context,
      repository: _repository,
      target: ForumRevisionTarget.thread,
      targetId: thread.id,
    );
  }

  Future<void> _deleteThread() async {
    final ThreadDetail? thread = _thread;
    if (thread == null ||
        !thread.canDelete ||
        !await _requireAuthentication()) {
      return;
    }
    if (!mounted) {
      return;
    }
    final bool confirmed = await _confirm(
      title: '删除主题？',
      body: '主题会被软删除；此操作不会伪装成客户端本地删除。',
      confirmLabel: '删除',
    );
    if (!confirmed) {
      return;
    }
    try {
      await _repository.deleteThread(thread.id);
      if (mounted) {
        _showMessage('主题已删除');
        context.go(AppRoutes.forum);
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        _showMessage(failure.message);
      }
    }
  }

  Future<void> _editComment(Comment comment) async {
    if (!comment.canEdit || !await _requireAuthentication() || !mounted) {
      return;
    }
    final bool? updated = await showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (BuildContext context) =>
          _EditCommentDialog(comment: comment, repository: _repository),
    );
    if (updated == true) {
      await _refreshComments();
    }
  }

  Future<void> _showCommentRevisions(Comment comment) async {
    if (!await _requireAuthentication() || !mounted) {
      return;
    }
    await showForumRevisionHistorySheet(
      context: context,
      repository: _repository,
      target: ForumRevisionTarget.comment,
      targetId: comment.id,
    );
  }

  Future<void> _deleteComment(Comment comment) async {
    if (!comment.canDelete || !await _requireAuthentication() || !mounted) {
      return;
    }
    final bool confirmed = await _confirm(
      title: '删除回复？',
      body: '回复会被软删除，页面刷新后显示服务端结果。',
      confirmLabel: '删除',
    );
    if (!confirmed) {
      return;
    }
    await _runAction(
      key: 'delete-${comment.id}',
      successMessage: '回复已删除',
      refreshComments: true,
      operation: () => _repository.deleteComment(comment.id),
    );
  }

  Future<void> _voteComment(Comment comment, String value) async {
    await _runAction(
      key: 'vote-${comment.id}',
      successMessage: '投票已更新',
      refreshThread: false,
      refreshComments: true,
      operation: () => _repository.vote(
        id: comment.id,
        postType: 'comment',
        value: value,
        remove: comment.viewerVote?.value == value,
      ),
    );
  }

  Future<void> _bookmarkComment(Comment comment) async {
    await _runAction(
      key: 'bookmark-${comment.id}',
      successMessage: comment.isBookmarked ? '已取消收藏' : '已收藏',
      refreshThread: false,
      refreshComments: true,
      operation: () => _repository.setBookmark(
        id: comment.id,
        postType: 'comment',
        bookmarked: comment.isBookmarked,
      ),
    );
  }

  Future<void> _toggleSolved(Comment comment) async {
    await _runAction(
      key: 'solve-${comment.id}',
      successMessage: comment.isSolved ? '已取消采纳' : '已采纳为答案',
      refreshComments: true,
      operation: () => comment.isSolved
          ? _repository.unmarkSolved(comment.id)
          : _repository.markSolved(comment.id),
    );
  }

  Future<bool> _confirm({
    required String title,
    required String body,
    required String confirmLabel,
  }) async {
    return await showDialog<bool>(
          context: context,
          builder: (BuildContext context) => AlertDialog(
            title: Text(title),
            content: Text(body),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(false),
                child: const Text('取消'),
              ),
              FilledButton(
                onPressed: () => Navigator.of(context).pop(true),
                child: Text(confirmLabel),
              ),
            ],
          ),
        ) ??
        false;
  }

  void _showMessage(String message) {
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  String? _boardName(String id) {
    for (final Board board in _boards) {
      if (board.id == id) {
        return board.name;
      }
    }
    return null;
  }

  String? _replyUnavailableReason(ThreadDetail thread) {
    if (thread.deletedAt != null) {
      return '帖子已被软删除，恢复后才能继续回复。';
    }
    if (thread.hiddenAt != null) {
      return '帖子正在治理隐藏状态，恢复公开后才能继续回复。';
    }
    if (thread.archivedAt != null) {
      return '帖子已归档，不再接受新回复。';
    }
    if (thread.closedAt != null) {
      return '帖子已关闭，不再接受新回复。';
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final AsyncValue<SessionState> session = ref.watch(sessionStateProvider);
    final SessionState? sessionState = session.value;
    if (_isLoading) {
      return const Scaffold(body: AppLoadingState(title: '加载主题'));
    }
    if (_error case final ApiFailure failure) {
      return Scaffold(
        appBar: AppBar(),
        body: failure.kind == ApiFailureKind.forbidden
            ? AppPermissionState(description: failure.message)
            : AppErrorState(description: failure.message, onRetry: _load),
      );
    }
    final ThreadDetail? thread = _thread;
    if (thread == null) {
      return const Scaffold(body: AppEmptyState(title: '主题不存在或已不可见'));
    }
    final String? replyUnavailable = _replyUnavailableReason(thread);
    final String? currentAccountId = sessionState?.account?.id;
    final bool canManageSolution =
        currentAccountId == thread.authorId || thread.canModerate;
    final bool canReadThreadHistory =
        sessionState?.isAuthenticated == true &&
        (currentAccountId == thread.authorId || thread.canModerate);
    return Scaffold(
      appBar: AppBar(
        title: Text(_boardName(thread.boardId) ?? '主题详情'),
        actions: <Widget>[
          if (thread.canEdit)
            IconButton(
              tooltip: '编辑主题',
              onPressed: _editThread,
              icon: const Icon(Icons.edit_outlined),
            ),
          if (thread.canDelete)
            IconButton(
              tooltip: '删除主题',
              onPressed: _deleteThread,
              icon: const Icon(Icons.delete_outline_rounded),
            ),
          PopupMenuButton<String>(
            tooltip: '更多主题操作',
            onSelected: (String value) {
              switch (value) {
                case 'history':
                  unawaited(_showThreadRevisions());
                case 'report':
                  unawaited(_reportPost(thread.id, 'thread'));
                default:
                  unawaited(_subscribe(value));
              }
            },
            itemBuilder: (BuildContext context) => <PopupMenuEntry<String>>[
              const PopupMenuItem<String>(value: 'none', child: Text('不订阅')),
              const PopupMenuItem<String>(
                value: 'watching',
                child: Text('订阅通知'),
              ),
              const PopupMenuItem<String>(value: 'tracking', child: Text('跟踪')),
              const PopupMenuItem<String>(value: 'muted', child: Text('静音')),
              if (canReadThreadHistory)
                const PopupMenuItem<String>(
                  value: 'history',
                  child: Text('修订历史'),
                ),
              const PopupMenuDivider(),
              const PopupMenuItem<String>(value: 'report', child: Text('举报主题')),
            ],
          ),
        ],
      ),
      body: RefreshIndicator(
        onRefresh: _load,
        child: ListView(
          physics: const AlwaysScrollableScrollPhysics(),
          padding: const EdgeInsets.all(16),
          children: <Widget>[
            Card(
              child: Padding(
                padding: const EdgeInsets.all(18),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      thread.title,
                      style: Theme.of(context).textTheme.headlineSmall
                          ?.copyWith(fontWeight: FontWeight.w700),
                    ),
                    const SizedBox(height: 12),
                    Wrap(
                      spacing: 8,
                      runSpacing: 6,
                      children: <Widget>[
                        PlatformAvatar(
                          radius: 17,
                          delivery: thread.authorAvatar,
                          fallbackText: thread.authorHandle,
                          semanticLabel: '${thread.authorHandle} 的头像',
                          onRefresh: _refreshThread,
                        ),
                        Text(
                          thread.authorDisplayName ?? '@${thread.authorHandle}',
                          style: Theme.of(context).textTheme.labelLarge,
                        ),
                        if (thread.authorDisplayName != null)
                          Text('@${thread.authorHandle}'),
                        Text(formatForumTime(thread.createdAt)),
                        Chip(label: Text('${thread.replyCount} 回复')),
                        Chip(label: Text('${thread.voteCount} 票')),
                        if (thread.pinnedAt != null)
                          const Chip(label: Text('置顶')),
                        if (thread.closedAt != null)
                          const Chip(label: Text('已关闭')),
                        if (thread.archivedAt != null)
                          const Chip(label: Text('已归档')),
                        if (thread.hiddenAt != null)
                          const Chip(label: Text('已隐藏')),
                        if (thread.deletedAt != null)
                          const Chip(label: Text('已删除')),
                        ...thread.tags.map(
                          (String tag) => Chip(label: Text('#$tag')),
                        ),
                      ],
                    ),
                    const Divider(height: 28),
                    if (thread.body case final String body)
                      ForumBody(
                        source: body,
                        format: thread.contentFormat,
                        attachments: thread.attachments,
                        onRefreshDelivery: _refreshThread,
                      )
                    else
                      const Text('这条主题没有正文。'),
                    if (thread.attachments.isNotEmpty) ...<Widget>[
                      const SizedBox(height: 14),
                      ...thread.attachments.map(
                        (ForumAttachment attachment) => Padding(
                          padding: const EdgeInsets.only(bottom: 10),
                          child: ForumAttachmentImage(
                            attachment: attachment,
                            onRefreshDelivery: _refreshThread,
                          ),
                        ),
                      ),
                    ],
                    const SizedBox(height: 16),
                    Wrap(
                      spacing: 8,
                      runSpacing: 8,
                      children: <Widget>[
                        FilledButton.tonalIcon(
                          onPressed: _pendingActions.contains('thread-vote')
                              ? null
                              : () => _voteThread('up'),
                          icon: const Icon(Icons.thumb_up_outlined),
                          label: Text(
                            thread.viewerVote?.value == 'up' ? '已顶' : '顶',
                          ),
                        ),
                        FilledButton.tonalIcon(
                          onPressed: _pendingActions.contains('thread-vote')
                              ? null
                              : () => _voteThread('down'),
                          icon: const Icon(Icons.thumb_down_outlined),
                          label: Text(
                            thread.viewerVote?.value == 'down' ? '已踩' : '踩',
                          ),
                        ),
                        OutlinedButton.icon(
                          onPressed: _pendingActions.contains('thread-bookmark')
                              ? null
                              : _bookmarkThread,
                          icon: Icon(
                            thread.isBookmarked
                                ? Icons.bookmark_rounded
                                : Icons.bookmark_outline_rounded,
                          ),
                          label: Text(thread.isBookmarked ? '已收藏' : '收藏'),
                        ),
                        Chip(
                          avatar: const Icon(Icons.notifications_outlined),
                          label: Text(
                            '订阅：${_subscriptionLabel(thread.mySubscriptionLevel?.value)}',
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
            if (thread.poll case final Poll poll) ...<Widget>[
              const SizedBox(height: 12),
              _PollCard(
                poll: poll,
                isAuthenticated: sessionState?.isAuthenticated ?? false,
                isPending: _pendingActions.contains('poll'),
                onRequireAuthentication: _requireAuthentication,
                onVote: (String optionId, bool remove) => _runAction(
                  key: 'poll',
                  successMessage: '投票已更新',
                  operation: () => _repository.votePoll(
                    pollId: poll.id,
                    optionId: optionId,
                    remove: remove,
                  ),
                ),
              ),
            ],
            const SizedBox(height: 12),
            if (replyUnavailable case final String reason)
              AppEmptyState(title: '当前主题不可回复', description: reason)
            else
              CommentComposer(
                threadId: thread.id,
                authenticated: sessionState?.isAuthenticated ?? false,
                sessionGeneration: sessionState?.generation ?? 0,
                repository: _repository,
                onLogin: () => context.push(
                  publicInteractionLoginLocation(GoRouterState.of(context).uri),
                ),
                onPosted: () async {
                  await Future.wait(<Future<void>>[
                    _refreshThread(),
                    _refreshComments(),
                  ]);
                },
              ),
            const SizedBox(height: 18),
            Row(
              children: <Widget>[
                const Icon(Icons.forum_outlined),
                const SizedBox(width: 8),
                Text(
                  '楼层',
                  style: Theme.of(
                    context,
                  ).textTheme.titleLarge?.copyWith(fontWeight: FontWeight.w700),
                ),
              ],
            ),
            const SizedBox(height: 10),
            if (_comments.isEmpty)
              const AppEmptyState(title: '暂无回复', description: '来补充第一条回复。')
            else
              ..._comments.map(
                (Comment comment) => Padding(
                  padding: const EdgeInsets.only(bottom: 10),
                  child: _CommentCard(
                    comment: comment,
                    canManageSolution: canManageSolution,
                    pendingActions: _pendingActions,
                    onVote: (String value) => _voteComment(comment, value),
                    onBookmark: () => _bookmarkComment(comment),
                    onReport: () => _reportPost(comment.id, 'comment'),
                    onEdit: () => _editComment(comment),
                    onHistory: () => _showCommentRevisions(comment),
                    onDelete: () => _deleteComment(comment),
                    onToggleSolved: () => _toggleSolved(comment),
                    onRefreshDelivery: _refreshComments,
                    canReadHistory:
                        sessionState?.isAuthenticated == true &&
                        (currentAccountId == comment.authorId ||
                            comment.canModerate),
                  ),
                ),
              ),
            if (_hasMoreComments)
              OutlinedButton.icon(
                onPressed: _isLoadingMore ? null : _loadMoreComments,
                icon: _isLoadingMore
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.expand_more_rounded),
                label: Text(_isLoadingMore ? '加载中' : '加载更多回复'),
              ),
            const SizedBox(height: 32),
          ],
        ),
      ),
    );
  }

  String _subscriptionLabel(String? value) {
    return switch (value) {
      'watching' => '通知',
      'tracking' => '跟踪',
      'muted' => '静音',
      _ => '无',
    };
  }
}

class _CommentCard extends StatelessWidget {
  const _CommentCard({
    required this.comment,
    required this.canManageSolution,
    required this.pendingActions,
    required this.onVote,
    required this.onBookmark,
    required this.onReport,
    required this.onEdit,
    required this.onHistory,
    required this.onDelete,
    required this.onToggleSolved,
    required this.onRefreshDelivery,
    required this.canReadHistory,
  });

  final Comment comment;
  final bool canManageSolution;
  final Set<String> pendingActions;
  final ValueChanged<String> onVote;
  final VoidCallback onBookmark;
  final VoidCallback onReport;
  final VoidCallback onEdit;
  final VoidCallback onHistory;
  final VoidCallback onDelete;
  final VoidCallback onToggleSolved;
  final VoidCallback onRefreshDelivery;
  final bool canReadHistory;

  @override
  Widget build(BuildContext context) {
    return Card(
      color: comment.isSolved
          ? Theme.of(
              context,
            ).colorScheme.primaryContainer.withValues(alpha: 0.4)
          : null,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                PlatformAvatar(
                  radius: 17,
                  delivery: comment.authorAvatar,
                  fallbackText: comment.authorHandle,
                  semanticLabel: '${comment.authorHandle} 的头像',
                  onRefresh: onRefreshDelivery,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    comment.authorDisplayName ?? '@${comment.authorHandle}',
                    style: Theme.of(context).textTheme.labelLarge,
                  ),
                ),
                if (comment.isSolved)
                  const Chip(
                    avatar: Icon(Icons.check_circle_outline, size: 18),
                    label: Text('已采纳'),
                  ),
                Text(formatForumTime(comment.createdAt)),
                PopupMenuButton<String>(
                  onSelected: (String value) {
                    switch (value) {
                      case 'edit':
                        onEdit();
                      case 'history':
                        onHistory();
                      case 'delete':
                        onDelete();
                      case 'report':
                        onReport();
                    }
                  },
                  itemBuilder: (BuildContext context) =>
                      <PopupMenuEntry<String>>[
                        if (comment.canEdit)
                          const PopupMenuItem<String>(
                            value: 'edit',
                            child: Text('编辑'),
                          ),
                        if (canReadHistory)
                          const PopupMenuItem<String>(
                            value: 'history',
                            child: Text('修订历史'),
                          ),
                        if (comment.canDelete)
                          const PopupMenuItem<String>(
                            value: 'delete',
                            child: Text('删除'),
                          ),
                        const PopupMenuItem<String>(
                          value: 'report',
                          child: Text('举报'),
                        ),
                      ],
                ),
              ],
            ),
            if (comment.isDeleted)
              const Text('这条回复已删除。')
            else if (comment.isHidden)
              const Text('这条回复当前不可见。')
            else
              ForumBody(
                source: comment.body,
                format: comment.contentFormat,
                attachments: comment.attachments,
                onRefreshDelivery: onRefreshDelivery,
              ),
            if (comment.attachments.isNotEmpty) ...<Widget>[
              const SizedBox(height: 10),
              ...comment.attachments.map(
                (ForumAttachment attachment) => Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: ForumAttachmentImage(
                    attachment: attachment,
                    onRefreshDelivery: onRefreshDelivery,
                  ),
                ),
              ),
            ],
            const SizedBox(height: 10),
            Wrap(
              spacing: 6,
              children: <Widget>[
                TextButton.icon(
                  onPressed: pendingActions.contains('vote-${comment.id}')
                      ? null
                      : () => onVote('up'),
                  icon: const Icon(Icons.thumb_up_outlined),
                  label: Text(
                    '${comment.viewerVote?.value == 'up' ? '已顶' : '顶'} ${comment.voteCount}',
                  ),
                ),
                TextButton.icon(
                  onPressed: pendingActions.contains('vote-${comment.id}')
                      ? null
                      : () => onVote('down'),
                  icon: const Icon(Icons.thumb_down_outlined),
                  label: Text(comment.viewerVote?.value == 'down' ? '已踩' : '踩'),
                ),
                TextButton.icon(
                  onPressed: pendingActions.contains('bookmark-${comment.id}')
                      ? null
                      : onBookmark,
                  icon: Icon(
                    comment.isBookmarked
                        ? Icons.bookmark_rounded
                        : Icons.bookmark_outline,
                  ),
                  label: Text(comment.isBookmarked ? '已收藏' : '收藏'),
                ),
                if (canManageSolution || comment.isSolved)
                  TextButton.icon(
                    onPressed: pendingActions.contains('solve-${comment.id}')
                        ? null
                        : onToggleSolved,
                    icon: const Icon(Icons.check_circle_outline),
                    label: Text(comment.isSolved ? '取消采纳' : '采纳'),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class _PollCard extends StatelessWidget {
  const _PollCard({
    required this.poll,
    required this.isAuthenticated,
    required this.isPending,
    required this.onRequireAuthentication,
    required this.onVote,
  });

  final Poll poll;
  final bool isAuthenticated;
  final bool isPending;
  final Future<bool> Function() onRequireAuthentication;
  final Future<void> Function(String optionId, bool remove) onVote;

  @override
  Widget build(BuildContext context) {
    final int total = poll.options.fold<int>(
      0,
      (int sum, PollOption option) => sum + option.voteCount,
    );
    final bool isClosed =
        poll.closesAt != null &&
        poll.closesAt! <= DateTime.now().millisecondsSinceEpoch ~/ 1000;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              poll.question,
              style: Theme.of(
                context,
              ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 12),
            ...poll.options.map((PollOption option) {
              final bool selected = poll.myVotes.contains(option.id);
              final double fraction = total == 0 ? 0 : option.voteCount / total;
              return Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: OutlinedButton(
                  onPressed: isPending || isClosed
                      ? null
                      : () async {
                          if (!isAuthenticated &&
                              !await onRequireAuthentication()) {
                            return;
                          }
                          await onVote(option.id, selected);
                        },
                  style: OutlinedButton.styleFrom(
                    padding: const EdgeInsets.all(12),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Row(
                        children: <Widget>[
                          if (selected)
                            const Icon(Icons.check_circle, size: 18),
                          if (selected) const SizedBox(width: 6),
                          Expanded(child: Text(option.label)),
                          Text(
                            '${option.voteCount} 票 · ${(fraction * 100).round()}%',
                          ),
                        ],
                      ),
                      const SizedBox(height: 6),
                      LinearProgressIndicator(value: fraction),
                    ],
                  ),
                ),
              );
            }),
            Text(
              '共 $total 票${poll.multiSelect ? ' · 可多选' : ''}${isClosed ? ' · 已截止' : ''}',
            ),
          ],
        ),
      ),
    );
  }
}

class _ReportDraft {
  const _ReportDraft(this.reason, this.note);

  final FlagInputReasonEnum reason;
  final String? note;
}

class _ReportDialog extends StatefulWidget {
  const _ReportDialog();

  @override
  State<_ReportDialog> createState() => _ReportDialogState();
}

class _ReportDialogState extends State<_ReportDialog> {
  final TextEditingController _noteController = TextEditingController();
  FlagInputReasonEnum _reason = FlagInputReasonEnum.spam;

  @override
  void dispose() {
    _noteController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('举报内容'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          DropdownButtonFormField<FlagInputReasonEnum>(
            initialValue: _reason,
            decoration: const InputDecoration(labelText: '原因'),
            items: const <DropdownMenuItem<FlagInputReasonEnum>>[
              DropdownMenuItem(
                value: FlagInputReasonEnum.spam,
                child: Text('垃圾信息'),
              ),
              DropdownMenuItem(
                value: FlagInputReasonEnum.abuse,
                child: Text('辱骂或骚扰'),
              ),
              DropdownMenuItem(
                value: FlagInputReasonEnum.offTopic,
                child: Text('偏离主题'),
              ),
              DropdownMenuItem(
                value: FlagInputReasonEnum.illegal,
                child: Text('违法内容'),
              ),
              DropdownMenuItem(
                value: FlagInputReasonEnum.other,
                child: Text('其他'),
              ),
            ],
            onChanged: (FlagInputReasonEnum? value) {
              if (value != null) {
                setState(() => _reason = value);
              }
            },
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _noteController,
            maxLength: 500,
            maxLines: 4,
            decoration: const InputDecoration(labelText: '补充说明（可选）'),
          ),
        ],
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: () => Navigator.of(
            context,
          ).pop(_ReportDraft(_reason, _noteController.text)),
          child: const Text('提交'),
        ),
      ],
    );
  }
}

class _EditThreadDialog extends StatefulWidget {
  const _EditThreadDialog({required this.thread, required this.repository});

  final ThreadDetail thread;
  final ForumRepository repository;

  @override
  State<_EditThreadDialog> createState() => _EditThreadDialogState();
}

class _EditThreadDialogState extends State<_EditThreadDialog> {
  late final TextEditingController _titleController = TextEditingController(
    text: widget.thread.title,
  );
  late final TextEditingController _bodyController = TextEditingController(
    text: widget.thread.body ?? '',
  );
  late final TextEditingController _tagsController = TextEditingController(
    text: widget.thread.tags.join(' '),
  );
  bool _isSaving = false;
  String? _error;

  @override
  void dispose() {
    _titleController.dispose();
    _bodyController.dispose();
    _tagsController.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (_isSaving || _titleController.text.trim().isEmpty) {
      return;
    }
    setState(() {
      _isSaving = true;
      _error = null;
    });
    try {
      await widget.repository.updateThread(
        widget.thread.id,
        ThreadUpdateInput(
          expectedVersion: widget.thread.contentVersion,
          title: _titleController.text.trim(),
          body: _bodyController.text,
          contentFormat: ContentFormat.markdownV1,
          tags: _tagsController.text
              .split(RegExp(r'[,\s，、]+'))
              .map((String tag) => tag.trim())
              .where((String tag) => tag.isNotEmpty)
              .take(3)
              .toSet(),
          attachmentAssetIds:
              RegExp(r'!\[[^\]]*\]\(yourtj-asset:([1-9][0-9]*)\)')
                  .allMatches(_bodyController.text)
                  .map((RegExpMatch match) => match.group(1)!)
                  .toSet(),
        ),
      );
      if (mounted) {
        Navigator.of(context).pop(true);
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() {
          _error = failure.kind == ApiFailureKind.conflict
              ? '主题已在其他设备更新。你的输入仍保留；关闭后刷新，再重新确认修改。'
              : failure.message;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _isSaving = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('编辑主题'),
      content: SizedBox(
        width: 640,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (widget.thread.contentFormat ==
                  ContentFormat.plainV1) ...<Widget>[
                Semantics(
                  liveRegion: true,
                  child: Text('这是旧版纯文本内容。保存会显式升级为 Markdown，请先确认预览。'),
                ),
                const SizedBox(height: 12),
              ],
              TextField(
                controller: _titleController,
                maxLength: 120,
                decoration: const InputDecoration(labelText: '标题'),
              ),
              const SizedBox(height: 12),
              ForumMarkdownComposer(
                controller: _bodyController,
                label: '正文（Markdown）',
                minLines: 8,
                maxLines: 16,
                maxLength: 50000,
                attachments: widget.thread.attachments,
              ),
              const SizedBox(height: 12),
              TextField(
                controller: _tagsController,
                decoration: const InputDecoration(labelText: '标签'),
              ),
              if (_error case final String error) ...<Widget>[
                const SizedBox(height: 12),
                Text(
                  error,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              ],
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _isSaving ? null : () => Navigator.of(context).pop(false),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _isSaving ? null : _save,
          child: Text(_isSaving ? '保存中' : '保存'),
        ),
      ],
    );
  }
}

class _EditCommentDialog extends StatefulWidget {
  const _EditCommentDialog({required this.comment, required this.repository});

  final Comment comment;
  final ForumRepository repository;

  @override
  State<_EditCommentDialog> createState() => _EditCommentDialogState();
}

class _EditCommentDialogState extends State<_EditCommentDialog> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.comment.body,
  );
  bool _isSaving = false;
  String? _error;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (_isSaving || _controller.text.trim().isEmpty) {
      return;
    }
    setState(() {
      _isSaving = true;
      _error = null;
    });
    try {
      await widget.repository.updateComment(
        widget.comment.id,
        CommentUpdateInput(
          expectedVersion: widget.comment.contentVersion,
          body: _controller.text,
          contentFormat: ContentFormat.markdownV1,
          attachmentAssetIds:
              RegExp(r'!\[[^\]]*\]\(yourtj-asset:([1-9][0-9]*)\)')
                  .allMatches(_controller.text)
                  .map((RegExpMatch match) => match.group(1)!)
                  .toSet(),
        ),
      );
      if (mounted) {
        Navigator.of(context).pop(true);
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() {
          _error = failure.kind == ApiFailureKind.conflict
              ? '回复已在其他设备更新。你的输入仍保留；关闭后刷新，再重新确认修改。'
              : failure.message;
        });
      }
    } finally {
      if (mounted) {
        setState(() => _isSaving = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('编辑回复'),
      content: SizedBox(
        width: 600,
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (widget.comment.contentFormat ==
                  ContentFormat.plainV1) ...<Widget>[
                Semantics(
                  liveRegion: true,
                  child: Text('这是旧版纯文本内容。保存会显式升级为 Markdown，请先确认预览。'),
                ),
                const SizedBox(height: 12),
              ],
              ForumMarkdownComposer(
                controller: _controller,
                label: '回复（Markdown）',
                minLines: 6,
                maxLines: 14,
                maxLength: 16000,
                attachments: widget.comment.attachments,
              ),
              if (_error case final String error)
                Text(
                  error,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _isSaving ? null : () => Navigator.of(context).pop(false),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _isSaving ? null : _save,
          child: Text(_isSaving ? '保存中' : '保存'),
        ),
      ],
    );
  }
}

class CommentComposer extends StatefulWidget {
  const CommentComposer({
    required this.threadId,
    required this.authenticated,
    required this.sessionGeneration,
    required this.repository,
    required this.onLogin,
    required this.onPosted,
    super.key,
  });

  final String threadId;
  final bool authenticated;
  final int sessionGeneration;
  final ForumRepository repository;
  final VoidCallback onLogin;
  final Future<void> Function() onPosted;

  @override
  State<CommentComposer> createState() => _CommentComposerState();
}

class _CommentComposerState extends State<CommentComposer> {
  final TextEditingController _controller = TextEditingController();
  Timer? _draftTimer;
  int _draftVersion = 0;
  DraftOutput? _remoteConflict;
  bool _isLoadingDraft = false;
  bool _isSavingDraft = false;
  bool _isPosting = false;
  bool _isPosted = false;
  String? _draftNotice;
  String? _error;
  int _generation = 0;

  String get _draftKey => 'comment:${widget.threadId}';

  @override
  void initState() {
    super.initState();
    _generation = 1;
    _controller.addListener(_scheduleSave);
    if (widget.authenticated) {
      unawaited(_loadDraft());
    }
  }

  @override
  void didUpdateWidget(CommentComposer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.threadId != widget.threadId ||
        oldWidget.sessionGeneration != widget.sessionGeneration ||
        oldWidget.authenticated != widget.authenticated) {
      _generation += 1;
      _draftTimer?.cancel();
      _isLoadingDraft = false;
      _isSavingDraft = false;
      _isPosting = false;
      _isPosted = false;
      _draftVersion = 0;
      _remoteConflict = null;
      _draftNotice = null;
      _error = null;
      _controller.removeListener(_scheduleSave);
      _controller.clear();
      _controller.addListener(_scheduleSave);
      if (widget.authenticated) {
        unawaited(_loadDraft());
      }
    }
  }

  @override
  void dispose() {
    _draftTimer?.cancel();
    _controller.dispose();
    super.dispose();
  }

  Set<String> get _attachmentAssetIds =>
      RegExp(r'!\[[^\]]*\]\(yourtj-asset:([1-9][0-9]*)\)')
          .allMatches(_controller.text)
          .map((RegExpMatch match) => match.group(1)!)
          .toSet();

  CommentDraftPayload get _payload => CommentDraftPayload(
    kind: CommentDraftPayloadKindEnum.comment,
    threadId: widget.threadId,
    body: _controller.text,
    contentFormat: ContentFormat.markdownV1,
    parentId: null,
    attachmentAssetIds: _attachmentAssetIds,
  );

  Future<void> _loadDraft() async {
    if (_isLoadingDraft) {
      return;
    }
    final int generation = _generation;
    final String threadId = widget.threadId;
    final String draftKey = _draftKey;
    setState(() => _isLoadingDraft = true);
    try {
      final DraftOutput? draft = await widget.repository.draft(draftKey);
      if (!mounted ||
          generation != _generation ||
          threadId != widget.threadId ||
          draft == null) {
        return;
      }
      _draftVersion = draft.version;
      if (draft.payload case final ForumCommentDraftPayload payload) {
        if (payload.payload.threadId == threadId && _controller.text.isEmpty) {
          _controller.text = payload.payload.body;
          setState(() => _draftNotice = '已恢复跨设备回复草稿');
        } else if (_controller.text != payload.payload.body) {
          setState(() {
            _remoteConflict = draft;
            _draftNotice = '云端有另一版回复草稿；请选择保留哪一版';
          });
        }
      }
    } on ApiFailure catch (failure) {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() => _draftNotice = '云端草稿暂不可用：${failure.message}');
      }
    } finally {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() => _isLoadingDraft = false);
      }
    }
  }

  void _scheduleSave() {
    if (!widget.authenticated || _isLoadingDraft || _isPosting || _isPosted) {
      return;
    }
    _draftTimer?.cancel();
    _draftTimer = Timer(const Duration(milliseconds: 900), () {
      unawaited(_saveDraft());
    });
  }

  Future<void> _saveDraft({int? expectedVersion}) async {
    if (_isSavingDraft || _isPosted || _controller.text.isEmpty) {
      return;
    }
    if (mounted) {
      setState(() {
        _isSavingDraft = true;
        _draftNotice = null;
      });
    }
    final int generation = _generation;
    final String threadId = widget.threadId;
    final String draftKey = _draftKey;
    final CommentDraftPayload payload = _payload;
    final int version = expectedVersion ?? _draftVersion;
    try {
      final DraftOutput saved = await widget.repository.saveDraft(
        DraftSaveInput(
          draftKey: draftKey,
          expectedVersion: version,
          payload: ForumDraftPayload.comment(payload),
        ),
      );
      if (!mounted ||
          generation != _generation ||
          threadId != widget.threadId) {
        return;
      }
      _draftVersion = saved.version;
      _remoteConflict = null;
      if (mounted) {
        setState(() => _draftNotice = '草稿已同步');
      }
    } on ApiFailure catch (failure) {
      if (!mounted ||
          generation != _generation ||
          threadId != widget.threadId) {
        return;
      }
      if (failure.kind == ApiFailureKind.conflict) {
        await _readConflict();
      } else {
        setState(() => _draftNotice = '草稿未同步：${failure.message}');
      }
    } finally {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() => _isSavingDraft = false);
      }
    }
  }

  Future<void> _readConflict() async {
    final int generation = _generation;
    final String threadId = widget.threadId;
    final String draftKey = _draftKey;
    try {
      final DraftOutput? latest = await widget.repository.draft(draftKey);
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() {
          _remoteConflict = latest;
          _draftNotice = latest == null
              ? '远端草稿已删除；本地输入仍保留'
              : '另一台设备修改了草稿；请选择保留哪一版';
        });
      }
    } on ApiFailure catch (failure) {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() => _draftNotice = '草稿冲突：${failure.message}');
      }
    }
  }

  void _useRemote() {
    final DraftOutput? remote = _remoteConflict;
    if (remote?.payload case final ForumCommentDraftPayload payload) {
      _controller.text = payload.payload.body;
      setState(() {
        _draftVersion = remote!.version;
        _remoteConflict = null;
        _draftNotice = '已使用云端版本';
      });
    }
  }

  Future<void> _keepLocal() async {
    final DraftOutput? remote = _remoteConflict;
    if (remote == null) {
      return;
    }
    _draftVersion = remote.version;
    await _saveDraft(expectedVersion: remote.version);
  }

  Future<void> _post() async {
    if (_isPosting || _controller.text.trim().isEmpty) {
      return;
    }
    setState(() {
      _isPosting = true;
      _error = null;
    });
    final int generation = _generation;
    final String threadId = widget.threadId;
    final String draftKey = _draftKey;
    final String body = _controller.text;
    final Set<String> attachmentAssetIds = _attachmentAssetIds;
    final Future<void> Function() onPosted = widget.onPosted;
    try {
      await widget.repository.createComment(
        threadId,
        CommentInput(
          body: body,
          contentFormat: ContentFormat.markdownV1,
          attachmentAssetIds: attachmentAssetIds,
        ),
      );
      if (!mounted ||
          generation != _generation ||
          threadId != widget.threadId) {
        return;
      }
      _isPosted = true;
      try {
        await widget.repository.deleteDraft(draftKey);
      } on ApiFailure {
        if (mounted &&
            generation == _generation &&
            threadId == widget.threadId) {
          ScaffoldMessenger.of(
            context,
          ).showSnackBar(const SnackBar(content: Text('回复已发布，但云端草稿清理失败')));
        }
      }
      if (!mounted ||
          generation != _generation ||
          threadId != widget.threadId) {
        return;
      }
      _controller.clear();
      _draftVersion = 0;
      _isPosted = false;
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('回复已发布')));
        await onPosted();
      }
    } on ApiFailure catch (failure) {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() => _error = failure.message);
      }
    } finally {
      if (mounted && generation == _generation && threadId == widget.threadId) {
        setState(() => _isPosting = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    if (!widget.authenticated) {
      return AppEmptyState(
        title: '登录后回复',
        description: '登录后可以参与讨论、投票、收藏和举报。',
        action: FilledButton(
          onPressed: widget.onLogin,
          child: const Text('登录'),
        ),
      );
    }
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text('回复', style: Theme.of(context).textTheme.titleMedium),
            if (_isLoadingDraft) const LinearProgressIndicator(),
            if (_draftNotice case final String notice) ...<Widget>[
              const SizedBox(height: 8),
              Text(notice),
              if (_remoteConflict != null)
                Wrap(
                  spacing: 8,
                  children: <Widget>[
                    OutlinedButton(
                      onPressed: _useRemote,
                      child: const Text('使用云端版本'),
                    ),
                    FilledButton.tonal(
                      onPressed: _keepLocal,
                      child: const Text('保留本地并覆盖'),
                    ),
                  ],
                ),
            ],
            const SizedBox(height: 10),
            ForumMarkdownComposer(
              key: ValueKey<String>(
                'comment-${widget.threadId}-${widget.sessionGeneration}',
              ),
              controller: _controller,
              label: '回复正文（Markdown）',
              minLines: 5,
              maxLines: 12,
              maxLength: 16000,
              helperText: '图片通过一次性 OSS 凭证直传；正文只保存 yourtj-asset 引用。',
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 10,
              runSpacing: 8,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: <Widget>[
                if (_attachmentAssetIds.length < 4)
                  MediaUploadButton(
                    key: ValueKey<String>(
                      'comment-upload-${widget.threadId}-${widget.sessionGeneration}',
                    ),
                    kind: MediaUploadKind.image,
                    usage: MediaUsage.forumComment,
                    onUploaded: _insertUploadedImage,
                  ),
                Text('${_attachmentAssetIds.length}/4 张图片'),
              ],
            ),
            if (_error case final String error)
              Text(
                error,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                OutlinedButton.icon(
                  onPressed: _isSavingDraft || _controller.text.trim().isEmpty
                      ? null
                      : _saveDraft,
                  icon: const Icon(Icons.cloud_upload_outlined),
                  label: const Text('保存草稿'),
                ),
                FilledButton.icon(
                  onPressed: _isPosting ? null : _post,
                  icon: _isPosting
                      ? const SizedBox.square(
                          dimension: 18,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Icon(Icons.send_rounded),
                  label: Text(_isPosting ? '发布中' : '发布回复'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  void _insertUploadedImage(CompletedMediaUpload upload) {
    final String separator =
        _controller.text.isEmpty || _controller.text.endsWith('\n') ? '' : '\n';
    _controller.text =
        '${_controller.text}$separator![图片](yourtj-asset:${upload.uploadId})\n';
    _controller.selection = TextSelection.collapsed(
      offset: _controller.text.length,
    );
    setState(() => _draftNotice = '图片已上传，发布前会保持为受控资源引用');
  }
}
