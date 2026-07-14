import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../data/schedule_local_repository.dart';
import '../data/selection_repository.dart';
import 'schedule_models.dart';

enum SelectionBrowseMode { major, nature, search }

class ScheduleController extends ChangeNotifier {
  ScheduleController({
    required ScheduleNamespace scope,
    required SelectionRepository selectionSource,
    required ScheduleLocalRepository localSource,
  }) : _namespace = scope,
       _selectionRepository = selectionSource,
       _localRepository = localSource;

  final ScheduleNamespace _namespace;
  final SelectionRepository _selectionRepository;
  final ScheduleLocalRepository _localRepository;

  List<Calendar> _calendars = const <Calendar>[];
  List<CourseNature> _natures = const <CourseNature>[];
  List<String> _grades = const <String>[];
  List<Major> _majors = const <Major>[];
  List<SelectionCourse> _courses = const <SelectionCourse>[];
  List<ScheduledCourse> _scheduled = const <ScheduledCourse>[];
  LatestUpdate? _latestUpdate;
  String? _calendarId;
  String? _grade;
  String? _majorId;
  String? _natureId;
  String _query = '';
  SelectionBrowseMode _mode = SelectionBrowseMode.major;
  bool _isLoading = true;
  bool _areContextOptionsLoading = false;
  bool _areCoursesLoading = false;
  final Set<String> _busyCourseCodes = <String>{};
  ApiFailure? _failure;
  ApiFailure? _contextFailure;
  ApiFailure? _coursesFailure;
  ApiFailure? _storageFailure;
  CancelToken? _metadataRequest;
  CancelToken? _contextRequest;
  CancelToken? _coursesRequest;
  int _metadataGeneration = 0;
  int _contextGeneration = 0;
  int _coursesGeneration = 0;
  bool _isDisposed = false;

  List<Calendar> get calendars => _calendars;
  List<CourseNature> get natures => _natures;
  List<String> get grades => _grades;
  List<Major> get majors => _majors;
  List<SelectionCourse> get courses => _courses;
  List<ScheduledCourse> get scheduled => _scheduled;
  LatestUpdate? get latestUpdate => _latestUpdate;
  String? get calendarId => _calendarId;
  String? get grade => _grade;
  String? get majorId => _majorId;
  String? get natureId => _natureId;
  String get query => _query;
  SelectionBrowseMode get mode => _mode;
  bool get isLoading => _isLoading;
  bool get areContextOptionsLoading => _areContextOptionsLoading;
  bool get areCoursesLoading => _areCoursesLoading;
  ApiFailure? get failure => _failure;
  ApiFailure? get contextFailure => _contextFailure;
  ApiFailure? get coursesFailure => _coursesFailure;
  ApiFailure? get storageFailure => _storageFailure;

  bool isCourseBusy(String code) => _busyCourseCodes.contains(code);

  num get totalCredits => _scheduled.fold<num>(
    0,
    (num total, ScheduledCourse item) => total + (item.course.credit ?? 0),
  );

