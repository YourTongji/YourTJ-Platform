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
import '../../captcha/presentation/captcha_dialog.dart';
import '../../reviews/data/reviews_repository.dart';
import '../../reviews/presentation/review_card.dart';
import '../../reviews/presentation/review_composer.dart';
import '../data/courses_repository.dart';
import '../domain/course_detail_controller.dart';

class CourseDetailPage extends ConsumerStatefulWidget {
  const CourseDetailPage({
    required this.courseId,
    this.targetReviewId,
    super.key,
  });

  final String courseId;
  final String? targetReviewId;

  @override
  ConsumerState<CourseDetailPage> createState() => _CourseDetailPageState();
}

class _CourseDetailPageState extends ConsumerState<CourseDetailPage> {
  late CourseDetailController _controller;
  final GlobalKey _targetReviewKey = GlobalKey();
  (int, SessionPhase, String?)? _sessionIdentity;
  String? _scheduledTargetReviewId;
  String? _scrolledTargetReviewId;

  @override
  void initState() {
    super.initState();
    _controller = _newController();
    ref.listenManual<AsyncValue<SessionState>>(
      sessionStateProvider,
      _handleSessionState,
      fireImmediately: true,
    );
    unawaited(_controller.initialize());
  }

  CourseDetailController _newController() {
    return CourseDetailController(
      courseId: widget.courseId,
      targetReviewId: widget.targetReviewId,
      courseSource: ref.read(coursesRepositoryProvider),
      reviewSource: ref.read(reviewsRepositoryProvider),
    );
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
    if (_sessionIdentity == null) {
      _sessionIdentity = identity;
      return;
    }
    if (_sessionIdentity == identity) {
      return;
    }
    _sessionIdentity = identity;
    final CourseDetailController controller = _controller;
    unawaited(
      Future.wait<void>(<Future<void>>[
        controller.reloadReviews(),
        controller.reloadTargetReview(),
      ]),
    );
  }

