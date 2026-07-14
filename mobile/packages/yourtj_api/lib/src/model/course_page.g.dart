// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'course_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CoursePage _$CoursePageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('CoursePage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = CoursePage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => Course.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$CoursePageToJson(CoursePage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
