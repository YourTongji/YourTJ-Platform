// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'selection_offering_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SelectionOfferingPage _$SelectionOfferingPageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('SelectionOfferingPage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = SelectionOfferingPage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map((e) => SelectionOffering.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$SelectionOfferingPageToJson(
  SelectionOfferingPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
