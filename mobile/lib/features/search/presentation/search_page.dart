import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../../core/widgets/platform_avatar.dart';
import '../../auth/domain/session_state.dart';
import '../data/search_repository.dart';
import '../domain/search_controller.dart';
import '../domain/search_models.dart';
import 'safe_highlighted_text.dart';

class SearchPage extends ConsumerStatefulWidget {
  const SearchPage({
    this.initialQuery = '',
    this.initialScope = SearchScope.all,
    super.key,
  });

  final String initialQuery;
  final SearchScope initialScope;

  @override
  ConsumerState<SearchPage> createState() => _SearchPageState();
}

class _SearchPageState extends ConsumerState<SearchPage> {
  late final FederatedSearchController _controller;
  late final TextEditingController _queryController;
  (int, SessionPhase, String?)? _sessionIdentity;

  @override
  void initState() {
    super.initState();
    _queryController = TextEditingController(text: widget.initialQuery);
    _controller = FederatedSearchController(
      ref.read(federatedSearchRepositoryProvider),
      initialScope: widget.initialScope,
    );
    ref.listenManual<AsyncValue<SessionState>>(
      sessionStateProvider,
      _handleSessionState,
      fireImmediately: true,
    );
    if (widget.initialQuery.trim().length >= 2) {
      _controller.submit(widget.initialQuery, scope: widget.initialScope);
    }
  }

  void _handleSessionState(
    AsyncValue<SessionState>? _,
    AsyncValue<SessionState> next,
  ) {
    final SessionState? state = next.value;
    if (state == null) {
      return;
    }
    final (int, SessionPhase, String?) identity = (
      state.generation,
      state.phase,
      state.account?.id,
    );
    if (_sessionIdentity == identity) {
      return;
    }
    _sessionIdentity = identity;
    final bool shouldReload = _controller.query.length >= 2;
    _controller.invalidateForSessionChange();
    if (shouldReload) {
      unawaited(_controller.reload());
    }
  }

