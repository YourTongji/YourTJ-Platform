import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';

abstract interface class SelectionRepository {
  Future<List<Calendar>> calendars({CancelToken? cancelToken});
  Future<List<CourseNature>> natures({CancelToken? cancelToken});
  Future<LatestUpdate?> latestUpdate({CancelToken? cancelToken});
  Future<List<String>> grades(String calendarId, {CancelToken? cancelToken});
  Future<List<Major>> majors({
    required String calendarId,
    required String grade,
    CancelToken? cancelToken,
  });
  Future<List<SelectionCourse>> byMajor({
    required String calendarId,
    required String majorId,
    required String grade,
    CancelToken? cancelToken,
  });
  Future<List<SelectionCourse>> byNature({
    required String calendarId,
    required String natureId,
    CancelToken? cancelToken,
  });
  Future<List<SelectionCourse>> search({
    required String calendarId,
    required String query,
    CancelToken? cancelToken,
  });
  Future<List<TimeSlot>> timeslots(
    String teachingClassId, {
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
  Future<List<CourseNature>> natures({CancelToken? cancelToken}) => _request(
    () => _api.selectionCourseNaturesGet(cancelToken: cancelToken),
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
  Future<List<SelectionCourse>> byMajor({
    required String calendarId,
    required String majorId,
    required String grade,
    CancelToken? cancelToken,
  }) => _request(
    () => _api.selectionCoursesByMajorGet(
      calendarId: calendarId,
      majorId: majorId,
      grade: grade,
      cancelToken: cancelToken,
    ),
    '培养方案课程',
  );

  @override
  Future<List<SelectionCourse>> byNature({
    required String calendarId,
    required String natureId,
    CancelToken? cancelToken,
  }) => _request(
    () => _api.selectionCoursesByNatureGet(
      calendarId: calendarId,
      natureId: natureId,
      cancelToken: cancelToken,
    ),
    '课程性质列表',
  );

  @override
  Future<List<SelectionCourse>> search({
    required String calendarId,
    required String query,
    CancelToken? cancelToken,
  }) => _request(
    () => _api.selectionCoursesSearchGet(
      calendarId: calendarId,
      q: query,
      cancelToken: cancelToken,
    ),
    '选课搜索',
  );

  @override
  Future<List<TimeSlot>> timeslots(
    String teachingClassId, {
    CancelToken? cancelToken,
  }) => _request(
    () => _api.selectionCoursesTeachingClassIdTimeslotsGet(
      teachingClassId: teachingClassId,
      cancelToken: cancelToken,
    ),
    '课程时段',
  );

  Future<List<T>> _request<T>(
    Future<Response<List<T>>> Function() request,
    String surface,
  ) async {
    try {
      final Response<List<T>> response = await request();
      return response.data ?? <T>[];
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
