import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../auth/domain/session_state.dart';
import '../data/forum_repository.dart';
import 'create_thread_sheet.dart';
import 'forum_widgets.dart';

class ForumPage extends ConsumerStatefulWidget {
  const ForumPage({this.initialBoardId, this.initialTag, super.key});

  final String? initialBoardId;
  final String? initialTag;

  @override
  ConsumerState<ForumPage> createState() => _ForumPageState();
}

class _ForumPageState extends ConsumerState<ForumPage> {
  List<Board> _boards = <Board>[];
  List<Tag> _tags = <Tag>[];
  List<ThreadFeed> _threads = <ThreadFeed>[];
  ForumFeed _feed = ForumFeed.hot;
  String? _boardId;
  String? _tag;
  String? _nextCursor;
  bool _hasMore = false;
  bool _isLoading = true;
  bool _isLoadingMore = false;
  ApiFailure? _error;
  int _requestGeneration = 0;
  int? _loadedSessionGeneration;

  ForumRepository get _repository => ref.read(forumRepositoryProvider);

  bool get _isAuthenticated =>
      ref.read(sessionStateProvider).value?.isAuthenticated ?? false;

  @override
  void initState() {
    super.initState();
    _boardId = widget.initialBoardId;
    _tag = widget.initialTag;
    _loadAll();
  }

