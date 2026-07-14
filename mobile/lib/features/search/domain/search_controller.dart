import 'package:dio/dio.dart';
import 'package:flutter/foundation.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../data/search_repository.dart';
import 'search_models.dart';

class FederatedSearchController extends ChangeNotifier {
  FederatedSearchController(
    this._repository, {
    SearchScope initialScope = SearchScope.all,
  }) : _scope = initialScope;

  final FederatedSearchRepository _repository;

  String _query = '';
  SearchScope _scope;
  List<CourseSearchHit> _courses = const <CourseSearchHit>[];
  List<ReviewSearchHit> _reviews = const <ReviewSearchHit>[];
  List<ThreadSearchHit> _threads = const <ThreadSearchHit>[];
  List<UserSearchHit> _users = const <UserSearchHit>[];
  List<BoardSearchHit> _boards = const <BoardSearchHit>[];
  List<TagSearchHit> _tags = const <TagSearchHit>[];
  List<SearchHighlight> _highlights = const <SearchHighlight>[];
  Set<SearchResultScope> _moreScopes = const <SearchResultScope>{};
  Set<SearchResultScope> _failedScopes = const <SearchResultScope>{};
  String? _suggestedQuery;
  String? _nextCursor;
  bool _hasMore = false;
  bool _isLoading = false;
  bool _isLoadingMore = false;
  ApiFailure? _failure;
  CancelToken? _request;
  int _generation = 0;
  bool _isDisposed = false;

  String get query => _query;
  SearchScope get scope => _scope;
  List<CourseSearchHit> get courses => _courses;
  List<ReviewSearchHit> get reviews => _reviews;
  List<ThreadSearchHit> get threads => _threads;
  List<UserSearchHit> get users => _users;
  List<BoardSearchHit> get boards => _boards;
  List<TagSearchHit> get tags => _tags;
  Set<SearchResultScope> get moreScopes => _moreScopes;
  Set<SearchResultScope> get failedScopes => _failedScopes;
  String? get suggestedQuery => _suggestedQuery;
  bool get hasMore => _hasMore;
  bool get isLoading => _isLoading;
  bool get isLoadingMore => _isLoadingMore;
  ApiFailure? get failure => _failure;
  int get totalResults =>
      _courses.length +
      _reviews.length +
      _threads.length +
      _users.length +
      _boards.length +
      _tags.length;

  Future<void> submit(String query, {SearchScope? scope}) async {
    _query = query.trim();
    if (scope != null) {
      _scope = scope;
    }
    if (_query.length < 2) {
      _cancelAndClear();
      notifyListeners();
      return;
    }
    await reload();
  }

  Future<void> setScope(SearchScope scope) async {
    if (_scope == scope) {
      return;
    }
    _scope = scope;
    if (_query.length < 2) {
      notifyListeners();
      return;
    }
    await reload();
  }

  void invalidateForSessionChange() {
    _cancelAndClear();
    notifyListeners();
  }

  Future<void> reload() async {
    if (_query.length < 2) {
      return;
    }
    final int generation = ++_generation;
    _request?.cancel('search replaced');
    final CancelToken request = CancelToken();
    _request = request;
    _clearResults();
    _isLoading = true;
    _isLoadingMore = false;
    _failure = null;
    notifyListeners();
    try {
      final SearchPageSlice page = await _repository.search(
        query: _query,
        scope: _scope,
        limit: _scope == SearchScope.all ? 6 : 30,
        cancelToken: request,
      );
      if (!_isCurrent(generation, request)) {
        return;
      }
      _apply(page.result, append: false);
    } on ApiFailure catch (failure) {
      if (_isCurrent(generation, request) &&
          failure.kind != ApiFailureKind.cancelled) {
        _failure = failure;
      }
    } finally {
      if (_isCurrent(generation, request)) {
        _isLoading = false;
        notifyListeners();
      }
    }
  }