  Future<void> initialize() async {
    final int generation = ++_metadataGeneration;
    _metadataRequest?.cancel('selection metadata replaced');
    final CancelToken request = CancelToken();
    _metadataRequest = request;
    _isLoading = true;
    _failure = null;
    notifyListeners();
    try {
      final List<Object?> values = await Future.wait<Object?>(<Future<Object?>>[
        _selectionRepository.calendars(cancelToken: request),
        _selectionRepository.natures(cancelToken: request),
        _selectionRepository.latestUpdate(cancelToken: request),
      ]);
      if (!_isCurrentMetadata(generation, request)) {
        return;
      }
      _calendars = (values[0]! as List<Calendar>)
          .where((Calendar item) => item.id?.isNotEmpty == true)
          .toList(growable: false);
      _natures = (values[1]! as List<CourseNature>)
          .where((CourseNature item) => item.id?.isNotEmpty == true)
          .toList(growable: false);
      _latestUpdate = values[2] as LatestUpdate?;
      if (_calendars.isNotEmpty) {
        final Calendar selected = _calendars.firstWhere(
          (Calendar item) => item.isCurrent == true,
          orElse: () => _calendars.first,
        );
        await selectCalendar(selected.id!);
      }
    } on ApiFailure catch (failure) {
      if (_isCurrentMetadata(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _failure = failure;
      }
    } finally {
      if (_isCurrentMetadata(generation, request)) {
        _isLoading = false;
        notifyListeners();
      }
    }
  }

  Future<void> selectCalendar(String? calendarId) async {
    if (calendarId == null || calendarId.isEmpty) {
      return;
    }
    final int generation = ++_contextGeneration;
    _contextRequest?.cancel('selection calendar replaced');
    final CancelToken request = CancelToken();
    _contextRequest = request;
    _calendarId = calendarId;
    _grade = null;
    _majorId = null;
    _grades = const <String>[];
    _majors = const <Major>[];
    _courses = const <SelectionCourse>[];
    _scheduled = const <ScheduledCourse>[];
    _areContextOptionsLoading = true;
    _contextFailure = null;
    _storageFailure = null;
    notifyListeners();
    try {
      await Future.wait<void>(<Future<void>>[
        _loadGrades(calendarId, generation, request),
        _loadLocalSchedule(calendarId, generation, request),
      ]);
    } finally {
      if (_isCurrentContext(generation, request)) {
        _areContextOptionsLoading = false;
        notifyListeners();
      }
    }
  }

  Future<void> _loadGrades(
    String calendarId,
    int generation,
    CancelToken request,
  ) async {
    try {
      final List<String> grades = await _selectionRepository.grades(
        calendarId,
        cancelToken: request,
      );
      if (_isCurrentContext(generation, request)) {
        _grades = grades
            .where((String item) => item.trim().isNotEmpty)
            .toList(growable: false);
      }
    } on ApiFailure catch (failure) {
      if (_isCurrentContext(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _contextFailure = failure;
      }
    }
  }

  Future<void> _loadLocalSchedule(
    String calendarId,
    int generation,
    CancelToken request,
  ) async {
    try {
      final List<ScheduledCourse> scheduled = await _localRepository.load(
        namespace: _namespace,
        calendarId: calendarId,
      );
      if (_isCurrentContext(generation, request)) {
        _scheduled = scheduled;
      }
    } on ApiFailure catch (failure) {
      if (_isCurrentContext(generation, request)) {
        _storageFailure = failure;
      }
    }
  }

  Future<void> selectGrade(String? grade) async {
    if (grade == null || grade.isEmpty) {
      return;
    }
    final int generation = ++_contextGeneration;
    _contextRequest?.cancel('selection grade replaced');
    final CancelToken request = CancelToken();
    _contextRequest = request;
    _grade = grade;
    _majorId = null;
    _majors = const <Major>[];
    _courses = const <SelectionCourse>[];
    _areContextOptionsLoading = true;
    _contextFailure = null;
    notifyListeners();
    try {
      final List<Major> majors = await _selectionRepository.majors(
        grade,
        cancelToken: request,
      );
      if (_isCurrentContext(generation, request)) {
        _majors = majors
            .where((Major item) => item.id?.isNotEmpty == true)
            .toList(growable: false);
      }
    } on ApiFailure catch (failure) {
      if (_isCurrentContext(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _contextFailure = failure;
      }
    } finally {
      if (_isCurrentContext(generation, request)) {
        _areContextOptionsLoading = false;
        notifyListeners();
      }
    }
  }

  Future<void> selectMajor(String? majorId) async {
    if (majorId == null || majorId.isEmpty || _grade == null) {
      return;
    }
    _majorId = majorId;
    await _loadCourses(
      (CancelToken request) => _selectionRepository.byMajor(
        majorId: majorId,
        grade: _grade!,
        cancelToken: request,
      ),
    );
  }

  Future<void> selectNature(String? natureId) async {
    if (natureId == null || natureId.isEmpty) {
      return;
    }
    _natureId = natureId;
    await _loadCourses(
      (CancelToken request) =>
          _selectionRepository.byNature(natureId, cancelToken: request),
    );
  }

  Future<void> submitSearch(String query) async {
    final String normalized = query.trim();
    _query = normalized;
    if (normalized.length < 2) {
      _courses = const <SelectionCourse>[];
      _coursesFailure = const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '请输入至少 2 个字符进行搜索',
      );
      notifyListeners();
      return;
    }
    await _loadCourses(
      (CancelToken request) =>
          _selectionRepository.search(normalized, cancelToken: request),
    );
  }

  void setMode(SelectionBrowseMode mode) {
    if (_mode == mode) {
      return;
    }
    _mode = mode;
    _courses = const <SelectionCourse>[];
    _coursesFailure = null;
    notifyListeners();
    if (mode == SelectionBrowseMode.major && _majorId != null) {
      selectMajor(_majorId);
    } else if (mode == SelectionBrowseMode.nature && _natureId != null) {
      selectNature(_natureId);
    } else if (mode == SelectionBrowseMode.search && _query.length >= 2) {
      submitSearch(_query);
    }
  }

  Future<void> _loadCourses(
    Future<List<SelectionCourse>> Function(CancelToken request) load,
  ) async {
    final int generation = ++_coursesGeneration;
    _coursesRequest?.cancel('selection result replaced');
    final CancelToken request = CancelToken();
    _coursesRequest = request;
    _areCoursesLoading = true;
    _coursesFailure = null;
    _courses = const <SelectionCourse>[];
    notifyListeners();
    try {
      final List<SelectionCourse> courses = await load(request);
      if (_isCurrentCourses(generation, request)) {
        _courses = courses;
      }
    } on ApiFailure catch (failure) {
      if (_isCurrentCourses(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _coursesFailure = failure;
      }
    } finally {
      if (_isCurrentCourses(generation, request)) {
        _areCoursesLoading = false;
        notifyListeners();
      }
    }
  }

  Future<ScheduleAddResult> addCourse(SelectionCourse course) async {
    if (!_busyCourseCodes.add(course.code)) {
      return const ScheduleAddResult.duplicate();
    }
    notifyListeners();
    try {
      if (_scheduled.any(
        (ScheduledCourse item) => item.course.code == course.code,
      )) {
        return const ScheduleAddResult.duplicate();
      }
      final List<TimeSlot> timeslots = (await _selectionRepository.timeslots(
        course.code,
      )).where(_isValidTimeslot).toList(growable: false);
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: _scheduled,
        candidate: timeslots,
      );
      if (conflict != null) {
        return ScheduleAddResult.conflict(
          conflict: conflict,
          pendingCourse: course,
          pendingTimeslots: timeslots,
        );
      }
      await confirmAdd(course, timeslots);
      return const ScheduleAddResult.added();
    } finally {
      if (!_isDisposed) {
        _busyCourseCodes.remove(course.code);
        notifyListeners();
      }
    }
  }

  Future<void> confirmAdd(
    SelectionCourse course,
    List<TimeSlot> timeslots,
  ) async {
    final String? selectedCalendarId = _calendarId;
    if (selectedCalendarId == null) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '请先选择学期',
      );
    }
    if (_scheduled.any(
      (ScheduledCourse item) => item.course.code == course.code,
    )) {
      return;
    }
    final List<ScheduledCourse> next = <ScheduledCourse>[
      ..._scheduled,
      ScheduledCourse(
        course: course,
        timeslots: timeslots.where(_isValidTimeslot).toList(growable: false),
        colorIndex: _scheduled.length % 8,
      ),
    ];
    await _localRepository.save(
      namespace: _namespace,
      calendarId: selectedCalendarId,
      courses: next,
    );
    if (!_isDisposed && _calendarId == selectedCalendarId) {
      _scheduled = next;
      _storageFailure = null;
      notifyListeners();
    }
  }

