import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../data/profile_activity_repository.dart';

enum ProfileActivityTab {
  threads('主题'),
  comments('回复'),
  media('媒体'),
  likes('喜欢');

  const ProfileActivityTab(this.label);

  final String label;
}

class ProfileActivityListState<T> {
  const ProfileActivityListState({
    this.items = const <Never>[],
    this.nextCursor,
    this.hasMore = false,
    this.hasLoaded = false,
    this.isLoading = false,
    this.isLoadingMore = false,
    this.failure,
  });

  final List<T> items;
  final String? nextCursor;
  final bool hasMore;
  final bool hasLoaded;
  final bool isLoading;
  final bool isLoadingMore;
  final ApiFailure? failure;

  ProfileActivityListState<T> copyWith({
    List<T>? items,
    String? nextCursor,
    bool clearCursor = false,
    bool? hasMore,
    bool? hasLoaded,
    bool? isLoading,
    bool? isLoadingMore,
    ApiFailure? failure,
    bool clearFailure = false,
  }) {
    return ProfileActivityListState<T>(
      items: items ?? this.items,
      nextCursor: clearCursor ? null : nextCursor ?? this.nextCursor,
      hasMore: hasMore ?? this.hasMore,
      hasLoaded: hasLoaded ?? this.hasLoaded,
      isLoading: isLoading ?? this.isLoading,
      isLoadingMore: isLoadingMore ?? this.isLoadingMore,
      failure: clearFailure ? null : failure ?? this.failure,
    );
  }
}

class ProfileActivityController extends ChangeNotifier {
  ProfileActivityController(this._source);

  final ProfileActivitySource _source;
  final _ProfileActivityBucket<UserThread> _threads =
      _ProfileActivityBucket<UserThread>((UserThread item) => item.id);
  final _ProfileActivityBucket<UserComment> _comments =
      _ProfileActivityBucket<UserComment>((UserComment item) => item.id);
  final _ProfileActivityBucket<ProfileContent> _media =
      _ProfileActivityBucket<ProfileContent>(
        (ProfileContent item) => '${item.targetType.value}:${item.id}',
      );
  final _ProfileActivityBucket<ProfileContent> _likes =
      _ProfileActivityBucket<ProfileContent>(
        (ProfileContent item) => '${item.targetType.value}:${item.id}',
      );

  String? _handle;
  String? _viewerKey;
  bool _canViewActivity = false;
  bool _isDisposed = false;
  int _contextRevision = 0;
  ProfileActivityTab _selectedTab = ProfileActivityTab.threads;

  String? get handle => _handle;
  bool get canViewActivity => _canViewActivity;
  ProfileActivityTab get selectedTab => _selectedTab;
  ProfileActivityListState<UserThread> get threads => _threads.state;
  ProfileActivityListState<UserComment> get comments => _comments.state;
  ProfileActivityListState<ProfileContent> get media => _media.state;
  ProfileActivityListState<ProfileContent> get likes => _likes.state;

  bool configure({
    required String handle,
    required String viewerKey,
    required bool canViewActivity,
  }) {
    final String normalizedHandle = handle.trim().toLowerCase();
    if (_handle == normalizedHandle &&
        _viewerKey == viewerKey &&
        _canViewActivity == canViewActivity) {
      return false;
    }
    _handle = normalizedHandle;
    _viewerKey = viewerKey;
    _canViewActivity = canViewActivity;
    _contextRevision += 1;
    _resetBucket(_threads);
    _resetBucket(_comments);
    _resetBucket(_media);
    _resetBucket(_likes);
    _notify();
    return true;
  }

  void selectTab(ProfileActivityTab tab) {
    if (_selectedTab != tab) {
      _selectedTab = tab;
      _notify();
    }
    final ProfileActivityListState<Object?> state = _stateFor(tab);
    if (_canViewActivity && !state.hasLoaded && !state.isLoading) {
      unawaited(load(tab));
    }
  }

  Future<void> loadSelected({bool refresh = false}) =>
      load(_selectedTab, refresh: refresh);

  Future<void> load(ProfileActivityTab tab, {bool refresh = false}) {
    return switch (tab) {
      ProfileActivityTab.threads => _loadBucket<UserThread>(
        bucket: _threads,
        refresh: refresh,
        request: (String handle, String? cursor) =>
            _source.threads(handle: handle, cursor: cursor),
      ),
      ProfileActivityTab.comments => _loadBucket<UserComment>(
        bucket: _comments,
        refresh: refresh,
        request: (String handle, String? cursor) =>
            _source.comments(handle: handle, cursor: cursor),
      ),
      ProfileActivityTab.media => _loadBucket<ProfileContent>(
        bucket: _media,
        refresh: refresh,
        request: (String handle, String? cursor) =>
            _source.media(handle: handle, cursor: cursor),
      ),
      ProfileActivityTab.likes => _loadBucket<ProfileContent>(
        bucket: _likes,
        refresh: refresh,
        request: (String handle, String? cursor) =>
            _source.likes(handle: handle, cursor: cursor),
      ),
    };
  }

