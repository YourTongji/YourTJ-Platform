// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'my_upload_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MyUploadPage _$MyUploadPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('MyUploadPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = MyUploadPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => MyUpload.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$MyUploadPageToJson(MyUploadPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
