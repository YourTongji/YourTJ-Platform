// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_summary_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UserSummaryPage _$UserSummaryPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UserSummaryPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = UserSummaryPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => UserSummary.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$UserSummaryPageToJson(UserSummaryPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