  Future<void> loadMore() async {
    final String? cursor = _nextCursor;
    if (_scope == SearchScope.all ||
        !_hasMore ||
        cursor == null ||
        _isLoading ||
        _isLoadingMore) {
      return;
    }
    final int generation = _generation;
    final CancelToken request = CancelToken();
    _request = request;
    _isLoadingMore = true;
    _failure = null;
    notifyListeners();
    try {
      final SearchPageSlice page = await _repository.search(
        query: _query,
        scope: _scope,
        limit: 30,
        cursor: cursor,
        cancelToken: request,
      );
      if (_isCurrent(generation, request)) {
        _apply(page.result, append: true);
      }
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

  List<SearchHighlightRange> rangesFor({
    required SearchResultScope scope,
    required String id,
    required SearchHighlightFieldEnum field,
  }) {
    for (final SearchHighlight highlight in _highlights.reversed) {
      if (highlight.scope == scope &&
          highlight.id == id &&
          highlight.field == field) {
        return highlight.ranges;
      }
    }
    return const <SearchHighlightRange>[];
  }

  void _apply(SearchResult result, {required bool append}) {
    _courses = _mergeById(
      append ? _courses : const <CourseSearchHit>[],
      result.courses,
      (CourseSearchHit item) => item.id,
    );
    _reviews = _mergeById(
      append ? _reviews : const <ReviewSearchHit>[],
      result.reviews,
      (ReviewSearchHit item) => item.id,
    );
    _threads = _mergeById(
      append ? _threads : const <ThreadSearchHit>[],
      result.threads,
      (ThreadSearchHit item) => item.id,
    );
    _users = _mergeById(
      append ? _users : const <UserSearchHit>[],
      result.users,
      (UserSearchHit item) => item.id,
    );
    _boards = _mergeById(
      append ? _boards : const <BoardSearchHit>[],
      result.boards,
      (BoardSearchHit item) => item.id,
    );
    _tags = _mergeById(
      append ? _tags : const <TagSearchHit>[],
      result.tags,
      (TagSearchHit item) => item.id,
    );
    _highlights = <SearchHighlight>[
      if (append) ..._highlights,
      ...result.highlights,
    ];
    _moreScopes = result.hasMoreScopes.toSet();
    _failedScopes = <SearchResultScope>{
      if (append) ..._failedScopes,
      ...result.failedScopes,
    };
    _suggestedQuery ??= result.suggestedQuery;
    if (!append) {
      _suggestedQuery = result.suggestedQuery;
    }
    _nextCursor = result.nextCursor;
    _hasMore = result.hasMore && result.nextCursor != null;
  }

  List<T> _mergeById<T>(
    List<T> existing,
    List<T> incoming,
    String Function(T item) id,
  ) {
    final Set<String> known = existing.map(id).toSet();
    return <T>[...existing, ...incoming.where((T item) => known.add(id(item)))];
  }

  void _cancelAndClear() {
    ++_generation;
    _request?.cancel('search cleared');
    _isLoading = false;
    _isLoadingMore = false;
    _failure = null;
    _clearResults();
  }

  void _clearResults() {
    _courses = const <CourseSearchHit>[];
    _reviews = const <ReviewSearchHit>[];
    _threads = const <ThreadSearchHit>[];
    _users = const <UserSearchHit>[];
    _boards = const <BoardSearchHit>[];
    _tags = const <TagSearchHit>[];
    _highlights = const <SearchHighlight>[];
    _moreScopes = const <SearchResultScope>{};
    _failedScopes = const <SearchResultScope>{};
    _suggestedQuery = null;
    _nextCursor = null;
    _hasMore = false;
  }

  bool _isCurrent(int generation, CancelToken request) {
    return !_isDisposed &&
        generation == _generation &&
        identical(_request, request);
  }

  @override
  void dispose() {
    _isDisposed = true;
    _request?.cancel('search disposed');
    super.dispose();
  }
}