  @override
  void didUpdateWidget(CourseDetailPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.courseId == widget.courseId &&
        oldWidget.targetReviewId == widget.targetReviewId) {
      return;
    }
    _controller.dispose();
    _scheduledTargetReviewId = null;
    _scrolledTargetReviewId = null;
    _controller = _newController();
    unawaited(_controller.initialize());
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: _controller,
      builder: (BuildContext context, Widget? child) {
        if (_controller.isLoading && _controller.course == null) {
          return const AppLoadingState(
            title: '正在加载课程详情',
            description: '正在读取课程、点评与关联课程。',
          );
        }
        if (_controller.failure case final ApiFailure failure) {
          if (_controller.course == null) {
            return AppErrorState(
              description: failure.message,
              onRetry: _controller.reloadDetails,
            );
          }
        }
        final CourseDetail? course = _controller.course;
        if (course == null) {
          return const AppEmptyState(
            title: '课程不存在',
            description: '这门课程可能已合并、删除或暂不可见。',
          );
        }
        return LayoutBuilder(
          builder: (BuildContext context, BoxConstraints constraints) {
            return ListView(
              key: PageStorageKey<String>('course-${widget.courseId}'),
              padding: const EdgeInsets.fromLTRB(16, 16, 16, 36),
              children: <Widget>[
                ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 1120),
                  child: _courseHeader(context, course),
                ),
                const SizedBox(height: 16),
                if (constraints.maxWidth >= 900)
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Expanded(child: _mainColumn(context, course)),
                      const SizedBox(width: 18),
                      SizedBox(width: 320, child: _relatedCard(context)),
                    ],
                  )
                else ...<Widget>[
                  _mainColumn(context, course),
                  const SizedBox(height: 16),
                  _relatedCard(context),
                ],
              ],
            );
          },
        );
      },
    );
  }

  Widget _courseHeader(BuildContext context, CourseDetail course) {
    final String teachers = course.teachers?.isNotEmpty == true
        ? course.teachers!
              .map((Teacher teacher) => teacher.name)
              .whereType<String>()
              .join(' / ')
        : course.teacherName ?? '教师待同步';
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        IconButton(
          tooltip: '返回课程列表',
          onPressed: () =>
              context.canPop() ? context.pop() : context.go('/courses'),
          icon: const Icon(Icons.arrow_back_rounded),
        ),
        const SizedBox(width: 4),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                course.code ?? '课号待同步',
                style: Theme.of(context).textTheme.labelLarge?.copyWith(
                  color: Theme.of(context).colorScheme.primary,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                course.name ?? '课程详情',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
              const SizedBox(height: 6),
              Text('${course.department ?? '院系待同步'} · $teachers'),
            ],
          ),
        ),
        const SizedBox(width: 12),
        Chip(label: Text('${_number(course.credit)} 学分')),
      ],
    );
  }

  Widget _mainColumn(BuildContext context, CourseDetail course) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _statisticsCard(context, course),
        const SizedBox(height: 16),
        _summaryCard(context),
        const SizedBox(height: 16),
        ReviewComposer(
          isPublishing: _controller.isPublishing,
          onPublish:
              (ReviewDraft draft, String captchaToken, String idempotencyKey) =>
                  _controller.publish(
                    rating: draft.rating,
                    comment: draft.comment,
                    semester: draft.semester,
                    score: draft.score,
                    captchaToken: captchaToken,
                    idempotencyKey: idempotencyKey,
                  ),
        ),
        const SizedBox(height: 20),
        _reviewsSection(context),
      ],
    );
  }

  Widget _statisticsCard(BuildContext context, CourseDetail course) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Wrap(
          spacing: 32,
          runSpacing: 18,
          children: <Widget>[
            _Metric(
              label: '平均评分',
              value: course.reviewAvg == null
                  ? '暂无'
                  : course.reviewAvg!.toStringAsFixed(1),
            ),
            _Metric(label: '点评数', value: '${course.reviewCount ?? 0}'),
            SizedBox(
              width: 280,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Text(
                    '别名',
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 7),
                  if (course.aliases?.isNotEmpty == true)
                    Wrap(
                      spacing: 6,
                      runSpacing: 6,
                      children: course.aliases!
                          .map((String alias) => Chip(label: Text(alias)))
                          .toList(growable: false),
                    )
                  else
                    const Text('暂无'),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _summaryCard(BuildContext context) {
    if (_controller.summary case final AiSummary summary
        when summary.summary?.trim().isNotEmpty == true) {
      return Card(
        child: Padding(
          padding: const EdgeInsets.all(18),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                children: <Widget>[
                  Icon(
                    Icons.auto_awesome_rounded,
                    color: Theme.of(context).colorScheme.primary,
                  ),
                  const SizedBox(width: 8),
                  Text(
                    'AI 点评摘要',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ],
              ),
              const SizedBox(height: 4),
              Text(
                '${summary.model ?? 'model'} · ${_formatUnixTime(summary.updatedAt)}',
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(height: 12),
              Text(summary.summary!),
              const SizedBox(height: 10),
              Text(
                'AI 摘要仅概括现有点评，不替代你对课程的独立判断。',
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
          ),
        ),
      );
    }
    if (_controller.summaryFailure case final ApiFailure failure) {
      return _PartialFailureCard(
        title: 'AI 摘要暂不可用',
        message: failure.message,
        onRetry: _controller.reloadDetails,
      );
    }
    return const Card(
      child: Padding(
        padding: EdgeInsets.all(18),
        child: Text('这门课程暂时没有可展示的 AI 点评摘要。'),
      ),
    );
  }

  Widget _reviewsSection(BuildContext context) {
    final String? targetId = _controller.targetReview?.id;
    if (targetId != null && targetId == widget.targetReviewId) {
      _scheduleTargetReviewScroll(targetId);
    }
    final List<Review> listedReviews = _controller.reviews
        .where((Review review) => review.id != targetId)
        .toList(growable: false);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        if (widget.targetReviewId != null) ...<Widget>[
          KeyedSubtree(
            key: _targetReviewKey,
            child: _targetReviewSurface(context),
          ),
          const SizedBox(height: 20),
        ],
        Row(
          children: <Widget>[
            Expanded(
              child: Text(
                '课程点评',
                style: Theme.of(context).textTheme.titleLarge,
              ),
            ),
            SegmentedButton<String>(
              segments: const <ButtonSegment<String>>[
                ButtonSegment<String>(value: 'hot', label: Text('热门')),
                ButtonSegment<String>(value: 'new', label: Text('最新')),
              ],
              selected: <String>{_controller.reviewSort},
              onSelectionChanged: (Set<String> selection) {
                _controller.setReviewSort(selection.first);
              },
            ),
          ],
        ),
        const SizedBox(height: 12),
        if (_controller.areReviewsLoading)
          const SizedBox(
            height: 260,
            child: AppLoadingState(
              title: '正在加载点评',
              description: '正在读取这门课程的公开点评。',
            ),
          )
        else if (_controller.reviewsFailure case final ApiFailure failure)
          if (listedReviews.isEmpty)
            SizedBox(
              height: 260,
              child: AppErrorState(
                description: failure.message,
                onRetry: _controller.reloadReviews,
              ),
            )
          else
            _PartialFailureCard(
              title: '继续加载点评失败',
              message: failure.message,
              onRetry: _controller.loadMoreReviews,
            )
        else if (listedReviews.isEmpty)
          SizedBox(
            height: 240,
            child: AppEmptyState(
              title: targetId == null ? '还没有点评' : '没有更多点评',
              description: targetId == null
                  ? '成为第一个记录这门课体验的人。'
                  : '已在上方单独展示定位到的点评。',
            ),
          )
        else ...<Widget>[
          ...listedReviews.map(
            (Review review) => Padding(
              padding: const EdgeInsets.only(bottom: 10),
              child: _reviewCard(
                review,
                onRefreshAvatar: _controller.reloadReviews,
              ),
            ),
          ),
          if (_controller.reviewsHaveMore)
            Center(
              child: OutlinedButton.icon(
                onPressed: _controller.areMoreReviewsLoading
                    ? null
                    : _controller.loadMoreReviews,
                icon: _controller.areMoreReviewsLoading
                    ? const SizedBox.square(
                        dimension: 18,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.expand_more_rounded),
                label: Text(
                  _controller.areMoreReviewsLoading ? '正在加载' : '加载更多点评',
                ),
              ),
            ),
        ],
      ],
    );
  }

  void _scheduleTargetReviewScroll(String reviewId) {
    if (_scheduledTargetReviewId == reviewId ||
        _scrolledTargetReviewId == reviewId) {
      return;
    }
    _scheduledTargetReviewId = reviewId;
    WidgetsBinding.instance.addPostFrameCallback((Duration _) {
      if (!mounted ||
          _controller.targetReview?.id != reviewId ||
          widget.targetReviewId != reviewId) {
        _scheduledTargetReviewId = null;
        return;
      }
      final BuildContext? targetContext = _targetReviewKey.currentContext;
      if (targetContext == null) {
        _scheduledTargetReviewId = null;
        return;
      }
      unawaited(
        Scrollable.ensureVisible(
          targetContext,
          duration: const Duration(milliseconds: 280),
          curve: Curves.easeOutCubic,
          alignment: 0.08,
        ).whenComplete(() {
          if (!mounted || widget.targetReviewId != reviewId) {
            return;
          }
          _scheduledTargetReviewId = null;
          _scrolledTargetReviewId = reviewId;
        }),
      );
    });
  }

  Widget _targetReviewSurface(BuildContext context) {
    if (_controller.isTargetReviewLoading) {
      return const SizedBox(
        height: 200,
        child: AppLoadingState(title: '正在定位点评', description: '正在按精确点评编号读取内容。'),
      );
    }
    if (_controller.targetReviewFailure case final ApiFailure failure) {
      return _PartialFailureCard(
        title: '无法定位点评',
        message: failure.message,
        onRetry: _controller.reloadTargetReview,
      );
    }
    final Review? review = _controller.targetReview;
    if (review == null || review.courseId != widget.courseId) {
      return const SizedBox(
        height: 200,
        child: AppEmptyState(title: '无法定位点评', description: '这条点评不存在，或不属于当前课程。'),
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Text('定位点评', style: Theme.of(context).textTheme.titleLarge),
        const SizedBox(height: 12),
        _reviewCard(review, onRefreshAvatar: _controller.reloadTargetReview),
      ],
    );
  }

  Widget _reviewCard(Review review, {required VoidCallback onRefreshAvatar}) {
    final String reviewId = review.id ?? '';
    return ReviewCard(
      review: review,
      isBusy: _controller.isReviewBusy(reviewId),
      onLike: () => _likeReview(review),
      onEdit: () => _editReview(review),
      onReport: () => _reportReview(reviewId),
      onRefreshAvatar: onRefreshAvatar,
    );
  }

  Widget _relatedCard(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text('相关课程', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 10),
            if (_controller.relatedFailure case final ApiFailure failure)
              _PartialFailureCard(
                title: '关联课程暂不可用',
                message: failure.message,
                onRetry: _controller.reloadDetails,
              )
            else if (_controller.related.isEmpty)
              const Text('暂无相关课程')
            else
              ..._controller.related.map((Course course) {
                return ListTile(
                  contentPadding: EdgeInsets.zero,
                  title: Text(course.name ?? '未命名课程'),
                  subtitle: Text(
                    '${course.code ?? '课号待同步'} · '
                    '${course.reviewAvg?.toStringAsFixed(1) ?? '暂无评分'}',
                  ),
                  trailing: const Icon(Icons.chevron_right_rounded),
                  onTap: course.id == null
                      ? null
                      : () => context.push(
                          '/courses/${Uri.encodeComponent(course.id!)}',
                        ),
                );
              }),
          ],
        ),
      ),
    );
  }

  bool _isAuthenticated() {
    final SessionState state =
        ref.read(sessionStateProvider).value ??
        ref.read(sessionManagerProvider).state;
    if (state.isAuthenticated) {
      return true;
    }
    context.push(publicInteractionLoginLocation(GoRouterState.of(context).uri));
    return false;
  }

  Future<void> _likeReview(Review review) async {
    if (!_isAuthenticated()) {
      return;
    }
    try {
      await _controller.toggleLike(review);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text(review.viewerLiked ? '已取消赞同' : '已赞同这条点评')),
        );
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(failure.message)));
      }
    }
  }

  Future<void> _editReview(Review review) async {
    if (!_isAuthenticated()) {
      return;
    }
    final ReviewEditDraft? draft = await requestReviewEdit(context, review);
    if (!mounted || draft == null) {
      return;
    }
    try {
      await _controller.edit(
        reviewId: review.id ?? '',
        rating: draft.rating,
        comment: draft.comment,
        semester: draft.semester,
        score: draft.score,
      );
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('点评已更新')));
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(failure.message)));
      }
    }
  }

  Future<void> _reportReview(String reviewId) async {
    if (!_isAuthenticated()) {
      return;
    }
    final String? reason = await requestReviewReportReason(context);
    if (!mounted || reason == null) {
      return;
    }
    final String? captchaToken = await showCaptchaDialog(
      context: context,
      client: ref.read(captchaClientProvider),
    );
    if (!mounted || captchaToken == null) {
      return;
    }
    try {
      await _controller.report(
        reviewId: reviewId,
        reason: reason,
        captchaToken: captchaToken,
      );
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('举报已进入审核队列')));
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(failure.message)));
      }
    }
  }
}

