// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'upload_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UploadPage _$UploadPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UploadPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = UploadPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => Upload.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$UploadPageToJson(UploadPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
