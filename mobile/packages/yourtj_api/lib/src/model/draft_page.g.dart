// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'draft_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DraftPage _$DraftPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DraftPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = DraftPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => DraftOutput.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$DraftPageToJson(DraftPage instance) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
