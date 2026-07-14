import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../auth/domain/session_state.dart';
import '../../forum/data/forum_repository.dart';
import '../../forum/presentation/forum_widgets.dart';

class BookmarksPage extends ConsumerStatefulWidget {
  const BookmarksPage({super.key});

  @override
  ConsumerState<BookmarksPage> createState() => _BookmarksPageState();
}

class _BookmarksPageState extends ConsumerState<BookmarksPage> {
  List<Bookmark> _items = <Bookmark>[];
  String? _nextCursor;
  bool _hasMore = false;
  bool _isLoading = true;
  bool _isLoadingMore = false;
  String? _removingId;
  ApiFailure? _error;
  String? _loadedAccountId;

  ForumRepository get _repository => ref.read(forumRepositoryProvider);

  @override
  void initState() {
    super.initState();
  }

  Future<void> _load() async {
    setState(() {
      _isLoading = true;
      _error = null;
    });
    try {
      final ForumPageSlice<Bookmark> page = await _repository.bookmarks();
      if (mounted) {
        setState(() {
          _items = page.items;
          _nextCursor = page.nextCursor;
          _hasMore = page.hasMore;
        });
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _error = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _loadMore() async {
    if (_isLoadingMore || !_hasMore || _nextCursor == null) {
      return;
    }
    setState(() => _isLoadingMore = true);
    try {
      final ForumPageSlice<Bookmark> page = await _repository.bookmarks(
        cursor: _nextCursor,
      );
      if (mounted) {
        final Set<String> keys = _items
            .map((Bookmark item) => '${item.targetType.value}:${item.targetId}')
            .toSet();
        setState(() {
          _items.addAll(
            page.items.where(
              (Bookmark item) =>
                  keys.add('${item.targetType.value}:${item.targetId}'),
            ),
          );
          _nextCursor = page.nextCursor;
          _hasMore = page.hasMore;
        });
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(failure.message)));
      }
    } finally {
      if (mounted) {
        setState(() => _isLoadingMore = false);
      }
    }
  }

  Future<void> _remove(Bookmark item) async {
    if (_removingId != null) {
      return;
    }
    setState(() => _removingId = item.targetId);
    try {
      await _repository.setBookmark(
        id: item.targetId,
        postType: item.targetType.value,
        bookmarked: true,
      );
      if (mounted) {
        setState(() => _items.remove(item));
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('已取消收藏')));
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(failure.message)));
      }
    } finally {
      if (mounted) {
        setState(() => _removingId = null);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final AsyncValue<SessionState> session = ref.watch(sessionStateProvider);
    final SessionState? state = session.value;
    final bool authenticated = state?.isAuthenticated ?? false;
    final String? accountId = state?.account?.id;
    if (accountId != _loadedAccountId) {
      _loadedAccountId = accountId;
      WidgetsBinding.instance.addPostFrameCallback((Duration _) {
        if (!mounted) {
          return;
        }
        if (accountId == null) {
          setState(() {
            _items = <Bookmark>[];
            _error = null;
            _isLoading = false;
          });
        } else {
          _load();
        }
      });
    }
    return Scaffold(
      appBar: AppBar(title: const Text('我的收藏')),
      body: session.isLoading
          ? const AppLoadingState(title: '恢复登录状态')
          : !authenticated
          ? AppEmptyState(
              title: '登录后查看收藏',
              action: FilledButton(
                onPressed: () => context.push(AppRoutes.login),
                child: const Text('登录'),
              ),
            )
          : _body(),
    );
  }

  Widget _body() {
    if (_isLoading) {
      return const AppLoadingState(title: '加载收藏');
    }
    if (_error case final ApiFailure failure) {
      return failure.kind == ApiFailureKind.forbidden
          ? AppPermissionState(description: failure.message)
          : AppErrorState(description: failure.message, onRetry: _load);
    }
    if (_items.isEmpty) {
      return AppEmptyState(
        title: '暂无可见收藏',
        description: '被删除或已不可见的内容不会出现在这里。',
        action: _hasMore
            ? OutlinedButton(
                onPressed: _isLoadingMore ? null : _loadMore,
                child: const Text('继续查找较早收藏'),
              )
            : null,
      );
    }
    return RefreshIndicator(
      onRefresh: _load,
      child: ListView.separated(
        padding: const EdgeInsets.all(16),
        itemCount: _items.length + 1,
        separatorBuilder: (BuildContext context, int index) =>
            const SizedBox(height: 10),
        itemBuilder: (BuildContext context, int index) {
          if (index == _items.length) {
            return _hasMore
                ? OutlinedButton.icon(
                    onPressed: _isLoadingMore ? null : _loadMore,
                    icon: _isLoadingMore
                        ? const SizedBox.square(
                            dimension: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.expand_more_rounded),
                    label: Text(_isLoadingMore ? '加载中' : '加载更多'),
                  )
                : const Center(
                    child: Padding(
                      padding: EdgeInsets.all(12),
                      child: Text('已经到底了'),
                    ),
                  );
          }
          final Bookmark item = _items[index];
          return Card(
            child: InkWell(
              onTap: () =>
                  context.push(AppRoutes.thread(item.content.threadId)),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Row(
                      children: <Widget>[
                        Expanded(
                          child: Text(
                            item.content.authorDisplayName ??
                                '@${item.content.authorHandle}',
                            style: Theme.of(context).textTheme.labelLarge,
                          ),
                        ),
                        Text('收藏于 ${formatForumTime(item.createdAt)}'),
                      ],
                    ),
                    const SizedBox(height: 10),
                    Text(
                      item.content.title,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    if (item.content.body case final String body) ...<Widget>[
                      const SizedBox(height: 8),
                      ForumBody(
                        source: body,
                        format: item.content.contentFormat,
                        attachments: item.content.attachments,
                        onRefreshDelivery: _load,
                      ),
                    ],
                    const SizedBox(height: 12),
                    Row(
                      children: <Widget>[
                        Text('${item.content.voteCount} 票'),
                        const SizedBox(width: 16),
                        Text('${item.content.replyCount} 回复'),
                        const Spacer(),
                        TextButton.icon(
                          onPressed: _removingId == item.targetId
                              ? null
                              : () => _remove(item),
                          icon: const Icon(Icons.bookmark_remove_outlined),
                          label: const Text('取消收藏'),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}
