import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/courses/data/courses_repository.dart';
import 'package:yourtj_mobile/features/courses/domain/course_detail_controller.dart';
import 'package:yourtj_mobile/features/courses/domain/course_models.dart';
import 'package:yourtj_mobile/features/reviews/data/reviews_repository.dart';

void main() {
  test('uses viewerLiked to unlike an already liked review', () async {
    final _FakeReviewsRepository reviews = _FakeReviewsRepository();
    final CourseDetailController controller = CourseDetailController(
      courseId: 'course-1',
      courseSource: _FakeCoursesRepository(),
      reviewSource: reviews,
    );
    final Review likedReview = _review(viewerLiked: true);

    await controller.toggleLike(likedReview);

    expect(reviews.unlikedIds, <String>['review-1']);
    expect(reviews.likedIds, isEmpty);
    controller.dispose();
  });

  test('edits an owned review and refreshes review and course state', () async {
    final _FakeCoursesRepository courses = _FakeCoursesRepository();
    final _FakeReviewsRepository reviews = _FakeReviewsRepository();
    final CourseDetailController controller = CourseDetailController(
      courseId: 'course-1',
      courseSource: courses,
      reviewSource: reviews,
    );

    await controller.edit(
      reviewId: 'review-1',
      rating: 5,
      comment: '更新后的体验',
      semester: '2026 春',
      score: 'A',
    );

    expect(reviews.editedReviewId, 'review-1');
    expect(reviews.editedRating, 5);
    expect(reviews.editedComment, '更新后的体验');
    expect(reviews.listCalls, 1);
    expect(courses.detailCalls, 1);
    controller.dispose();
  });

  test(
    'loads an exact target review independently from the first page',
    () async {
      final _FakeReviewsRepository reviews = _FakeReviewsRepository();
      final CourseDetailController controller = CourseDetailController(
        courseId: 'course-1',
        targetReviewId: 'review-target',
        courseSource: _FakeCoursesRepository(),
        reviewSource: reviews,
      );

      await controller.initialize();

      expect(reviews.requestedReviewId, 'review-target');
      expect(controller.targetReview?.id, 'review-target');
      expect(controller.reviews, isEmpty);
      controller.dispose();
    },
  );

  test('does not expose a target review owned by another course', () async {
    final _FakeReviewsRepository reviews = _FakeReviewsRepository(
      targetReview: _review(id: 'review-target', courseId: 'course-2'),
    );
    final CourseDetailController controller = CourseDetailController(
      courseId: 'course-1',
      targetReviewId: 'review-target',
      courseSource: _FakeCoursesRepository(),
      reviewSource: reviews,
    );

    await controller.reloadTargetReview();

    expect(controller.targetReview, isNull);
    controller.dispose();
  });

  test(
    'reloads server report capability for list and matching target',
    () async {
      final _FakeReviewsRepository reviews = _FakeReviewsRepository(
        listItems: <Review>[_review(canReport: true)],
        targetReview: _review(canReport: true),
        reportedListItems: <Review>[_review(canReport: false)],
        reportedTargetReview: _review(canReport: false),
      );
      final CourseDetailController controller = CourseDetailController(
        courseId: 'course-1',
        targetReviewId: 'review-1',
        courseSource: _FakeCoursesRepository(),
        reviewSource: reviews,
      );

      await controller.initialize();
      expect(controller.reviews.single.canReport, isTrue);
      expect(controller.targetReview?.canReport, isTrue);

      await controller.report(
        reviewId: 'review-1',
        reason: 'spam',
        captchaToken: 'captcha-token',
      );

      expect(reviews.reportedReviewId, 'review-1');
      expect(reviews.reportedReason, 'spam');
      expect(reviews.listCalls, 2);
      expect(reviews.getCalls, 2);
      expect(controller.reviews.single.canReport, isFalse);
      expect(controller.targetReview?.canReport, isFalse);
      controller.dispose();
    },
  );
}

Review _review({
  String id = 'review-1',
  String courseId = 'course-1',
  bool viewerLiked = false,
  bool canEdit = false,
  bool canReport = true,
}) {
  return Review(
    id: id,
    courseId: courseId,
    rating: 4,
    comment: '课程体验',
    authorHandle: 'alice',
    approveCount: 3,
    viewerLiked: viewerLiked,
    canEdit: canEdit,
    canReport: canReport,
  );
}

class _FakeCoursesRepository implements CoursesRepository {
  int detailCalls = 0;

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
    detailCalls += 1;
    return CourseDetail(id: courseId, code: 'CS101', name: '程序设计');
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

class _FakeReviewsRepository implements ReviewsRepository {
  _FakeReviewsRepository({
    Review? targetReview,
    this.listItems = const <Review>[],
    this.reportedListItems,
    this.reportedTargetReview,
  }) : _targetReview = targetReview ?? _review(id: 'review-target');

  final Review _targetReview;
  final List<Review> listItems;
  final List<Review>? reportedListItems;
  final Review? reportedTargetReview;
  final List<String> likedIds = <String>[];
  final List<String> unlikedIds = <String>[];
  int listCalls = 0;
  int getCalls = 0;
  bool _didReport = false;
  String? requestedReviewId;
  String? editedReviewId;
  int? editedRating;
  String? editedComment;
  String? reportedReviewId;
  String? reportedReason;

  @override
  Future<Review> edit({
    required String reviewId,
    required int rating,
    String? comment,
    String? semester,
    String? score,
  }) async {
    editedReviewId = reviewId;
    editedRating = rating;
    editedComment = comment;
    return _review(id: reviewId, canEdit: true, canReport: false);
  }

  @override
  Future<Review> get(String reviewId, {CancelToken? cancelToken}) async {
    getCalls += 1;
    requestedReviewId = reviewId;
    final Review? currentReportedTarget = reportedTargetReview;
    if (_didReport && currentReportedTarget != null) {
      return currentReportedTarget;
    }
    return _targetReview;
  }

  @override
  Future<void> like(String reviewId) async {
    likedIds.add(reviewId);
  }

  @override
  Future<ReviewPageSlice> list({
    required String courseId,
    required String sort,
    String? cursor,
    CancelToken? cancelToken,
  }) async {
    listCalls += 1;
    final List<Review>? currentReportedItems = reportedListItems;
    return ReviewPageSlice(
      items: _didReport && currentReportedItems != null
          ? currentReportedItems
          : listItems,
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
  }) async => _review();

  @override
  Future<void> report({
    required String reviewId,
    required String reason,
    required String captchaToken,
  }) async {
    reportedReviewId = reviewId;
    reportedReason = reason;
    _didReport = true;
  }

  @override
  Future<void> unlike(String reviewId) async {
    unlikedIds.add(reviewId);
  }
}
