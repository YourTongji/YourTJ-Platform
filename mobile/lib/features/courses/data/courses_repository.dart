import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';
import '../domain/course_models.dart';

abstract interface class CoursesRepository {
  Future<List<Department>> departments({CancelToken? cancelToken});

  Future<CoursePageSlice> browse({
    required String sort,
    String? departmentId,
    String? cursor,
    CancelToken? cancelToken,
  });

  Future<CoursePageSlice> search({
    required String query,
    String? cursor,
    CancelToken? cancelToken,
  });

  Future<CourseDetail> detail(String courseId, {CancelToken? cancelToken});

  Future<AiSummary?> aiSummary(String courseId, {CancelToken? cancelToken});

  Future<List<Course>> related(String courseId, {CancelToken? cancelToken});
}

class GeneratedCoursesRepository implements CoursesRepository {
  const GeneratedCoursesRepository(this._coursesApi, this._searchApi);

  final CoursesApi _coursesApi;
  final SearchApi _searchApi;

  @override
  Future<List<Department>> departments({CancelToken? cancelToken}) async {
    try {
      final Response<List<Department>> response = await _coursesApi
          .departmentsGet(cancelToken: cancelToken);
      return response.data ?? const <Department>[];
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  @override
  Future<CoursePageSlice> browse({
    required String sort,
    String? departmentId,
    String? cursor,
    CancelToken? cancelToken,
  }) async {
    try {
      final Response<CoursePage> response = await _coursesApi.coursesGet(
        dept: departmentId,
        sort: sort,
        cursor: cursor,
        limit: 20,
        cancelToken: cancelToken,
      );
      final CoursePage? page = response.data;
      if (page == null) {
        throw _incompleteResponse('课程列表');
      }
      return CoursePageSlice(
        items: page.items
            .map(CourseListEntry.fromCourse)
            .where((CourseListEntry course) => course.id.isNotEmpty)
            .toList(growable: false),
        nextCursor: page.nextCursor,
        hasMore: page.hasMore,
      );
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  @override
  Future<CoursePageSlice> search({
    required String query,
    String? cursor,
    CancelToken? cancelToken,
  }) async {
    try {
      final Response<SearchResult> response = await _searchApi.searchGet(
        q: query,
        type: 'course',
        limit: 30,
        cursor: cursor,
        cancelToken: cancelToken,
      );
      final SearchResult? result = response.data;
      if (result == null) {
        throw _incompleteResponse('课程搜索');
      }
      return CoursePageSlice(
        items: result.courses
            .map(CourseListEntry.fromSearchHit)
            .toList(growable: false),
        nextCursor: result.nextCursor,
        hasMore: result.hasMore,
      );
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  @override
  Future<CourseDetail> detail(
    String courseId, {
    CancelToken? cancelToken,
  }) async {
    try {
      final Response<CourseDetail> response = await _coursesApi.coursesIdGet(
        id: courseId,
        cancelToken: cancelToken,
      );
      return response.data ?? (throw _incompleteResponse('课程详情'));
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  @override
  Future<AiSummary?> aiSummary(
    String courseId, {
    CancelToken? cancelToken,
  }) async {
    try {
      final Response<AiSummary> response = await _coursesApi
          .coursesIdAiSummaryGet(id: courseId, cancelToken: cancelToken);
      return response.data;
    } on DioException catch (exception) {
      final ApiFailure failure = ApiFailure.fromDio(exception);
      if (failure.kind == ApiFailureKind.notFound) {
        return null;
      }
      throw failure;
    }
  }

  @override
  Future<List<Course>> related(
    String courseId, {
    CancelToken? cancelToken,
  }) async {
    try {
      final Response<List<Course>> response = await _coursesApi
          .coursesIdRelatedGet(id: courseId, cancelToken: cancelToken);
      return response.data ?? const <Course>[];
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  ApiFailure _incompleteResponse(String surface) {
    return ApiFailure(
      kind: ApiFailureKind.unexpected,
      message: '$surface响应不完整，请稍后重试',
    );
  }
}

final Provider<CoursesRepository> coursesRepositoryProvider =
    Provider<CoursesRepository>((Ref ref) {
      final YourtjApi api = ref.watch(apiProvider);
      return GeneratedCoursesRepository(
        api.getCoursesApi(),
        api.getSearchApi(),
      );
    });
