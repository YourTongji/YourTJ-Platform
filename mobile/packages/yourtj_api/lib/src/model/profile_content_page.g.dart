// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'profile_content_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ProfileContentPage _$ProfileContentPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ProfileContentPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = ProfileContentPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => ProfileContent.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$ProfileContentPageToJson(ProfileContentPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
