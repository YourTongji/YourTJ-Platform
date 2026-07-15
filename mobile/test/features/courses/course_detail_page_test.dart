import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/core/design/app_theme.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';
import 'package:yourtj_mobile/features/courses/data/courses_repository.dart';
import 'package:yourtj_mobile/features/courses/domain/course_models.dart';
import 'package:yourtj_mobile/features/courses/presentation/course_detail_page.dart';
import 'package:yourtj_mobile/features/reviews/data/reviews_repository.dart';
import 'package:yourtj_mobile/features/reviews/presentation/review_card.dart';

void main() {
  testWidgets(
    'replaces the exact review when the route target changes in place',
    (WidgetTester tester) async {
      final _ViewerAwareReviewsRepository reviews =
          _ViewerAwareReviewsRepository(includeListReview: false);
      final GlobalKey<_CourseDetailHarnessState> harnessKey =
          GlobalKey<_CourseDetailHarnessState>();

      await tester.pumpWidget(
        _testApp(
          reviews: reviews,
          child: _CourseDetailHarness(
            key: harnessKey,
            initialTargetReviewId: 'review-a',
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(reviews.requestedReviewIds, <String>['review-a']);
      expect(_reviewCards(tester).single.review.id, 'review-a');
      expect(find.text('review-a viewer 1'), findsOneWidget);
      _expectOnScreen(tester, find.byType(ReviewCard));

      harnessKey.currentState!.showReview('review-b');
      await tester.pumpAndSettle();

      expect(reviews.requestedReviewIds, <String>['review-a', 'review-b']);
      expect(_reviewCards(tester).single.review.id, 'review-b');
      expect(find.text('review-a viewer 1'), findsNothing);
      expect(find.text('review-b viewer 1'), findsOneWidget);
      _expectOnScreen(tester, find.byType(ReviewCard));
    },
  );

  testWidgets('account generation change reloads list and exact review flags', (
    WidgetTester tester,
  ) async {
    final StreamController<SessionState> sessions =
        StreamController<SessionState>();
    final _ViewerAwareReviewsRepository reviews =
        _ViewerAwareReviewsRepository();
    addTearDown(sessions.close);
    sessions.add(
      SessionState.authenticated(generation: 1, account: _account('account-a')),
    );

    await tester.pumpWidget(
      _testApp(
        reviews: reviews,
        sessionStream: sessions.stream,
        child: const CourseDetailPage(
          courseId: 'course-1',
          targetReviewId: 'target-review',
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(reviews.listCalls, 1);
    expect(reviews.requestedReviewIds, <String>['target-review']);
    final Map<String, Review> firstViewer = _reviewsById(tester);
    expect(firstViewer['list-review']?.viewerLiked, isTrue);
    expect(firstViewer['list-review']?.canEdit, isTrue);
    expect(firstViewer['list-review']?.canReport, isFalse);
    expect(firstViewer['target-review']?.viewerLiked, isTrue);
    expect(firstViewer['target-review']?.canEdit, isTrue);
    expect(firstViewer['target-review']?.canReport, isFalse);

    reviews.viewerRevision = 2;
    sessions.add(
      SessionState.authenticated(generation: 2, account: _account('account-b')),
    );
    await tester.pumpAndSettle();

    expect(reviews.listCalls, 2);
    expect(reviews.requestedReviewIds, <String>[
      'target-review',
      'target-review',
    ]);
    expect(find.text('list-review viewer 1'), findsNothing);
    expect(find.text('target-review viewer 1'), findsNothing);
    final Map<String, Review> secondViewer = _reviewsById(tester);
    expect(secondViewer['list-review']?.viewerLiked, isFalse);
    expect(secondViewer['list-review']?.canEdit, isFalse);
    expect(secondViewer['list-review']?.canReport, isTrue);
    expect(secondViewer['target-review']?.viewerLiked, isFalse);
    expect(secondViewer['target-review']?.canEdit, isFalse);
    expect(secondViewer['target-review']?.canReport, isTrue);
  });
}

Widget _testApp({
  required _ViewerAwareReviewsRepository reviews,
  required Widget child,
  Stream<SessionState>? sessionStream,
}) {
  return ProviderScope(
    overrides: [
      coursesRepositoryProvider.overrideWithValue(_FakeCoursesRepository()),
      reviewsRepositoryProvider.overrideWithValue(reviews),
      sessionStateProvider.overrideWith(
        (Ref ref) =>
            sessionStream ??
            Stream<SessionState>.value(
              const SessionState.anonymous(generation: 1),
            ),
      ),
    ],
    child: MaterialApp(
      theme: AppTheme.light,
      home: Scaffold(body: child),
    ),
  );
}

List<ReviewCard> _reviewCards(WidgetTester tester) {
  return tester
      .widgetList<ReviewCard>(find.byType(ReviewCard))
      .toList(growable: false);
}

Map<String, Review> _reviewsById(WidgetTester tester) {
  return <String, Review>{
    for (final ReviewCard card in _reviewCards(tester))
      if (card.review.id case final String id) id: card.review,
  };
}

void _expectOnScreen(WidgetTester tester, Finder finder) {
  final Rect bounds = tester.getRect(finder);
  final Size surfaceSize =
      tester.view.physicalSize / tester.view.devicePixelRatio;
  expect(bounds.top, greaterThanOrEqualTo(0));
  expect(bounds.bottom, lessThanOrEqualTo(surfaceSize.height));
}

class _CourseDetailHarness extends StatefulWidget {
  const _CourseDetailHarness({required this.initialTargetReviewId, super.key});

  final String initialTargetReviewId;

  @override
  State<_CourseDetailHarness> createState() => _CourseDetailHarnessState();
}

class _CourseDetailHarnessState extends State<_CourseDetailHarness> {
  late String _targetReviewId = widget.initialTargetReviewId;

  void showReview(String reviewId) {
    setState(() => _targetReviewId = reviewId);
  }

  @override
  Widget build(BuildContext context) {
    return CourseDetailPage(
      courseId: 'course-1',
      targetReviewId: _targetReviewId,
    );
  }
}

class _FakeCoursesRepository implements CoursesRepository {
  @override
  Future<AiSummary?> aiSummary(
    String courseId, {
    CancelToken? cancelToken,
  }) async => null;

  @override
  Future<CoursePageSlice> browse({
    required String sort,
    String? departmentId,
    String? cursor,
    CancelToken? cancelToken,
  }) {
    throw UnimplementedError();
  }

  @override
  Future<List<Department>> departments({CancelToken? cancelToken}) {
    throw UnimplementedError();
  }

  @override
  Future<CourseDetail> detail(
    String courseId, {
    CancelToken? cancelToken,
  }) async {
    return CourseDetail(
      id: courseId,
      code: 'CS101',
      name: '程序设计',
      department: '计算机',
      credit: 3,
      reviewCount: 2,
      reviewAvg: 4.5,
    );
  }

  @override
  Future<List<Course>> related(
    String courseId, {
    CancelToken? cancelToken,
  }) async => const <Course>[];

  @override
  Future<CoursePageSlice> search({
    required String query,
    String? cursor,
    CancelToken? cancelToken,
  }) {
    throw UnimplementedError();
  }
}

class _ViewerAwareReviewsRepository implements ReviewsRepository {
  _ViewerAwareReviewsRepository({this.includeListReview = true});

  final bool includeListReview;
  final List<String> requestedReviewIds = <String>[];
  int listCalls = 0;
  int viewerRevision = 1;

  @override
  Future<Review> edit({
    required String reviewId,
    required int rating,
    String? comment,
    String? semester,
    String? score,
  }) async => _review(reviewId);

  @override
  Future<Review> get(String reviewId, {CancelToken? cancelToken}) async {
    requestedReviewIds.add(reviewId);
    return _review(reviewId);
  }

  @override
  Future<void> like(String reviewId) async {}

  @override
  Future<ReviewPageSlice> list({
    required String courseId,
    required String sort,
    String? cursor,
    CancelToken? cancelToken,
  }) async {
    listCalls += 1;
    return ReviewPageSlice(
      items: includeListReview ? <Review>[_review('list-review')] : <Review>[],
      nextCursor: null,
      hasMore: false,
    );
  }

  @override
  Future<Review> publish({
    required String courseId,
    required int rating,
    required String captchaToken,
    required String idempotencyKey,
    String? comment,
    String? semester,
    String? score,
  }) async => _review('published-review');

  @override
  Future<void> report({
    required String reviewId,
    required String reason,
    required String captchaToken,
  }) async {}

  @override
  Future<void> unlike(String reviewId) async {}

  Review _review(String id) {
    final bool firstViewer = viewerRevision == 1;
    return Review(
      id: id,
      courseId: 'course-1',
      rating: 4,
      comment: '$id viewer $viewerRevision',
      authorHandle: 'alice',
      approveCount: 3,
      viewerLiked: firstViewer,
      canEdit: firstViewer,
      canReport: !firstViewer,
    );
  }
}

Account _account(String id) {
  return Account(
    id: id,
    handle: id,
    avatarUrl: null,
    role: AccountRoleEnum.user,
    capabilities: const <String>[],
    trustLevel: 1,
    hasPassword: true,
    onboardingRequired: false,
    createdAt: 1,
  );
}
