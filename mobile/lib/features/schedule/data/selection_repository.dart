import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';

abstract interface class SelectionRepository {
  Future<List<Calendar>> calendars({CancelToken? cancelToken});
  Future<List<CourseNature>> natures(
    String calendarId, {
    CancelToken? cancelToken,
  });
  Future<LatestUpdate?> latestUpdate({CancelToken? cancelToken});
  Future<List<String>> grades(String calendarId, {CancelToken? cancelToken});
  Future<List<Major>> majors({
    required String calendarId,
    required String grade,
    CancelToken? cancelToken,
  });
  Future<SelectionOfferingPage> offerings({
    required String calendarId,
    String? query,
    String? majorId,
    String? grade,
    String? natureId,
    int? weekday,
    int? startSlot,
    int? endSlot,
    int? week,
    bool includeUnknownSchedule = true,
    String? cursor,
    int limit = 20,
    CancelToken? cancelToken,
  });
  Future<List<TimeSlot>> timeslots(
    String offeringId, {
    CancelToken? cancelToken,
  });
}

class GeneratedSelectionRepository implements SelectionRepository {
  const GeneratedSelectionRepository(this._api);

  final SelectionApi _api;

  @override
  Future<List<Calendar>> calendars({CancelToken? cancelToken}) => _request(
    () => _api.selectionCalendarsGet(cancelToken: cancelToken),
    '学期',
  );

  @override
  Future<List<CourseNature>> natures(
    String calendarId, {
    CancelToken? cancelToken,
  }) => _request(
    () => _api.selectionCourseNaturesGet(
      calendarId: calendarId,
      cancelToken: cancelToken,
    ),
    '课程性质',
  );

  @override
  Future<LatestUpdate?> latestUpdate({CancelToken? cancelToken}) async {
    try {
      final Response<LatestUpdate> response = await _api
          .selectionLatestUpdateGet(cancelToken: cancelToken);
      return response.data;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  @override
  Future<List<String>> grades(String calendarId, {CancelToken? cancelToken}) =>
      _request(
        () => _api.selectionGradesGet(
          calendarId: calendarId,
          cancelToken: cancelToken,
        ),
        '年级',
      );

  @override
  Future<List<Major>> majors({
    required String calendarId,
    required String grade,
    CancelToken? cancelToken,
  }) => _request(
    () => _api.selectionMajorsGet(
      calendarId: calendarId,
      grade: grade,
      cancelToken: cancelToken,
    ),
    '专业',
  );

  @override
  Future<SelectionOfferingPage> offerings({
    required String calendarId,
    String? query,
    String? majorId,
    String? grade,
    String? natureId,
    int? weekday,
    int? startSlot,
    int? endSlot,
    int? week,
    bool includeUnknownSchedule = true,
    String? cursor,
    int limit = 20,
    CancelToken? cancelToken,
  }) async {
    try {
      final Response<SelectionOfferingPage> response = await _api
          .selectionOfferingsGet(
            q: query,
            calendarId: calendarId,
            majorId: majorId,
            grade: grade,
            natureId: natureId,
            weekday: weekday,
            startSlot: startSlot,
            endSlot: endSlot,
            week: week,
            includeUnknownSchedule: includeUnknownSchedule,
            cursor: cursor,
            limit: limit,
            cancelToken: cancelToken,
          );
      final SelectionOfferingPage? page = response.data;
      if (page == null) {
        throw const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '教学班列表响应不完整，请稍后重试',
        );
      }
      return page;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    } on TypeError {
      throw const ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '教学班列表响应不完整，请稍后重试',
      );
    }
  }

  @override
  Future<List<TimeSlot>> timeslots(
    String offeringId, {
    CancelToken? cancelToken,
  }) => _request(
    () => _api.selectionOfferingsOfferingIdTimeslotsGet(
      offeringId: offeringId,
      cancelToken: cancelToken,
    ),
    '教学班时段',
  );

  Future<List<T>> _request<T>(
    Future<Response<List<T>>> Function() request,
    String surface,
  ) async {
    try {
      final Response<List<T>> response = await request();
      final List<T>? data = response.data;
      if (data == null) {
        throw ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '$surface响应不完整，请稍后重试',
        );
      }
      return data;
    } on ApiFailure {
      rethrow;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    } on TypeError {
      throw ApiFailure(
        kind: ApiFailureKind.unexpected,
        message: '$surface响应不完整，请稍后重试',
      );
    }
  }
}

final Provider<SelectionRepository> selectionRepositoryProvider =
    Provider<SelectionRepository>((Ref ref) {
      return GeneratedSelectionRepository(
        ref.watch(apiProvider).getSelectionApi(),
      );
    });
