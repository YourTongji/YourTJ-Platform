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
import '../../forum/data/forum_repository.dart';
import '../../forum/presentation/forum_widgets.dart';
import '../data/home_repository.dart';
import '../domain/promotion_navigation.dart';
import '../domain/promotion_presentation.dart';
import 'promotion_impression_tracker.dart';

class HomePage extends ConsumerStatefulWidget {
  const HomePage({super.key});

  @override
  ConsumerState<HomePage> createState() => _HomePageState();
}

class _HomePageState extends ConsumerState<HomePage> {
  ForumFeed _feed = ForumFeed.hot;
  List<ThreadFeed> _threads = <ThreadFeed>[];
  List<Board> _boards = <Board>[];
  List<Announcement> _announcements = <Announcement>[];
  List<Promotion> _promotions = <Promotion>[];
  HomeGrowth? _growth;
  String? _nextCursor;
  bool _hasMore = false;
  bool _isLoading = true;
  bool _isLoadingMore = false;
  bool _isLoadingGrowth = false;
  bool _isCheckingIn = false;
  ApiFailure? _error;
  ApiFailure? _growthError;
  String? _loadedAccountId;
  bool _hasObservedAccount = false;
  int _feedGeneration = 0;

  ForumRepository get _forum => ref.read(forumRepositoryProvider);
  HomeRepository get _home => ref.read(homeRepositoryProvider);

  @override
  void initState() {
    super.initState();
    _loadPublic();
  }

