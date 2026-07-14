// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'verification_grant_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

VerificationGrantPage _$VerificationGrantPageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('VerificationGrantPage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = VerificationGrantPage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map((e) => VerificationGrant.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$VerificationGrantPageToJson(
  VerificationGrantPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