  Future<void> removeCourse(String courseCode) async {
    final String? selectedCalendarId = _calendarId;
    if (selectedCalendarId == null) {
      return;
    }
    final List<ScheduledCourse> next = _scheduled
        .where((ScheduledCourse item) => item.course.code != courseCode)
        .toList(growable: false);
    await _localRepository.save(
      namespace: _namespace,
      calendarId: selectedCalendarId,
      courses: next,
    );
    if (!_isDisposed && _calendarId == selectedCalendarId) {
      _scheduled = next;
      notifyListeners();
    }
  }

  Future<void> clearSchedule() async {
    final String? selectedCalendarId = _calendarId;
    if (selectedCalendarId == null) {
      return;
    }
    await _localRepository.clear(
      namespace: _namespace,
      calendarId: selectedCalendarId,
    );
    if (!_isDisposed && _calendarId == selectedCalendarId) {
      _scheduled = const <ScheduledCourse>[];
      notifyListeners();
    }
  }

  bool _isValidTimeslot(TimeSlot timeslot) {
    return timeslot.weekday >= 1 &&
        timeslot.weekday <= 7 &&
        timeslot.startSlot >= 1 &&
        timeslot.startSlot <= 13 &&
        timeslot.endSlot >= timeslot.startSlot &&
        timeslot.endSlot <= 13;
  }

  bool _isCurrentMetadata(int generation, CancelToken request) {
    return !_isDisposed &&
        generation == _metadataGeneration &&
        identical(_metadataRequest, request);
  }

  bool _isCurrentContext(int generation, CancelToken request) {
    return !_isDisposed &&
        generation == _contextGeneration &&
        identical(_contextRequest, request);
  }

  bool _isCurrentCourses(int generation, CancelToken request) {
    return !_isDisposed &&
        generation == _coursesGeneration &&
        identical(_coursesRequest, request);
  }

  @override
  void dispose() {
    _isDisposed = true;
    _metadataRequest?.cancel('selection metadata disposed');
    _contextRequest?.cancel('selection context disposed');
    _coursesRequest?.cancel('selection courses disposed');
    super.dispose();
  }
}