  Future<void> _loadPublic() async {
    final int generation = ++_feedGeneration;
    setState(() {
      _isLoading = true;
      _error = null;
    });
    try {
      final List<Object> results = await Future.wait<Object>(<Future<Object>>[
        _forum.threads(feed: _feed),
        _forum.boards(),
        _optional(_home.announcements()),
        _optional(_home.promotions()),
      ]);
      if (!mounted || generation != _feedGeneration) {
        return;
      }
      final ForumPageSlice<ThreadFeed> page =
          results[0] as ForumPageSlice<ThreadFeed>;
      setState(() {
        _threads = page.items;
        _nextCursor = page.nextCursor;
        _hasMore = page.hasMore;
        _boards = results[1] as List<Board>;
        _announcements = results[2] as List<Announcement>;
        _promotions = results[3] as List<Promotion>;
      });
    } on ApiFailure catch (failure) {
      if (mounted && generation == _feedGeneration) {
        setState(() => _error = failure);
      }
    } finally {
      if (mounted && generation == _feedGeneration) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<List<T>> _optional<T>(Future<List<T>> request) async {
    try {
      return await request;
    } on ApiFailure {
      return <T>[];
    }
  }

  void _recordPromotionImpression(Promotion promotion) {
    unawaited(
      _home
          .recordPromotionEvent(
            promotion: promotion,
            eventType: PromotionEventInputEventTypeEnum.impression,
          )
          .onError((Object _, StackTrace _) {}),
    );
  }

  void _openPromotion(Promotion promotion) {
    final String? location = PromotionNavigation.internalLocation(
      promotion.targetUrl,
    );
    if (location == null) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('该推广目标不是受支持的站内页面')));
      return;
    }
    unawaited(
      _home
          .recordPromotionEvent(
            promotion: promotion,
            eventType: PromotionEventInputEventTypeEnum.click,
          )
          .onError((Object _, StackTrace _) {}),
    );
    context.go(location);
  }

  Future<void> _loadGrowth() async {
    if (_isLoadingGrowth) {
      return;
    }
    setState(() {
      _isLoadingGrowth = true;
      _growthError = null;
    });
    try {
      final HomeGrowth growth = await _home.growth();
      if (mounted) {
        setState(() => _growth = growth);
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _growthError = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isLoadingGrowth = false);
      }
    }
  }

  Future<void> _selectFeed(ForumFeed feed, bool authenticated) async {
    if (feed.requiresAuthentication && !authenticated) {
      await context.push(
        publicInteractionLoginLocation(GoRouterState.of(context).uri),
      );
      if (!mounted ||
          !(ref.read(sessionStateProvider).value?.isAuthenticated ?? false)) {
        return;
      }
    }
    if (feed == _feed) {
      return;
    }
    setState(() => _feed = feed);
    await _loadPublic();
  }

  Future<void> _loadMore() async {
    if (_isLoadingMore || !_hasMore || _nextCursor == null) {
      return;
    }
    setState(() => _isLoadingMore = true);
    try {
      final ForumPageSlice<ThreadFeed> page = await _forum.threads(
        feed: _feed,
        cursor: _nextCursor,
      );
      if (mounted) {
        final Set<String> known = _threads
            .map((ThreadFeed thread) => thread.id)
            .toSet();
        setState(() {
          _threads.addAll(
            page.items.where((ThreadFeed thread) => known.add(thread.id)),
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

  Future<void> _checkIn() async {
    if (_isCheckingIn) {
      return;
    }
    setState(() => _isCheckingIn = true);
    try {
      final CheckInStatus status = await _home.checkIn();
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            status.newlyCheckedIn
                ? '签到成功，已连续 ${status.currentStreak} 天'
                : '今天已经签到',
          ),
        ),
      );
      await _loadGrowth();
    } on ApiFailure catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(failure.message)));
      }
    } finally {
      if (mounted) {
        setState(() => _isCheckingIn = false);
      }
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
    final String? accountId = sessionState?.account?.id;
    if (!_hasObservedAccount || accountId != _loadedAccountId) {
      _hasObservedAccount = true;
      _loadedAccountId = accountId;
      WidgetsBinding.instance.addPostFrameCallback((Duration _) {
        if (!mounted) {
          return;
        }
        if (accountId == null) {
          final bool mustResetFeed = _feed.requiresAuthentication;
          setState(() {
            _growth = null;
            _growthError = null;
            if (mustResetFeed) {
              _feed = ForumFeed.hot;
            }
          });
          unawaited(_loadPublic());
        } else {
          unawaited(_loadGrowth());
          unawaited(_loadPublic());
        }
      });
    }
    return RefreshIndicator(
      onRefresh: () async {
        await _loadPublic();
        if (authenticated) {
          await _loadGrowth();
        }
      },
      child: CustomScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        slivers: <Widget>[
          const SliverAppBar(pinned: true, title: Text('YourTJ')),
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
            sliver: SliverToBoxAdapter(
              child: _GrowthCard(
                authenticated: authenticated,
                growth: _growth,
                isLoading: _isLoadingGrowth,
                isCheckingIn: _isCheckingIn,
                error: _growthError,
                onLogin: () => context.push(
                  publicInteractionLoginLocation(GoRouterState.of(context).uri),
                ),
                onRetry: _loadGrowth,
                onCheckIn: _checkIn,
              ),
            ),
          ),
          if (_announcements.isNotEmpty)
            SliverPadding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
              sliver: SliverToBoxAdapter(
                child: Card(
                  child: ExpansionTile(
                    leading: const Icon(Icons.campaign_outlined),
                    title: Text(_announcements.first.title),
                    subtitle: Text('${_announcements.length} 条当前公告'),
                    children: _announcements
                        .take(5)
                        .map(
                          (Announcement announcement) => ListTile(
                            title: Text(announcement.title),
                            subtitle: announcement.body == null
                                ? null
                                : Text(
                                    announcement.body!,
                                    maxLines: 3,
                                    overflow: TextOverflow.ellipsis,
                                  ),
                          ),
                        )
                        .toList(),
                  ),
                ),
              ),
            ),
          if (_promotions.isNotEmpty)
            SliverPadding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
              sliver: SliverToBoxAdapter(
                child: _PromotionsPanel(
                  promotions: _promotions,
                  onImpression: _recordPromotionImpression,
                  onOpen: _openPromotion,
                  onRefreshDelivery: _loadPublic,
                ),
              ),
            ),
          SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 10),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    '校园动态',
                    style: Theme.of(context).textTheme.titleLarge?.copyWith(
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                  const SizedBox(height: 10),
                  SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: Row(
                      children: ForumFeed.values
                          .where((ForumFeed feed) => feed != ForumFeed.unread)
                          .map(
                            (ForumFeed feed) => Padding(
                              padding: const EdgeInsets.only(right: 8),
                              child: ChoiceChip(
                                selected: feed == _feed,
                                onSelected: (_) =>
                                    _selectFeed(feed, authenticated),
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
                ],
              ),
            ),
          ),
          if (_isLoading)
            const SliverFillRemaining(child: AppLoadingState(title: '加载校园动态'))
          else if (_error case final ApiFailure failure)
            SliverFillRemaining(
              child: AppErrorState(
                description: failure.message,
                onRetry: _loadPublic,
              ),
            )
          else if (_threads.isEmpty)
            const SliverFillRemaining(child: AppEmptyState(title: '暂无校园动态'))
          else ...<Widget>[
            SliverPadding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              sliver: SliverList.separated(
                itemCount: _threads.length,
                itemBuilder: (BuildContext context, int index) {
                  final ThreadFeed thread = _threads[index];
                  return ForumThreadCard(
                    thread: thread,
                    boardName: _boardName(thread.boardId),
                    onRefreshDelivery: _loadPublic,
                  );
                },
                separatorBuilder: (BuildContext context, int index) =>
                    const SizedBox(height: 10),
              ),
            ),
            SliverToBoxAdapter(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 32),
                child: _hasMore
                    ? OutlinedButton.icon(
                        onPressed: _isLoadingMore ? null : _loadMore,
                        icon: _isLoadingMore
                            ? const SizedBox.square(
                                dimension: 18,
                                child: CircularProgressIndicator(
                                  strokeWidth: 2,
                                ),
                              )
                            : const Icon(Icons.expand_more_rounded),
                        label: Text(_isLoadingMore ? '加载中' : '加载更多'),
                      )
                    : const Center(child: Text('已经到底了')),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _PromotionsPanel extends StatelessWidget {
  const _PromotionsPanel({
    required this.promotions,
    required this.onImpression,
    required this.onOpen,
    required this.onRefreshDelivery,
  });

  final List<Promotion> promotions;
  final ValueChanged<Promotion> onImpression;
  final ValueChanged<Promotion> onOpen;
  final VoidCallback onRefreshDelivery;

  @override
  Widget build(BuildContext context) {
    final List<Promotion> visible = promotions.take(2).toList(growable: false);
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) {
        final List<Widget> cards = visible
            .map(
              (Promotion promotion) => _PromotionCard(
                promotion: promotion,
                onImpression: () => onImpression(promotion),
                onOpen: () => onOpen(promotion),
                onRefreshDelivery: onRefreshDelivery,
              ),
            )
            .toList(growable: false);
        if (constraints.maxWidth >= 720 && cards.length > 1) {
          return Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Expanded(child: cards[0]),
              const SizedBox(width: 12),
              Expanded(child: cards[1]),
            ],
          );
        }
        return Column(
          children: cards
              .map(
                (Widget card) => Padding(
                  padding: const EdgeInsets.only(bottom: 10),
                  child: card,
                ),
              )
              .toList(growable: false),
        );
      },
    );
  }
}

