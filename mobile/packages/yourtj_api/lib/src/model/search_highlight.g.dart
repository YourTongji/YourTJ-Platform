// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'search_highlight.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SearchHighlight _$SearchHighlightFromJson(Map<String, dynamic> json) =>
    $checkedCreate('SearchHighlight', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['scope', 'id', 'field', 'ranges']);
      final val = SearchHighlight(
        scope: $checkedConvert(
          'scope',
          (v) => $enumDecode(
            _$SearchResultScopeEnumMap,
            v,
            unknownValue: SearchResultScope.unknownDefaultOpenApi,
          ),
        ),
        id: $checkedConvert('id', (v) => v as String),
        field: $checkedConvert(
          'field',
          (v) => $enumDecode(
            _$SearchHighlightFieldEnumEnumMap,
            v,
            unknownValue: SearchHighlightFieldEnum.unknownDefaultOpenApi,
          ),
        ),
        ranges: $checkedConvert(
          'ranges',
          (v) => (v as List<dynamic>)
              .map(
                (e) => SearchHighlightRange.fromJson(e as Map<String, dynamic>),
              )
              .toList(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$SearchHighlightToJson(SearchHighlight instance) =>
    <String, dynamic>{
      'scope': _$SearchResultScopeEnumMap[instance.scope]!,
      'id': instance.id,
      'field': _$SearchHighlightFieldEnumEnumMap[instance.field]!,
      'ranges': instance.ranges.map((e) => e.toJson()).toList(),
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

const _$SearchHighlightFieldEnumEnumMap = {
  SearchHighlightFieldEnum.name: 'name',
  SearchHighlightFieldEnum.code: 'code',
  SearchHighlightFieldEnum.teacherName: 'teacherName',
  SearchHighlightFieldEnum.department: 'department',
  SearchHighlightFieldEnum.courseName: 'courseName',
  SearchHighlightFieldEnum.comment: 'comment',
  SearchHighlightFieldEnum.title: 'title',
  SearchHighlightFieldEnum.bodyExcerpt: 'bodyExcerpt',
  SearchHighlightFieldEnum.board: 'board',
  SearchHighlightFieldEnum.authorHandle: 'authorHandle',
  SearchHighlightFieldEnum.handle: 'handle',
  SearchHighlightFieldEnum.displayName: 'displayName',
  SearchHighlightFieldEnum.slug: 'slug',
  SearchHighlightFieldEnum.description: 'description',
  SearchHighlightFieldEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
