import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';

@immutable
class ReviewPageSlice {
  const ReviewPageSlice({
    required this.items,
    required this.nextCursor,
    required this.hasMore,
  });

  final List<Review> items;
  final String? nextCursor;
  final bool hasMore;
}

abstract interface class ReviewsRepository {
  Future<ReviewPageSlice> list({
    required String courseId,
    required String sort,
    String? cursor,
    CancelToken? cancelToken,
  });

  Future<Review> get(String reviewId, {CancelToken? cancelToken});

  Future<Review> publish({
    required String courseId,
    required int rating,
    required String captchaToken,
    required String idempotencyKey,
    String? comment,
    String? semester,
    String? score,
  });

  Future<Review> edit({
    required String reviewId,
    required int rating,
    String? comment,
    String? semester,
    String? score,
  });

  Future<void> like(String reviewId);

  Future<void> unlike(String reviewId);

  Future<void> report({
    required String reviewId,
    required String reason,
    required String captchaToken,
  });
}

class GeneratedReviewsRepository implements ReviewsRepository {
  const GeneratedReviewsRepository(this._api);

  final ReviewsApi _api;

  @override
  Future<ReviewPageSlice> list({
    required String courseId,
    required String sort,
    String? cursor,
    CancelToken? cancelToken,
  }) async {
    try {
      final Response<ReviewPage> response = await _api.coursesIdReviewsGet(
        id: courseId,
        sort: sort,
        cursor: cursor,
        limit: 20,
        cancelToken: cancelToken,
      );
      final ReviewPage? page = response.data;
      if (page == null) {
        throw _incompleteResponse('课程点评');
      }
      return ReviewPageSlice(
        items: page.items,
        nextCursor: page.nextCursor,
        hasMore: page.hasMore,
      );
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  @override
  Future<Review> get(String reviewId, {CancelToken? cancelToken}) async {
    try {
      final Response<Review> response = await _api.reviewsIdGet(
        id: reviewId,
        cancelToken: cancelToken,
      );
      return response.data ?? (throw _incompleteResponse('点评详情'));
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
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
  }) async {
    try {
      final Response<Review> response = await _api.coursesIdReviewsPost(
        id: courseId,
        idempotencyKey: idempotencyKey,
        createReviewInput: CreateReviewInput(
          rating: rating,
          comment: _optionalText(comment),
          semester: _optionalText(semester),
          score: _optionalText(score),
          captchaToken: captchaToken,
        ),
      );
      return response.data ?? (throw _incompleteResponse('发布点评'));
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  @override
  Future<Review> edit({
    required String reviewId,
    required int rating,
    String? comment,
    String? semester,
    String? score,
  }) async {
    try {
      final Response<Review> response = await _api.reviewsIdPatch(
        id: reviewId,
        reviewInput: ReviewInput(
          rating: rating,
          comment: _optionalText(comment),
          semester: _optionalText(semester),
          score: _optionalText(score),
        ),
      );
      return response.data ?? (throw _incompleteResponse('编辑点评'));
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  @override
  Future<void> like(String reviewId) async {
    try {
      await _api.reviewsIdLikePost(id: reviewId);
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  @override
  Future<void> unlike(String reviewId) async {
    try {
      await _api.reviewsIdLikeDelete(id: reviewId);
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  @override
  Future<void> report({
    required String reviewId,
    required String reason,
    required String captchaToken,
  }) async {
    try {
      await _api.reviewsIdReportPost(
        id: reviewId,
        reviewsIdReportPostRequest: ReviewsIdReportPostRequest(
          reason: reason.trim(),
          captchaToken: captchaToken,
        ),
      );
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  String? _optionalText(String? value) {
    final String normalized = value?.trim() ?? '';
    return normalized.isEmpty ? null : normalized;
  }

  ApiFailure _incompleteResponse(String surface) {
    return ApiFailure(
      kind: ApiFailureKind.unexpected,
      message: '$surface响应不完整，请稍后重试',
    );
  }
}

final Provider<ReviewsRepository> reviewsRepositoryProvider =
    Provider<ReviewsRepository>((Ref ref) {
      return GeneratedReviewsRepository(ref.watch(apiProvider).getReviewsApi());
    });