class _Metric extends StatelessWidget {
  const _Metric({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 130,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(
            label,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 6),
          Text(value, style: Theme.of(context).textTheme.headlineMedium),
        ],
      ),
    );
  }
}

class _PartialFailureCard extends StatelessWidget {
  const _PartialFailureCard({
    required this.title,
    required this.message,
    required this.onRetry,
  });

  final String title;
  final String message;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Card(
      color: Theme.of(context).colorScheme.errorContainer,
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(title, style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            Text(message),
            const SizedBox(height: 8),
            TextButton.icon(
              onPressed: onRetry,
              icon: const Icon(Icons.refresh_rounded),
              label: const Text('重试'),
            ),
          ],
        ),
      ),
    );
  }
}

String _number(num? value) {
  if (value == null) {
    return '0';
  }
  return value % 1 == 0 ? value.toInt().toString() : value.toStringAsFixed(1);
}

String _formatUnixTime(int? seconds) {
  if (seconds == null) {
    return '更新时间待同步';
  }
  final DateTime date = DateTime.fromMillisecondsSinceEpoch(
    seconds * 1000,
    isUtc: true,
  ).toLocal();
  return '${date.year}-${date.month.toString().padLeft(2, '0')}-'
      '${date.day.toString().padLeft(2, '0')}';
}
