import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../../reviews/data/reviews_repository.dart';
import '../data/courses_repository.dart';

class CourseDetailController extends ChangeNotifier {
  CourseDetailController({
    required this.courseId,
    required CoursesRepository courseSource,
    required ReviewsRepository reviewSource,
  }) : _coursesRepository = courseSource,
       _reviewsRepository = reviewSource;

  final String courseId;
  final CoursesRepository _coursesRepository;
  final ReviewsRepository _reviewsRepository;

  CourseDetail? _course;
  AiSummary? _summary;
  List<Course> _related = const <Course>[];
  List<Review> _reviews = const <Review>[];
  String _reviewSort = 'hot';
  String? _reviewCursor;
  bool _reviewsHaveMore = false;
  bool _isLoading = true;
  bool _areReviewsLoading = true;
  bool _areMoreReviewsLoading = false;
  bool _isPublishing = false;
  final Set<String> _busyReviewIds = <String>{};
  ApiFailure? _failure;
  ApiFailure? _summaryFailure;
  ApiFailure? _relatedFailure;
  ApiFailure? _reviewsFailure;
  ApiFailure? _mutationFailure;
  CancelToken? _detailRequest;
  CancelToken? _reviewsRequest;
  int _detailGeneration = 0;
  int _reviewsGeneration = 0;
  bool _isDisposed = false;

  CourseDetail? get course => _course;
  AiSummary? get summary => _summary;
  List<Course> get related => _related;
  List<Review> get reviews => _reviews;
  String get reviewSort => _reviewSort;
  bool get reviewsHaveMore => _reviewsHaveMore;
  bool get isLoading => _isLoading;
  bool get areReviewsLoading => _areReviewsLoading;
  bool get areMoreReviewsLoading => _areMoreReviewsLoading;
  bool get isPublishing => _isPublishing;
  ApiFailure? get failure => _failure;
  ApiFailure? get summaryFailure => _summaryFailure;
  ApiFailure? get relatedFailure => _relatedFailure;
  ApiFailure? get reviewsFailure => _reviewsFailure;
  ApiFailure? get mutationFailure => _mutationFailure;

  bool isReviewBusy(String reviewId) => _busyReviewIds.contains(reviewId);

  Future<void> initialize() async {
    await Future.wait<void>(<Future<void>>[reloadDetails(), reloadReviews()]);
  }