  @override
  void didUpdateWidget(covariant ForumPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.initialBoardId == widget.initialBoardId &&
        oldWidget.initialTag == widget.initialTag) {
      return;
    }
    _boardId = widget.initialBoardId;
    _tag = widget.initialTag;
    unawaited(_loadAll());
  }

  Future<void> _loadAll() async {
    final int generation = ++_requestGeneration;
    setState(() {
      _isLoading = true;
      _error = null;
    });
    try {
      final List<Object> results = await Future.wait<Object>(<Future<Object>>[
        _repository.boards(),
        _repository.tags(),
        _repository.threads(feed: _feed, boardId: _boardId, tag: _tag),
      ]);
      if (!mounted || generation != _requestGeneration) {
        return;
      }
      final ForumPageSlice<ThreadFeed> page =
          results[2] as ForumPageSlice<ThreadFeed>;
      setState(() {
        _boards = results[0] as List<Board>;
        _tags = results[1] as List<Tag>;
        _threads = page.items;
        _nextCursor = page.nextCursor;
        _hasMore = page.hasMore;
      });
    } on ApiFailure catch (failure) {
      if (mounted && generation == _requestGeneration) {
        setState(() => _error = failure);
      }
    } finally {
      if (mounted && generation == _requestGeneration) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _reloadThreads() async {
    final int generation = ++_requestGeneration;
    setState(() {
      _isLoading = true;
      _error = null;
    });
    try {
      final ForumPageSlice<ThreadFeed> page = await _repository.threads(
        feed: _feed,
        boardId: _boardId,
        tag: _tag,
      );
      if (mounted && generation == _requestGeneration) {
        setState(() {
          _threads = page.items;
          _nextCursor = page.nextCursor;
          _hasMore = page.hasMore;
        });
      }
    } on ApiFailure catch (failure) {
      if (mounted && generation == _requestGeneration) {
        setState(() => _error = failure);
      }
    } finally {
      if (mounted && generation == _requestGeneration) {
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
      final ForumPageSlice<ThreadFeed> page = await _repository.threads(
        feed: _feed,
        boardId: _boardId,
        tag: _tag,
        cursor: _nextCursor,
      );
      if (mounted) {
        final Set<String> knownIds = _threads
            .map((ThreadFeed thread) => thread.id)
            .toSet();
        setState(() {
          _threads.addAll(
            page.items.where((ThreadFeed thread) => knownIds.add(thread.id)),
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

  Future<void> _selectFeed(ForumFeed feed) async {
    if (feed.requiresAuthentication && !_isAuthenticated) {
      await context.push(
        publicInteractionLoginLocation(GoRouterState.of(context).uri),
      );
      if (!mounted || !_isAuthenticated) {
        return;
      }
    }
    if (feed == _feed) {
      return;
    }
    setState(() => _feed = feed);
    await _reloadThreads();
  }

  Future<void> _createThread() async {
    if (!_isAuthenticated) {
      await context.push(
        publicInteractionLoginLocation(GoRouterState.of(context).uri),
      );
      if (!mounted || !_isAuthenticated) {
        return;
      }
    }
    if (!mounted) {
      return;
    }
    try {
      final List<Board> boards = await _repository.boards();
      if (!mounted) {
        return;
      }
      setState(() => _boards = boards);
    } on ApiFailure catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(failure.message)));
      }
      return;
    }
    await showCreateThreadSheet(context: context, boards: _boards);
    if (mounted) {
      await _reloadThreads();
    }
  }

  String? _boardName(String id) {
    for (final Board board in _boards) {
      if (board.id == id) {
        return board.name;
      }
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final AsyncValue<SessionState> session = ref.watch(sessionStateProvider);
    final SessionState? sessionState = session.value;
    final bool authenticated = sessionState?.isAuthenticated ?? false;
    if (sessionState != null &&
        _loadedSessionGeneration != sessionState.generation) {
      _loadedSessionGeneration = sessionState.generation;
      WidgetsBinding.instance.addPostFrameCallback((Duration _) {
        if (mounted) {
          unawaited(_loadAll());
        }
      });
    }
    if (!authenticated && _feed.requiresAuthentication) {
      WidgetsBinding.instance.addPostFrameCallback((Duration _) {
        if (mounted && _feed.requiresAuthentication) {
          setState(() => _feed = ForumFeed.hot);
          unawaited(_reloadThreads());
        }
      });
    }
    return CustomScrollView(
      slivers: <Widget>[
        SliverAppBar(
          pinned: true,
          title: const Text('社区'),
          actions: <Widget>[
            IconButton(
              tooltip: '我的收藏',
              onPressed: () async {
                if (!authenticated) {
                  await context.push(
                    publicInteractionLoginLocation(
                      GoRouterState.of(context).uri,
                    ),
                  );
                  if (context.mounted && _isAuthenticated) {
                    await context.push(AppRoutes.bookmarks);
                  }
                } else if (context.mounted) {
                  await context.push(AppRoutes.bookmarks);
                }
              },
              icon: const Icon(Icons.bookmarks_outlined),
            ),
            IconButton(
              tooltip: '发布新帖',
              onPressed: _createThread,
              icon: const Icon(Icons.add_circle_outline_rounded),
            ),
          ],
        ),
        SliverToBoxAdapter(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: ForumFeed.values
                        .map(
                          (ForumFeed feed) => Padding(
                            padding: const EdgeInsets.only(right: 8),
                            child: ChoiceChip(
                              selected: _feed == feed,
                              onSelected: (_) => _selectFeed(feed),
                              avatar: feed.requiresAuthentication
                                  ? const Icon(Icons.lock_outline, size: 16)
                                  : null,
                              label: Text(feed.label),
                            ),
                          ),
                        )
                        .toList(),
                  ),
                ),
                const SizedBox(height: 10),
                Wrap(
                  spacing: 10,
                  runSpacing: 10,
                  children: <Widget>[
                    DropdownMenu<String?>(
                      key: ValueKey<String>('forum-board-${_boardId ?? 'all'}'),
                      initialSelection: _boardId,
                      label: const Text('板块'),
                      dropdownMenuEntries: <DropdownMenuEntry<String?>>[
                        const DropdownMenuEntry<String?>(
                          value: null,
                          label: '全部板块',
                        ),
                        ..._boards.map(
                          (Board board) => DropdownMenuEntry<String?>(
                            value: board.id,
                            label: board.name,
                          ),
                        ),
                      ],
                      onSelected: (String? value) {
                        setState(() => _boardId = value);
                        _reloadThreads();
                      },
                    ),
                    DropdownMenu<String?>(
                      key: ValueKey<String>('forum-tag-${_tag ?? 'all'}'),
                      initialSelection: _tag,
                      label: const Text('标签'),
                      dropdownMenuEntries: <DropdownMenuEntry<String?>>[
                        const DropdownMenuEntry<String?>(
                          value: null,
                          label: '全部标签',
                        ),
                        ..._tags
                            .where((Tag tag) => tag.slug != null)
                            .map(
                              (Tag tag) => DropdownMenuEntry<String?>(
                                value: tag.slug,
                                label: tag.name ?? '#${tag.slug}',
                              ),
                            ),
                      ],
                      onSelected: (String? value) {
                        setState(() => _tag = value);
                        _reloadThreads();
                      },
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
        if (_isLoading)
          const SliverFillRemaining(child: AppLoadingState(title: '加载社区'))
        else if (_error case final ApiFailure failure)
          SliverFillRemaining(
            child: failure.kind == ApiFailureKind.forbidden
                ? AppPermissionState(description: failure.message)
                : AppErrorState(
                    description: failure.message,
                    onRetry: _loadAll,
                  ),
          )
        else if (_threads.isEmpty)
          const SliverFillRemaining(
            child: AppEmptyState(
              title: '这里还没有主题',
              description: '可以换一个板块、标签或动态流看看。',
            ),
          )
        else ...<Widget>[
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(16, 4, 16, 12),
            sliver: SliverList.separated(
              itemCount: _threads.length,
              itemBuilder: (BuildContext context, int index) {
                final ThreadFeed thread = _threads[index];
                return ForumThreadCard(
                  thread: thread,
                  boardName: _boardName(thread.boardId),
                  onRefreshDelivery: _reloadThreads,
                );
              },
              separatorBuilder: (BuildContext context, int index) =>
                  const SizedBox(height: 10),
            ),
          ),
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 32),
              child: _hasMore
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
                  : const Center(child: Text('已经到底了')),
            ),
          ),
        ],
      ],
    );
  }
}
