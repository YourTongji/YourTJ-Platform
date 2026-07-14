// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'search_result.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SearchResult _$SearchResultFromJson(Map<String, dynamic> json) =>
    $checkedCreate('SearchResult', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'courses',
          'reviews',
          'threads',
          'users',
          'boards',
          'tags',
          'nextCursor',
          'hasMore',
          'hasMoreScopes',
          'failedScopes',
          'highlights',
          'suggestedQuery',
        ],
      );
      final val = SearchResult(
        courses: $checkedConvert(
          'courses',
          (v) => (v as List<dynamic>)
              .map((e) => CourseSearchHit.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        reviews: $checkedConvert(
          'reviews',
          (v) => (v as List<dynamic>)
              .map((e) => ReviewSearchHit.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        threads: $checkedConvert(
          'threads',
          (v) => (v as List<dynamic>)
              .map((e) => ThreadSearchHit.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        users: $checkedConvert(
          'users',
          (v) => (v as List<dynamic>)
              .map((e) => UserSearchHit.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        boards: $checkedConvert(
          'boards',
          (v) => (v as List<dynamic>)
              .map((e) => BoardSearchHit.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        tags: $checkedConvert(
          'tags',
          (v) => (v as List<dynamic>)
              .map((e) => TagSearchHit.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
        hasMoreScopes: $checkedConvert(
          'hasMoreScopes',
          (v) => (v as List<dynamic>)
              .map((e) => $enumDecode(_$SearchResultScopeEnumMap, e))
              .toList(),
        ),
        failedScopes: $checkedConvert(
          'failedScopes',
          (v) => (v as List<dynamic>)
              .map((e) => $enumDecode(_$SearchResultScopeEnumMap, e))
              .toList(),
        ),
        highlights: $checkedConvert(
          'highlights',
          (v) => (v as List<dynamic>)
              .map((e) => SearchHighlight.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        suggestedQuery: $checkedConvert('suggestedQuery', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$SearchResultToJson(SearchResult instance) =>
    <String, dynamic>{
      'courses': instance.courses.map((e) => e.toJson()).toList(),
      'reviews': instance.reviews.map((e) => e.toJson()).toList(),
      'threads': instance.threads.map((e) => e.toJson()).toList(),
      'users': instance.users.map((e) => e.toJson()).toList(),
      'boards': instance.boards.map((e) => e.toJson()).toList(),
      'tags': instance.tags.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
      'hasMoreScopes': instance.hasMoreScopes
          .map((e) => _$SearchResultScopeEnumMap[e]!)
          .toList(),
      'failedScopes': instance.failedScopes
          .map((e) => _$SearchResultScopeEnumMap[e]!)
          .toList(),
      'highlights': instance.highlights.map((e) => e.toJson()).toList(),
      'suggestedQuery': instance.suggestedQuery,
    };

const _$SearchResultScopeEnumMap = {
  SearchResultScope.course: 'course',
  SearchResultScope.review: 'review',
  SearchResultScope.thread: 'thread',
  SearchResultScope.user: 'user',
  SearchResultScope.board: 'board',
  SearchResultScope.tag: 'tag',
  SearchResultScope.unknownDefaultOpenApi: 'unknown_default_open_api',
};
