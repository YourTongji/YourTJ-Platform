import 'dart:async';

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
  List<SelectionOffering> _offerings = const <SelectionOffering>[];
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
  bool _areOfferingsLoading = false;
  bool _isLoadingMore = false;
  String? _nextCursor;
  bool _hasMore = false;
  final Set<String> _busyOfferingIds = <String>{};
  final Map<String, CancelToken> _addRequests = <String, CancelToken>{};
  Future<void> _scheduleMutation = Future<void>.value();
  ApiFailure? _failure;
  ApiFailure? _contextFailure;
  ApiFailure? _offeringsFailure;
  ApiFailure? _storageFailure;
  CancelToken? _metadataRequest;
  CancelToken? _contextRequest;
  CancelToken? _offeringsRequest;
  int _metadataGeneration = 0;
  int _contextGeneration = 0;
  int _offeringsGeneration = 0;
  bool _isDisposed = false;

  List<Calendar> get calendars => _calendars;
  List<CourseNature> get natures => _natures;
  List<String> get grades => _grades;
  List<Major> get majors => _majors;
  List<SelectionOffering> get offerings => _offerings;
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
  bool get areOfferingsLoading => _areOfferingsLoading;
  bool get isLoadingMore => _isLoadingMore;
  bool get hasMore => _hasMore;
  ApiFailure? get failure => _failure;
  ApiFailure? get contextFailure => _contextFailure;
  ApiFailure? get offeringsFailure => _offeringsFailure;
  ApiFailure? get storageFailure => _storageFailure;

  bool isOfferingBusy(String offeringId) =>
      _busyOfferingIds.contains(offeringId);

  num get totalCredits => _scheduled.fold<num>(
    0,
    (num total, ScheduledCourse item) => total + (item.offering.credit ?? 0),
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
    _cancelOfferingRequest('selection calendar replaced');
    _cancelAddRequests('selection calendar replaced');
    _calendarId = calendarId;
    _grade = null;
    _majorId = null;
    _grades = const <String>[];
    _majors = const <Major>[];
    _resetOfferingResults();
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
    _cancelOfferingRequest('selection grade replaced');
    _grade = grade;
    _majorId = null;
    _majors = const <Major>[];
    _resetOfferingResults();
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
    await _loadOfferings();
  }

  Future<void> selectNature(String? natureId) async {
    if (natureId == null || natureId.isEmpty) {
      return;
    }
    _natureId = natureId;
    await _loadOfferings();
  }

  Future<void> submitSearch(String query) async {
    final String normalized = query.trim();
    _query = normalized;
    if (normalized.length < 2) {
      _cancelOfferingRequest('selection search invalidated');
      _resetOfferingResults();
      _offeringsFailure = const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '请输入至少 2 个字符进行搜索',
      );
      notifyListeners();
      return;
    }
    await _loadOfferings();
  }

  void setMode(SelectionBrowseMode mode) {
    if (_mode == mode) {
      return;
    }
    _cancelOfferingRequest('selection browse mode replaced');
    _mode = mode;
    _resetOfferingResults();
    notifyListeners();
    if (mode == SelectionBrowseMode.major && _majorId != null) {
      selectMajor(_majorId);
    } else if (mode == SelectionBrowseMode.nature && _natureId != null) {
      selectNature(_natureId);
    } else if (mode == SelectionBrowseMode.search && _query.length >= 2) {
      submitSearch(_query);
    }
  }

  Future<void> loadMore() async {
    if (!_hasMore || _isLoadingMore || _areOfferingsLoading) {
      return;
    }
    await _loadOfferings(append: true);
  }

  Future<void> retryOfferings() {
    return _offerings.isNotEmpty && _hasMore
        ? _loadOfferings(append: true)
        : _loadOfferings();
  }

  Future<void> _loadOfferings({bool append = false}) async {
    final String? selectedCalendarId = _calendarId;
    if (selectedCalendarId == null) {
      return;
    }
    final String? cursor = append ? _nextCursor : null;
    if (append && (cursor == null || cursor.isEmpty)) {
      _hasMore = false;
      notifyListeners();
      return;
    }
    final int generation = ++_offeringsGeneration;
    _offeringsRequest?.cancel('selection result replaced');
    final CancelToken request = CancelToken();
    _offeringsRequest = request;
    _areOfferingsLoading = !append;
    _isLoadingMore = append;
    _offeringsFailure = null;
    if (!append) {
      _offerings = const <SelectionOffering>[];
      _nextCursor = null;
      _hasMore = false;
    }
    notifyListeners();
    try {
      final SelectionOfferingPage page = await _selectionRepository.offerings(
        calendarId: selectedCalendarId,
        query: _mode == SelectionBrowseMode.search ? _query : null,
        majorId: _mode == SelectionBrowseMode.major ? _majorId : null,
        grade: _mode == SelectionBrowseMode.major ? _grade : null,
        natureId: _mode == SelectionBrowseMode.nature ? _natureId : null,
        cursor: cursor,
        cancelToken: request,
      );
      if (_isCurrentOfferings(generation, request)) {
        if (page.hasMore &&
            (page.nextCursor == null || page.nextCursor!.isEmpty)) {
          throw const ApiFailure(
            kind: ApiFailureKind.unexpected,
            message: '教学班分页响应不完整，请稍后重试',
          );
        }
        final List<SelectionOffering> valid = page.items
            .where(
              (SelectionOffering item) =>
                  item.offeringId.isNotEmpty &&
                  item.calendarId == selectedCalendarId,
            )
            .toList(growable: false);
        _offerings = append
            ? _mergeOfferings(_offerings, valid)
            : _mergeOfferings(const <SelectionOffering>[], valid);
        _nextCursor = page.nextCursor;
        _hasMore = page.hasMore;
      }
    } on ApiFailure catch (failure) {
      if (_isCurrentOfferings(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _offeringsFailure = failure;
      }
    } finally {
      if (_isCurrentOfferings(generation, request)) {
        _areOfferingsLoading = false;
        _isLoadingMore = false;
        notifyListeners();
      }
    }
  }

  List<SelectionOffering> _mergeOfferings(
    List<SelectionOffering> current,
    List<SelectionOffering> incoming,
  ) {
    final Set<String> ids = current
        .map((SelectionOffering item) => item.offeringId)
        .toSet();
    return <SelectionOffering>[
      ...current,
      ...incoming.where((SelectionOffering item) => ids.add(item.offeringId)),
    ];
  }

  Future<ScheduleAddResult> addOffering(SelectionOffering offering) async {
    final String? selectedCalendarId = _calendarId;
    if (selectedCalendarId == null ||
        offering.calendarId != selectedCalendarId) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '该教学班不属于当前学期，请刷新后重试',
      );
    }
    if (!_busyOfferingIds.add(offering.offeringId)) {
      return const ScheduleAddResult.duplicate();
    }
    final CancelToken request = CancelToken();
    _addRequests[offering.offeringId] = request;
    notifyListeners();
    try {
      if (_scheduled.any(
        (ScheduledCourse item) =>
            item.offering.offeringId == offering.offeringId,
      )) {
        return const ScheduleAddResult.duplicate();
      }
      final List<TimeSlot> timeslots =
          (await _selectionRepository.timeslots(
                offering.offeringId,
                cancelToken: request,
              ))
              .where((TimeSlot item) {
                return item.offeringId == offering.offeringId &&
                    _isValidTimeslot(item);
              })
              .toList(growable: false);
      if (_calendarId != selectedCalendarId || request.isCancelled) {
        throw const ApiFailure(
          kind: ApiFailureKind.cancelled,
          message: '教学班时段请求已取消',
        );
      }
      final ScheduleConflict? conflict = findScheduleConflict(
        existing: _scheduled,
        candidate: timeslots,
      );
      if (conflict != null) {
        return ScheduleAddResult.conflict(
          conflict: conflict,
          pendingOffering: offering,
          pendingTimeslots: timeslots,
        );
      }
      await confirmAdd(offering, timeslots);
      return const ScheduleAddResult.added();
    } finally {
      if (!_isDisposed) {
        if (identical(_addRequests[offering.offeringId], request)) {
          _addRequests.remove(offering.offeringId);
        }
        _busyOfferingIds.remove(offering.offeringId);
        notifyListeners();
      }
    }
  }

  Future<void> confirmAdd(
    SelectionOffering offering,
    List<TimeSlot> timeslots,
  ) => _serializeScheduleMutation(() async {
    final String? selectedCalendarId = _calendarId;
    if (selectedCalendarId == null ||
        offering.calendarId != selectedCalendarId) {
      throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '该教学班不属于当前学期，请刷新后重试',
      );
    }
    if (_scheduled.any(
      (ScheduledCourse item) => item.offering.offeringId == offering.offeringId,
    )) {
      return;
    }
    final List<ScheduledCourse> next = <ScheduledCourse>[
      ..._scheduled,
      ScheduledCourse(
        offering: offering,
        timeslots: timeslots
            .where((TimeSlot item) {
              return item.offeringId == offering.offeringId &&
                  _isValidTimeslot(item);
            })
            .toList(growable: false),
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
  });

  Future<void> removeOffering(String offeringId) =>
      _serializeScheduleMutation(() async {
        final String? selectedCalendarId = _calendarId;
        if (selectedCalendarId == null) {
          return;
        }
        final List<ScheduledCourse> next = _scheduled
            .where(
              (ScheduledCourse item) => item.offering.offeringId != offeringId,
            )
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
      });

  Future<void> clearSchedule() => _serializeScheduleMutation(() async {
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
  });

  bool _isValidTimeslot(TimeSlot timeslot) {
    return timeslot.weekday >= 1 &&
        timeslot.weekday <= 7 &&
        timeslot.startSlot >= 1 &&
        timeslot.startSlot <= 13 &&
        timeslot.endSlot >= timeslot.startSlot &&
        timeslot.endSlot <= 13;
  }

  Future<T> _serializeScheduleMutation<T>(Future<T> Function() mutation) async {
    final Future<void> previous = _scheduleMutation;
    final Completer<void> release = Completer<void>();
    _scheduleMutation = release.future;
    try {
      await previous;
      return await mutation();
    } finally {
      release.complete();
    }
  }

  void _resetOfferingResults() {
    _offerings = const <SelectionOffering>[];
    _nextCursor = null;
    _hasMore = false;
    _offeringsFailure = null;
  }

  void _cancelOfferingRequest(String reason) {
    _offeringsGeneration += 1;
    _offeringsRequest?.cancel(reason);
    _offeringsRequest = null;
    _areOfferingsLoading = false;
    _isLoadingMore = false;
  }

  void _cancelAddRequests(String reason) {
    for (final CancelToken request in _addRequests.values) {
      request.cancel(reason);
    }
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

  bool _isCurrentOfferings(int generation, CancelToken request) {
    return !_isDisposed &&
        generation == _offeringsGeneration &&
        identical(_offeringsRequest, request);
  }

  @override
  void dispose() {
    _isDisposed = true;
    _metadataRequest?.cancel('selection metadata disposed');
    _contextRequest?.cancel('selection context disposed');
    _offeringsRequest?.cancel('selection offerings disposed');
    _cancelAddRequests('selection offering timeslots disposed');
    super.dispose();
  }
}