class _PromotionCard extends StatefulWidget {
  const _PromotionCard({
    required this.promotion,
    required this.onImpression,
    required this.onOpen,
    required this.onRefreshDelivery,
  });

  final Promotion promotion;
  final VoidCallback onImpression;
  final VoidCallback onOpen;
  final VoidCallback onRefreshDelivery;

  @override
  State<_PromotionCard> createState() => _PromotionCardState();
}

class _PromotionCardState extends State<_PromotionCard> {
  bool _requestedDeliveryRefresh = false;

  @override
  void didUpdateWidget(covariant _PromotionCard oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.promotion.trackingToken != widget.promotion.trackingToken) {
      _requestedDeliveryRefresh = false;
    }
  }

  void _refreshDelivery() {
    if (_requestedDeliveryRefresh) {
      return;
    }
    _requestedDeliveryRefresh = true;
    WidgetsBinding.instance.addPostFrameCallback((Duration _) {
      if (mounted) {
        widget.onRefreshDelivery();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final MediaDelivery? delivery = widget.promotion.assetDelivery;
    final int now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    final Uri? deliveryUri = delivery == null
        ? null
        : PromotionPresentation.freshImageUri(delivery, now: now);
    if (delivery != null && deliveryUri == null) {
      _refreshDelivery();
    }
    return PromotionImpressionTracker(
      trackingToken: widget.promotion.trackingToken,
      onImpression: widget.onImpression,
      child: Card(
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: widget.onOpen,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              if (deliveryUri != null)
                AspectRatio(
                  aspectRatio: delivery!.width / delivery.height,
                  child: Image.network(
                    deliveryUri.toString(),
                    fit: BoxFit.cover,
                    semanticLabel: widget.promotion.title,
                    errorBuilder:
                        (
                          BuildContext context,
                          Object error,
                          StackTrace? stack,
                        ) {
                          _refreshDelivery();
                          return const ColoredBox(
                            color: Color(0x11000000),
                            child: Center(
                              child: Icon(Icons.image_not_supported_outlined),
                            ),
                          );
                        },
                  ),
                ),
              Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    const Text('YourTJ 站内推广'),
                    const SizedBox(height: 6),
                    Text(
                      widget.promotion.title,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    if (widget.promotion.body
                        case final String body) ...<Widget>[
                      const SizedBox(height: 6),
                      Text(body),
                    ],
                    const SizedBox(height: 10),
                    Align(
                      alignment: Alignment.centerRight,
                      child: FilledButton.tonalIcon(
                        onPressed: widget.onOpen,
                        icon: const Icon(Icons.arrow_forward_rounded),
                        label: Text(widget.promotion.ctaLabel ?? '查看详情'),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _GrowthCard extends StatelessWidget {
  const _GrowthCard({
    required this.authenticated,
    required this.growth,
    required this.isLoading,
    required this.isCheckingIn,
    required this.error,
    required this.onLogin,
    required this.onRetry,
    required this.onCheckIn,
  });

  final bool authenticated;
  final HomeGrowth? growth;
  final bool isLoading;
  final bool isCheckingIn;
  final ApiFailure? error;
  final VoidCallback onLogin;
  final VoidCallback onRetry;
  final VoidCallback onCheckIn;

  @override
  Widget build(BuildContext context) {
    if (!authenticated) {
      return Card(
        child: Padding(
          padding: const EdgeInsets.all(18),
          child: Row(
            children: <Widget>[
              const Icon(Icons.local_cafe_outlined, size: 36),
              const SizedBox(width: 14),
              const Expanded(child: Text('登录后签到，并查看活跃记录与茶等级成长。')),
              FilledButton.tonal(onPressed: onLogin, child: const Text('登录')),
            ],
          ),
        ),
      );
    }
    if (isLoading && growth == null) {
      return const Card(
        child: Padding(
          padding: EdgeInsets.all(24),
          child: Center(child: CircularProgressIndicator()),
        ),
      );
    }
    if (error != null && growth == null) {
      return Card(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: <Widget>[
              const Icon(Icons.error_outline_rounded),
              const SizedBox(width: 12),
              Expanded(child: Text(error!.message)),
              TextButton(onPressed: onRetry, child: const Text('重试')),
            ],
          ),
        ),
      );
    }
    final HomeGrowth? current = growth;
    if (current == null) {
      return const SizedBox.shrink();
    }
    final TrustProgress trust = current.trust;
    final CheckInStatus checkIn = current.checkIn;
    final List<ActivityDay> recentDays = current.activity.days.length > 28
        ? current.activity.days.sublist(current.activity.days.length - 28)
        : current.activity.days;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              children: <Widget>[
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        '每日签到',
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      Text(
                        '连续 ${checkIn.currentStreak} 天 · 累计 ${checkIn.totalDays} 天',
                      ),
                    ],
                  ),
                ),
                FilledButton.icon(
                  onPressed: checkIn.checkedIn || isCheckingIn
                      ? null
                      : onCheckIn,
                  icon: isCheckingIn
                      ? const SizedBox.square(
                          dimension: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : Icon(
                          checkIn.checkedIn
                              ? Icons.check_circle_rounded
                              : Icons.calendar_today_outlined,
                        ),
                  label: Text(checkIn.checkedIn ? '今日已签' : '签到'),
                ),
              ],
            ),
            const Divider(height: 28),
            Text(
              '茶等级 ${trust.trustLevel} · ${trust.teaName}',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const SizedBox(height: 8),
            LinearProgressIndicator(value: trust.progressPercent / 100),
            const SizedBox(height: 6),
            Text(
              trust.isMaxLevel
                  ? '已达到当前最高等级'
                  : '当前 ${trust.qualifyingScore} 分，距离下一级还需 ${trust.remainingScore ?? 0} 分',
            ),
            const SizedBox(height: 16),
            Text('最近活跃', style: Theme.of(context).textTheme.labelLarge),
            const SizedBox(height: 8),
            Wrap(
              spacing: 4,
              runSpacing: 4,
              children: recentDays.map((ActivityDay day) {
                final double intensity = day.score == 0
                    ? 0.08
                    : (0.2 + day.score / 20).clamp(0.2, 1.0);
                return Tooltip(
                  message:
                      '${day.date.toIso8601String().substring(0, 10)} · ${day.score} 分',
                  child: Container(
                    width: 16,
                    height: 16,
                    decoration: BoxDecoration(
                      color: Theme.of(
                        context,
                      ).colorScheme.primary.withValues(alpha: intensity),
                      borderRadius: BorderRadius.circular(3),
                    ),
                  ),
                );
              }).toList(),
            ),
          ],
        ),
      ),
    );
  }
}
