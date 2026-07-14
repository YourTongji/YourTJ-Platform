// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'activity_policy_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ActivityPolicyPage _$ActivityPolicyPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ActivityPolicyPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = ActivityPolicyPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => ActivityPolicy.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$ActivityPolicyPageToJson(ActivityPolicyPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
