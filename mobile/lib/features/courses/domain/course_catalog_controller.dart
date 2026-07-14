import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../data/courses_repository.dart';
import 'course_models.dart';

class CourseCatalogController extends ChangeNotifier {
  CourseCatalogController(this._repository);

  final CoursesRepository _repository;

  List<CourseListEntry> _courses = const <CourseListEntry>[];
  List<Department> _departments = const <Department>[];
  String _sort = 'hot';
  String? _departmentId;
  String _query = '';
  String? _nextCursor;
  bool _hasMore = false;
  bool _isLoading = true;
  bool _isLoadingMore = false;
  ApiFailure? _failure;
  ApiFailure? _departmentsFailure;
  CancelToken? _request;
  int _generation = 0;
  bool _isDisposed = false;

  List<CourseListEntry> get courses => _courses;
  List<Department> get departments => _departments;
  String get sort => _sort;
  String? get departmentId => _departmentId;
  String get query => _query;
  bool get hasMore => _hasMore;
  bool get isLoading => _isLoading;
  bool get isLoadingMore => _isLoadingMore;
  ApiFailure? get failure => _failure;
  ApiFailure? get departmentsFailure => _departmentsFailure;

  Future<void> initialize() async {
    await Future.wait<void>(<Future<void>>[_loadDepartments(), reload()]);
  }

  Future<void> _loadDepartments() async {
    try {
      final List<Department> next = await _repository.departments();
      if (_isDisposed) {
        return;
      }
      _departments = next
          .where(
            (Department department) =>
                (department.id?.isNotEmpty ?? false) &&
                (department.name?.isNotEmpty ?? false),
          )
          .toList(growable: false);
      _departmentsFailure = null;
    } on ApiFailure catch (failure) {
      if (_isDisposed) {
        return;
      }
      _departmentsFailure = failure;
    }
    notifyListeners();
  }

  Future<void> setSort(String sort) async {
    if (!const <String>{'hot', 'rating', 'new'}.contains(sort) ||
        _sort == sort) {
      return;
    }
    _sort = sort;
    await reload();
  }

  Future<void> setDepartment(String? departmentId) async {
    final String? normalized = departmentId?.isEmpty == true
        ? null
        : departmentId;
    if (_departmentId == normalized) {
      return;
    }
    _departmentId = normalized;
    await reload();
  }

  Future<void> submitQuery(String query) async {
    final String normalized = query.trim();
    if (normalized.isNotEmpty && normalized.length < 2) {
      _failure = const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '请输入至少 2 个字符进行搜索',
      );
      notifyListeners();
      return;
    }
    if (_query == normalized) {
      await reload();
      return;
    }
    _query = normalized;
    await reload();
  }

  Future<void> reload() async {
    final int generation = ++_generation;
    _request?.cancel('course query replaced');
    final CancelToken request = CancelToken();
    _request = request;
    _isLoading = true;
    _isLoadingMore = false;
    _failure = null;
    _courses = const <CourseListEntry>[];
    _nextCursor = null;
    _hasMore = false;
    notifyListeners();
    try {
      final CoursePageSlice page = await _loadPage(
        cursor: null,
        cancelToken: request,
      );
      if (!_isCurrent(generation, request)) {
        return;
      }
      _courses = page.items;
      _nextCursor = page.nextCursor;
      _hasMore = page.hasMore && page.nextCursor != null;
    } on ApiFailure catch (failure) {
      if (!_isCurrent(generation, request) ||
          failure.kind == ApiFailureKind.cancelled) {
        return;
      }
      _failure = failure;
    } finally {
      if (_isCurrent(generation, request)) {
        _isLoading = false;
        notifyListeners();
      }
    }
  }

  Future<void> loadMore() async {
    final String? cursor = _nextCursor;
    if (_isLoading || _isLoadingMore || !_hasMore || cursor == null) {
      return;
    }
    final int generation = _generation;
    final CancelToken request = CancelToken();
    _request = request;
    _isLoadingMore = true;
    _failure = null;
    notifyListeners();
    try {
      final CoursePageSlice page = await _loadPage(
        cursor: cursor,
        cancelToken: request,
      );
      if (!_isCurrent(generation, request)) {
        return;
      }
      final Set<String> known = _courses
          .map((CourseListEntry course) => course.id)
          .toSet();
      _courses = <CourseListEntry>[
        ..._courses,
        ...page.items.where((CourseListEntry course) => known.add(course.id)),
      ];
      _nextCursor = page.nextCursor;
      _hasMore = page.hasMore && page.nextCursor != null;
    } on ApiFailure catch (failure) {
      if (_isCurrent(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _failure = failure;
      }
    } finally {
      if (_isCurrent(generation, request)) {
        _isLoadingMore = false;
        notifyListeners();
      }
    }
  }

  Future<CoursePageSlice> _loadPage({
    required String? cursor,
    required CancelToken cancelToken,
  }) {
    if (_query.length >= 2) {
      return _repository.search(
        query: _query,
        cursor: cursor,
        cancelToken: cancelToken,
      );
    }
    return _repository.browse(
      sort: _sort,
      departmentId: _departmentId,
      cursor: cursor,
      cancelToken: cancelToken,
    );
  }

  bool _isCurrent(int generation, CancelToken request) {
    return !_isDisposed &&
        generation == _generation &&
        identical(_request, request);
  }

  @override
  void dispose() {
    _isDisposed = true;
    _request?.cancel('course catalog disposed');
    super.dispose();
  }
}
