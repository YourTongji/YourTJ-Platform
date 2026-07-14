// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'trust_level_policy_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TrustLevelPolicyPage _$TrustLevelPolicyPageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('TrustLevelPolicyPage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = TrustLevelPolicyPage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map((e) => TrustLevelPolicy.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$TrustLevelPolicyPageToJson(
  TrustLevelPolicyPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