  @override
  void dispose() {
    _queryController.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('全站搜索')),
      body: ListenableBuilder(
        listenable: _controller,
        builder: (BuildContext context, Widget? child) {
          return CustomScrollView(
            slivers: <Widget>[
              SliverPadding(
                padding: const EdgeInsets.fromLTRB(16, 18, 16, 0),
                sliver: SliverToBoxAdapter(child: _searchField(context)),
              ),
              SliverPadding(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 0),
                sliver: SliverToBoxAdapter(child: _scopePicker()),
              ),
              ..._content(context),
              const SliverPadding(padding: EdgeInsets.only(bottom: 36)),
            ],
          );
        },
      ),
    );
  }

  Widget _searchField(BuildContext context) {
    return ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 900),
      child: Row(
        children: <Widget>[
          Expanded(
            child: TextField(
              controller: _queryController,
              autofocus: widget.initialQuery.isEmpty,
              textInputAction: TextInputAction.search,
              decoration: const InputDecoration(
                labelText: '搜索课程、课评、帖子、用户、板块或标签',
                prefixIcon: Icon(Icons.search_rounded),
              ),
              onSubmitted: _submit,
            ),
          ),
          const SizedBox(width: 10),
          FilledButton(
            onPressed: () => _submit(_queryController.text),
            child: const Text('搜索'),
          ),
        ],
      ),
    );
  }

  Widget _scopePicker() {
    return Semantics(
      container: true,
      label: '搜索范围',
      child: SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Row(
          children: SearchScope.values
              .map((SearchScope scope) {
                return Padding(
                  padding: const EdgeInsets.only(right: 8),
                  child: ChoiceChip(
                    label: Text(scope.label),
                    selected: _controller.scope == scope,
                    onSelected: (_) => _changeScope(scope),
                  ),
                );
              })
              .toList(growable: false),
        ),
      ),
    );
  }

  List<Widget> _content(BuildContext context) {
    if (_controller.query.length < 2) {
      return const <Widget>[
        SliverFillRemaining(
          hasScrollBody: false,
          child: AppEmptyState(
            title: '输入至少 2 个字符开始搜索',
            description: '可以搜索课程、课评、社区内容、用户、板块和标签。',
          ),
        ),
      ];
    }
    if (_controller.isLoading) {
      return const <Widget>[
        SliverFillRemaining(
          hasScrollBody: false,
          child: AppLoadingState(
            title: '正在聚合搜索结果',
            description: '各业务域正在回表校验可见性。',
          ),
        ),
      ];
    }
    if (_controller.failure case final ApiFailure failure) {
      if (_controller.totalResults == 0) {
        return <Widget>[
          SliverFillRemaining(
            hasScrollBody: false,
            child: AppErrorState(
              title: '搜索暂时不可用',
              description: failure.message,
              onRetry: _controller.reload,
            ),
          ),
        ];
      }
    }
    if (_controller.failedScopes.isNotEmpty && _controller.totalResults == 0) {
      return <Widget>[
        SliverFillRemaining(
          hasScrollBody: false,
          child: AppErrorState(
            title: '相关搜索分类暂时不可用',
            description: '请稍后重试；未授权内容不会以降级结果返回。',
            onRetry: _controller.reload,
          ),
        ),
      ];
    }
    if (_controller.totalResults == 0) {
      return <Widget>[
        SliverFillRemaining(
          hasScrollBody: false,
          child: AppEmptyState(
            title: '没有找到“${_controller.query}”',
            description: '试试更短的关键词、课程代码、用户昵称或标签名。',
          ),
        ),
      ];
    }

    return <Widget>[
      SliverPadding(
        padding: const EdgeInsets.fromLTRB(16, 18, 16, 0),
        sliver: SliverToBoxAdapter(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 900),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                if (_controller.failedScopes.isNotEmpty)
                  _PartialSearchWarning(scopes: _controller.failedScopes),
                if (_controller.suggestedQuery case final String suggestion
                    when suggestion.toLowerCase() !=
                        _controller.query.toLowerCase()) ...<Widget>[
                  const SizedBox(height: 12),
                  Wrap(
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: <Widget>[
                      const Text('你是不是要搜索 '),
                      TextButton(
                        onPressed: () {
                          _queryController.text = suggestion;
                          _submit(suggestion);
                        },
                        child: Text('“$suggestion”'),
                      ),
                      const Text('？'),
                    ],
                  ),
                ],
                const SizedBox(height: 12),
                Text('共找到 ${_controller.totalResults} 条结果'),
                const SizedBox(height: 18),
                if (_show(SearchScope.course) && _controller.courses.isNotEmpty)
                  _section(
                    context,
                    scope: SearchScope.course,
                    icon: Icons.menu_book_outlined,
                    count: _controller.courses.length,
                    children: _controller.courses.map(_courseCard).toList(),
                  ),
                if (_show(SearchScope.review) && _controller.reviews.isNotEmpty)
                  _section(
                    context,
                    scope: SearchScope.review,
                    icon: Icons.rate_review_outlined,
                    count: _controller.reviews.length,
                    children: _controller.reviews.map(_reviewCard).toList(),
                  ),
                if (_show(SearchScope.thread) && _controller.threads.isNotEmpty)
                  _section(
                    context,
                    scope: SearchScope.thread,
                    icon: Icons.forum_outlined,
                    count: _controller.threads.length,
                    children: _controller.threads.map(_threadCard).toList(),
                  ),
                if (_show(SearchScope.user) && _controller.users.isNotEmpty)
                  _section(
                    context,
                    scope: SearchScope.user,
                    icon: Icons.people_outline_rounded,
                    count: _controller.users.length,
                    children: _controller.users.map(_userCard).toList(),
                  ),
                if (_show(SearchScope.board) && _controller.boards.isNotEmpty)
                  _section(
                    context,
                    scope: SearchScope.board,
                    icon: Icons.dashboard_outlined,
                    count: _controller.boards.length,
                    children: _controller.boards.map(_boardCard).toList(),
                  ),
                if (_show(SearchScope.tag) && _controller.tags.isNotEmpty)
                  _section(
                    context,
                    scope: SearchScope.tag,
                    icon: Icons.tag_rounded,
                    count: _controller.tags.length,
                    children: _controller.tags.map(_tagCard).toList(),
                  ),
                if (_controller.failure
                    case final ApiFailure failure) ...<Widget>[
                  _PartialSearchFailure(
                    message: failure.message,
                    onRetry: _controller.loadMore,
                  ),
                  const SizedBox(height: 12),
                ],
                if (_controller.scope != SearchScope.all && _controller.hasMore)
                  OutlinedButton.icon(
                    onPressed: _controller.isLoadingMore
                        ? null
                        : _controller.loadMore,
                    icon: _controller.isLoadingMore
                        ? const SizedBox.square(
                            dimension: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.expand_more_rounded),
                    label: Text(
                      _controller.isLoadingMore
                          ? '正在加载'
                          : '加载更多${_controller.scope.label}',
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    ];
  }

  bool _show(SearchScope scope) {
    return _controller.scope == SearchScope.all || _controller.scope == scope;
  }

  Widget _section(
    BuildContext context, {
    required SearchScope scope,
    required IconData icon,
    required int count,
    required List<Widget> children,
  }) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              Icon(icon, color: Theme.of(context).colorScheme.primary),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  scope.label,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ),
              Chip(label: Text('$count')),
            ],
          ),
          const SizedBox(height: 10),
          LayoutBuilder(
            builder: (BuildContext context, BoxConstraints constraints) {
              final bool useGrid =
                  constraints.maxWidth >= 680 &&
                  const <SearchScope>{
                    SearchScope.course,
                    SearchScope.user,
                    SearchScope.board,
                  }.contains(scope);
              if (!useGrid) {
                return Column(
                  children:
                      children
                          .expand(
                            (Widget child) => <Widget>[
                              child,
                              const SizedBox(height: 10),
                            ],
                          )
                          .toList()
                        ..removeLast(),
                );
              }
              final double width = (constraints.maxWidth - 10) / 2;
              return Wrap(
                spacing: 10,
                runSpacing: 10,
                children: children
                    .map((Widget child) => SizedBox(width: width, child: child))
                    .toList(growable: false),
              );
            },
          ),
          if (_controller.scope == SearchScope.all &&
              scope.generatedScope != null &&
              _controller.moreScopes.contains(
                scope.generatedScope,
              )) ...<Widget>[
            const SizedBox(height: 10),
            OutlinedButton(
              onPressed: () => _changeScope(scope),
              child: Text('查看更多${scope.label}'),
            ),
          ],
        ],
      ),
    );
  }

  Widget _courseCard(CourseSearchHit course) {
    return _ResultCard(
      onTap: () => context.push('/courses/${Uri.encodeComponent(course.id)}'),
      title: SafeHighlightedText(
        text: course.name,
        ranges: _ranges(
          SearchResultScope.course,
          course.id,
          SearchHighlightFieldEnum.name,
        ),
        style: Theme.of(context).textTheme.titleSmall,
      ),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Wrap(
            children: <Widget>[
              SafeHighlightedText(
                text: course.code,
                ranges: _ranges(
                  SearchResultScope.course,
                  course.id,
                  SearchHighlightFieldEnum.code,
                ),
              ),
              const Text(' · '),
              SafeHighlightedText(
                text: course.teacherName ?? '教师待同步',
                ranges: _ranges(
                  SearchResultScope.course,
                  course.id,
                  SearchHighlightFieldEnum.teacherName,
                ),
              ),
            ],
          ),
          const SizedBox(height: 5),
          SafeHighlightedText(
            text: course.department ?? '院系待同步',
            ranges: _ranges(
              SearchResultScope.course,
              course.id,
              SearchHighlightFieldEnum.department,
            ),
          ),
          Text('${course.reviewCount} 条课评 · ${_rating(course.reviewAvg)}'),
        ],
      ),
    );
  }

  Widget _reviewCard(ReviewSearchHit review) {
    return _ResultCard(
      onTap: () => context.push(
        Uri(
          path: '/courses/${Uri.encodeComponent(review.courseId)}',
          queryParameters: <String, String>{'review': review.id},
        ).toString(),
      ),
      title: Row(
        children: <Widget>[
          Expanded(
            child: SafeHighlightedText(
              text: review.courseName,
              ranges: _ranges(
                SearchResultScope.review,
                review.id,
                SearchHighlightFieldEnum.courseName,
              ),
              style: Theme.of(context).textTheme.titleSmall,
            ),
          ),
          Chip(label: Text('${review.rating} 星')),
        ],
      ),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          SafeHighlightedText(
            text: review.comment ?? '该课评没有文字内容',
            ranges: _ranges(
              SearchResultScope.review,
              review.id,
              SearchHighlightFieldEnum.comment,
            ),
            maxLines: 3,
            overflow: TextOverflow.ellipsis,
          ),
          const SizedBox(height: 5),
          Text(
            '${review.approveCount} 人赞同 · ${_formatUnixTime(review.createdAt)}',
          ),
        ],
      ),
    );
  }

  Widget _threadCard(ThreadSearchHit thread) {
    return _ResultCard(
      onTap: () =>
          context.push('/forum/threads/${Uri.encodeComponent(thread.id)}'),
      title: SafeHighlightedText(
        text: thread.title,
        ranges: _ranges(
          SearchResultScope.thread,
          thread.id,
          SearchHighlightFieldEnum.title,
        ),
        style: Theme.of(context).textTheme.titleSmall,
      ),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          if (thread.bodyExcerpt.isNotEmpty)
            SafeHighlightedText(
              text: thread.bodyExcerpt,
              ranges: _ranges(
                SearchResultScope.thread,
                thread.id,
                SearchHighlightFieldEnum.bodyExcerpt,
              ),
              maxLines: 3,
              overflow: TextOverflow.ellipsis,
            ),
          const SizedBox(height: 5),
          Text(
            '${thread.authorHandle} · ${thread.board} · ${thread.replyCount} 条回复',
          ),
        ],
      ),
    );
  }

  Widget _userCard(UserSearchHit user) {
    final String display = user.displayName ?? user.handle;
    return _ResultCard(
      onTap: () => context.push('/profile/${Uri.encodeComponent(user.handle)}'),
      leading: PlatformAvatar(
        compatibilityUrl: user.avatarUrl,
        fallbackText: user.handle,
        semanticLabel: '${user.handle} 的头像',
        onRefresh: () => unawaited(_controller.reload()),
      ),
      title: SafeHighlightedText(
        text: display,
        ranges: _ranges(
          SearchResultScope.user,
          user.id,
          user.displayName == null
              ? SearchHighlightFieldEnum.handle
              : SearchHighlightFieldEnum.displayName,
        ),
        style: Theme.of(context).textTheme.titleSmall,
      ),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Wrap(
            children: <Widget>[
              const Text('@'),
              SafeHighlightedText(
                text: user.handle,
                ranges: _ranges(
                  SearchResultScope.user,
                  user.id,
                  SearchHighlightFieldEnum.handle,
                ),
              ),
            ],
          ),
          Text('${user.followerCount} 位关注者${user.following ? ' · 已关注' : ''}'),
        ],
      ),
    );
  }

  Widget _boardCard(BoardSearchHit board) {
    return _ResultCard(
      onTap: () => context.go(
        Uri(
          path: '/forum',
          queryParameters: <String, String>{'board': board.id},
        ).toString(),
      ),
      title: SafeHighlightedText(
        text: board.name,
        ranges: _ranges(
          SearchResultScope.board,
          board.id,
          SearchHighlightFieldEnum.name,
        ),
        style: Theme.of(context).textTheme.titleSmall,
      ),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          SafeHighlightedText(
            text: board.description ?? '浏览该板块的公开讨论',
            ranges: _ranges(
              SearchResultScope.board,
              board.id,
              SearchHighlightFieldEnum.description,
            ),
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
          ),
          Text('${board.threadCount} 个帖子'),
        ],
      ),
    );
  }

  Widget _tagCard(TagSearchHit tag) {
    return _ResultCard(
      onTap: () => context.go(
        Uri(
          path: '/forum',
          queryParameters: <String, String>{'tag': tag.slug},
        ).toString(),
      ),
      title: Wrap(
        children: <Widget>[
          const Text('#'),
          SafeHighlightedText(
            text: tag.name,
            ranges: _ranges(
              SearchResultScope.tag,
              tag.id,
              SearchHighlightFieldEnum.name,
            ),
            style: Theme.of(context).textTheme.titleSmall,
          ),
        ],
      ),
      subtitle: Text('${tag.threadCount} 个帖子'),
    );
  }

  List<SearchHighlightRange> _ranges(
    SearchResultScope scope,
    String id,
    SearchHighlightFieldEnum field,
  ) {
    return _controller.rangesFor(scope: scope, id: id, field: field);
  }

  void _submit(String query) {
    final String normalized = query.trim();
    _controller.submit(normalized);
    _replaceLocation(normalized, _controller.scope);
  }

  void _changeScope(SearchScope scope) {
    _controller.setScope(scope);
    _replaceLocation(_controller.query, scope);
  }

  void _replaceLocation(String query, SearchScope scope) {
    final Map<String, String> parameters = <String, String>{};
    if (query.isNotEmpty) {
      parameters['q'] = query;
    }
    if (scope != SearchScope.all) {
      parameters['type'] = scope.wireValue;
    }
    context.replace(
      Uri(path: '/search', queryParameters: parameters).toString(),
    );
  }
}