  Future<void> reloadDetails() async {
    final int generation = ++_detailGeneration;
    _detailRequest?.cancel('course detail replaced');
    final CancelToken request = CancelToken();
    _detailRequest = request;
    _isLoading = true;
    _failure = null;
    _summaryFailure = null;
    _relatedFailure = null;
    notifyListeners();
    try {
      final CourseDetail course = await _coursesRepository.detail(
        courseId,
        cancelToken: request,
      );
      if (!_isCurrentDetail(generation, request)) {
        return;
      }
      _course = course;
      _isLoading = false;
      notifyListeners();
      await Future.wait<void>(<Future<void>>[
        _loadSummary(generation, request),
        _loadRelated(generation, request),
      ]);
    } on ApiFailure catch (failure) {
      if (_isCurrentDetail(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _failure = failure;
        _isLoading = false;
        notifyListeners();
      }
    }
  }

  Future<void> _loadSummary(int generation, CancelToken request) async {
    try {
      final AiSummary? summary = await _coursesRepository.aiSummary(
        courseId,
        cancelToken: request,
      );
      if (_isCurrentDetail(generation, request)) {
        _summary = summary;
        _summaryFailure = null;
      }
    } on ApiFailure catch (failure) {
      if (_isCurrentDetail(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _summaryFailure = failure;
      }
    }
    if (_isCurrentDetail(generation, request)) {
      notifyListeners();
    }
  }

  Future<void> _loadRelated(int generation, CancelToken request) async {
    try {
      final List<Course> related = await _coursesRepository.related(
        courseId,
        cancelToken: request,
      );
      if (_isCurrentDetail(generation, request)) {
        _related = related;
        _relatedFailure = null;
      }
    } on ApiFailure catch (failure) {
      if (_isCurrentDetail(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _relatedFailure = failure;
      }
    }
    if (_isCurrentDetail(generation, request)) {
      notifyListeners();
    }
  }

  Future<void> setReviewSort(String sort) async {
    if (!const <String>{'hot', 'new'}.contains(sort) || _reviewSort == sort) {
      return;
    }
    _reviewSort = sort;
    await reloadReviews();
  }

  Future<void> reloadReviews() async {
    final int generation = ++_reviewsGeneration;
    _reviewsRequest?.cancel('review list replaced');
    final CancelToken request = CancelToken();
    _reviewsRequest = request;
    _areReviewsLoading = true;
    _areMoreReviewsLoading = false;
    _reviewsFailure = null;
    _reviews = const <Review>[];
    _reviewCursor = null;
    _reviewsHaveMore = false;
    notifyListeners();
    try {
      final ReviewPageSlice page = await _reviewsRepository.list(
        courseId: courseId,
        sort: _reviewSort,
        cancelToken: request,
      );
      if (!_isCurrentReviews(generation, request)) {
        return;
      }
      _reviews = page.items;
      _reviewCursor = page.nextCursor;
      _reviewsHaveMore = page.hasMore && page.nextCursor != null;
    } on ApiFailure catch (failure) {
      if (_isCurrentReviews(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _reviewsFailure = failure;
      }
    } finally {
      if (_isCurrentReviews(generation, request)) {
        _areReviewsLoading = false;
        notifyListeners();
      }
    }
  }

  Future<void> loadMoreReviews() async {
    final String? cursor = _reviewCursor;
    if (_areReviewsLoading ||
        _areMoreReviewsLoading ||
        !_reviewsHaveMore ||
        cursor == null) {
      return;
    }
    final int generation = _reviewsGeneration;
    final CancelToken request = CancelToken();
    _reviewsRequest = request;
    _areMoreReviewsLoading = true;
    _reviewsFailure = null;
    notifyListeners();
    try {
      final ReviewPageSlice page = await _reviewsRepository.list(
        courseId: courseId,
        sort: _reviewSort,
        cursor: cursor,
        cancelToken: request,
      );
      if (!_isCurrentReviews(generation, request)) {
        return;
      }
      final Set<String> known = _reviews
          .map((Review review) => review.id ?? '')
          .toSet();
      _reviews = <Review>[
        ..._reviews,
        ...page.items.where((Review review) => known.add(review.id ?? '')),
      ];
      _reviewCursor = page.nextCursor;
      _reviewsHaveMore = page.hasMore && page.nextCursor != null;
    } on ApiFailure catch (failure) {
      if (_isCurrentReviews(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _reviewsFailure = failure;
      }
    } finally {
      if (_isCurrentReviews(generation, request)) {
        _areMoreReviewsLoading = false;
        notifyListeners();
      }
    }
  }

  Future<void> publish({
    required int rating,
    required String captchaToken,
    required String idempotencyKey,
    String? comment,
    String? semester,
    String? score,
  }) async {
    if (_isPublishing) {
      return;
    }
    _isPublishing = true;
    _mutationFailure = null;
    notifyListeners();
    try {
      await _reviewsRepository.publish(
        courseId: courseId,
        rating: rating,
        comment: comment,
        semester: semester,
        score: score,
        captchaToken: captchaToken,
        idempotencyKey: idempotencyKey,
      );
      if (!_isDisposed) {
        await Future.wait<void>(<Future<void>>[
          reloadReviews(),
          reloadDetails(),
        ]);
      }
    } on ApiFailure catch (failure) {
      if (!_isDisposed) {
        _mutationFailure = failure;
        rethrow;
      }
    } finally {
      if (!_isDisposed) {
        _isPublishing = false;
        notifyListeners();
      }
    }
  }

  Future<void> like(String reviewId) async {
    await _runReviewMutation(reviewId, () => _reviewsRepository.like(reviewId));
  }

  Future<void> report({
    required String reviewId,
    required String reason,
    required String captchaToken,
  }) async {
    await _runReviewMutation(
      reviewId,
      () => _reviewsRepository.report(
        reviewId: reviewId,
        reason: reason,
        captchaToken: captchaToken,
      ),
      reloadAfter: false,
    );
  }

  Future<void> _runReviewMutation(
    String reviewId,
    Future<void> Function() mutation, {
    bool reloadAfter = true,
  }) async {
    if (reviewId.isEmpty || !_busyReviewIds.add(reviewId)) {
      return;
    }
    _mutationFailure = null;
    notifyListeners();
    try {
      await mutation();
      if (reloadAfter && !_isDisposed) {
        await reloadReviews();
      }
    } on ApiFailure catch (failure) {
      if (!_isDisposed) {
        _mutationFailure = failure;
        rethrow;
      }
    } finally {
      if (!_isDisposed) {
        _busyReviewIds.remove(reviewId);
        notifyListeners();
      }
    }
  }

  bool _isCurrentDetail(int generation, CancelToken request) {
    return !_isDisposed &&
        generation == _detailGeneration &&
        identical(_detailRequest, request);
  }

  bool _isCurrentReviews(int generation, CancelToken request) {
    return !_isDisposed &&
        generation == _reviewsGeneration &&
        identical(_reviewsRequest, request);
  }

  @override
  void dispose() {
    _isDisposed = true;
    _detailRequest?.cancel('course detail disposed');
    _reviewsRequest?.cancel('review list disposed');
    super.dispose();
  }
}
