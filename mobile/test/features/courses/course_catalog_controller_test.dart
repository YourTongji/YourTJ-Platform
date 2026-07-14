import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/courses/data/courses_repository.dart';
import 'package:yourtj_mobile/features/courses/domain/course_catalog_controller.dart';
import 'package:yourtj_mobile/features/courses/domain/course_models.dart';

void main() {
  test(
    'loads filters, continues cursor pages, and switches to typed search',
    () async {
      final _FakeCoursesRepository repository = _FakeCoursesRepository();
      final CourseCatalogController controller = CourseCatalogController(
        repository,
      );

      await controller.initialize();
      expect(controller.departments.single.name, '计算机');
      expect(
        controller.courses.map((CourseListEntry item) => item.id),
        <String>['1'],
      );
      expect(controller.hasMore, isTrue);

      await controller.loadMore();
      expect(
        controller.courses.map((CourseListEntry item) => item.id),
        <String>['1', '2'],
      );
      expect(controller.hasMore, isFalse);

      await controller.setSort('rating');
      expect(repository.lastSort, 'rating');

      await controller.submitQuery('算法');
      expect(repository.lastQuery, '算法');
      expect(controller.courses.single.name, '算法设计');

      controller.dispose();
    },
  );
}

class _FakeCoursesRepository implements CoursesRepository {
  String? lastSort;
  String? lastQuery;

  @override
  Future<List<Department>> departments({CancelToken? cancelToken}) async {
    return <Department>[Department(id: 'cs', name: '计算机')];
  }

  @override
  Future<CoursePageSlice> browse({
    required String sort,
    String? departmentId,
    String? cursor,
    CancelToken? cancelToken,
  }) async {
    lastSort = sort;
    if (cursor == 'next') {
      return CoursePageSlice(
        items: <CourseListEntry>[_entry('2', '数据结构')],
        nextCursor: null,
        hasMore: false,
      );
    }
    return CoursePageSlice(
      items: <CourseListEntry>[_entry('1', '程序设计')],
      nextCursor: 'next',
      hasMore: true,
    );
  }

  @override
  Future<CoursePageSlice> search({
    required String query,
    String? cursor,
    CancelToken? cancelToken,
  }) async {
    lastQuery = query;
    return CoursePageSlice(
      items: <CourseListEntry>[_entry('3', '算法设计')],
      nextCursor: null,
      hasMore: false,
    );
  }

  @override
  Future<AiSummary?> aiSummary(
    String courseId, {
    CancelToken? cancelToken,
  }) async => null;

  @override
  Future<CourseDetail> detail(
    String courseId, {
    CancelToken? cancelToken,
  }) async {
    return CourseDetail(id: courseId, code: 'CS101', name: '程序设计');
  }

  @override
  Future<List<Course>> related(
    String courseId, {
    CancelToken? cancelToken,
  }) async => const <Course>[];
}

CourseListEntry _entry(String id, String name) {
  return CourseListEntry(id: id, code: 'CS$id', name: name, reviewCount: 0);
}
