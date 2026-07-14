// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'verification_type_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

VerificationTypePage _$VerificationTypePageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('VerificationTypePage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = VerificationTypePage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map((e) => VerificationType.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$VerificationTypePageToJson(
  VerificationTypePage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