class _ResultCard extends StatelessWidget {
  const _ResultCard({
    required this.onTap,
    required this.title,
    required this.subtitle,
    this.leading,
  });

  final VoidCallback onTap;
  final Widget title;
  final Widget subtitle;
  final Widget? leading;

  @override
  Widget build(BuildContext context) {
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(14),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              if (leading != null) ...<Widget>[
                leading!,
                const SizedBox(width: 12),
              ],
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    title,
                    const SizedBox(height: 7),
                    DefaultTextStyle.merge(
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                      child: subtitle,
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              const Icon(Icons.chevron_right_rounded),
            ],
          ),
        ),
      ),
    );
  }
}

class _PartialSearchWarning extends StatelessWidget {
  const _PartialSearchWarning({required this.scopes});

  final Set<SearchResultScope> scopes;

  @override
  Widget build(BuildContext context) {
    final String labels = scopes.map(_scopeLabel).join('、');
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.tertiaryContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          children: <Widget>[
            const Icon(Icons.warning_amber_rounded),
            const SizedBox(width: 8),
            Expanded(child: Text('$labels 暂时不可用，其余结果仍可查看。')),
          ],
        ),
      ),
    );
  }
}

class _PartialSearchFailure extends StatelessWidget {
  const _PartialSearchFailure({required this.message, required this.onRetry});

  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.errorContainer,
        borderRadius: BorderRadius.circular(12),
      ),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          children: <Widget>[
            Expanded(child: Text('继续加载失败：$message')),
            TextButton(onPressed: onRetry, child: const Text('重试')),
          ],
        ),
      ),
    );
  }
}

String _scopeLabel(SearchResultScope scope) {
  return switch (scope) {
    SearchResultScope.course => '课程与教师',
    SearchResultScope.review => '课评',
    SearchResultScope.thread => '社区帖子',
    SearchResultScope.user => '用户',
    SearchResultScope.board => '板块',
    SearchResultScope.tag => '标签',
    SearchResultScope.unknownDefaultOpenApi => '未知分类',
  };
}

String _rating(num? value) =>
    value == null ? '暂无评分' : '${value.toStringAsFixed(1)} 分';

String _formatUnixTime(int seconds) {
  final DateTime date = DateTime.fromMillisecondsSinceEpoch(
    seconds * 1000,
    isUtc: true,
  ).toLocal();
  return '${date.year}-${date.month.toString().padLeft(2, '0')}-'
      '${date.day.toString().padLeft(2, '0')}';
}