  Future<void> loadMore(ProfileActivityTab tab) {
    return switch (tab) {
      ProfileActivityTab.threads => _loadBucket<UserThread>(
        bucket: _threads,
        loadMore: true,
        request: (String handle, String? cursor) =>
            _source.threads(handle: handle, cursor: cursor),
      ),
      ProfileActivityTab.comments => _loadBucket<UserComment>(
        bucket: _comments,
        loadMore: true,
        request: (String handle, String? cursor) =>
            _source.comments(handle: handle, cursor: cursor),
      ),
      ProfileActivityTab.media => _loadBucket<ProfileContent>(
        bucket: _media,
        loadMore: true,
        request: (String handle, String? cursor) =>
            _source.media(handle: handle, cursor: cursor),
      ),
      ProfileActivityTab.likes => _loadBucket<ProfileContent>(
        bucket: _likes,
        loadMore: true,
        request: (String handle, String? cursor) =>
            _source.likes(handle: handle, cursor: cursor),
      ),
    };
  }

  ProfileActivityListState<Object?> _stateFor(ProfileActivityTab tab) {
    return switch (tab) {
      ProfileActivityTab.threads => _threads.state,
      ProfileActivityTab.comments => _comments.state,
      ProfileActivityTab.media => _media.state,
      ProfileActivityTab.likes => _likes.state,
    };
  }

  Future<void> _loadBucket<T>({
    required _ProfileActivityBucket<T> bucket,
    required Future<ProfileActivityPage<T>> Function(
      String handle,
      String? cursor,
    )
    request,
    bool refresh = false,
    bool loadMore = false,
  }) async {
    final String? handle = _handle;
    if (!_canViewActivity || handle == null || _isDisposed) {
      return;
    }
    final ProfileActivityListState<T> previous = bucket.state;
    if (loadMore) {
      if (previous.isLoadingMore ||
          previous.isLoading ||
          !previous.hasMore ||
          previous.nextCursor == null) {
        return;
      }
    } else if (previous.isLoading || (previous.hasLoaded && !refresh)) {
      return;
    }

    final int contextRevision = _contextRevision;
    final int requestRevision = ++bucket.requestRevision;
    final String? cursor = loadMore ? previous.nextCursor : null;
    bucket.state = previous.copyWith(
      isLoading: !loadMore,
      isLoadingMore: loadMore,
      clearFailure: true,
    );
    _notify();
    try {
      final ProfileActivityPage<T> page = await request(handle, cursor);
      if (!_isCurrent(bucket, contextRevision, requestRevision)) {
        return;
      }
      final List<T> items = loadMore
          ? _deduplicate<T>(<T>[...previous.items, ...page.items], bucket.key)
          : _deduplicate<T>(page.items, bucket.key);
      bucket.state = ProfileActivityListState<T>(
        items: List<T>.unmodifiable(items),
        nextCursor: page.nextCursor,
        hasMore: page.hasMore,
        hasLoaded: true,
      );
      _notify();
    } on ApiFailure catch (failure) {
      if (!_isCurrent(bucket, contextRevision, requestRevision)) {
        return;
      }
      bucket.state = previous.copyWith(
        hasLoaded: true,
        isLoading: false,
        isLoadingMore: false,
        failure: failure,
      );
      _notify();
    } on Object {
      if (!_isCurrent(bucket, contextRevision, requestRevision)) {
        return;
      }
      bucket.state = previous.copyWith(
        hasLoaded: true,
        isLoading: false,
        isLoadingMore: false,
        failure: const ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: '公开动态加载失败，请稍后重试',
        ),
      );
      _notify();
    }
  }

  bool _isCurrent<T>(
    _ProfileActivityBucket<T> bucket,
    int contextRevision,
    int requestRevision,
  ) {
    return !_isDisposed &&
        contextRevision == _contextRevision &&
        requestRevision == bucket.requestRevision;
  }

  List<T> _deduplicate<T>(List<T> items, String Function(T) key) {
    final Set<String> seen = <String>{};
    return items.where((T item) => seen.add(key(item))).toList(growable: false);
  }

  void _resetBucket<T>(_ProfileActivityBucket<T> bucket) {
    bucket.requestRevision += 1;
    bucket.state = ProfileActivityListState<T>();
  }

  void _notify() {
    if (!_isDisposed) {
      notifyListeners();
    }
  }

  @override
  void dispose() {
    _isDisposed = true;
    _contextRevision += 1;
    super.dispose();
  }
}

class _ProfileActivityBucket<T> {
  _ProfileActivityBucket(this.key);

  final String Function(T) key;
  ProfileActivityListState<T> state = ProfileActivityListState<T>();
  int requestRevision = 0;
}
