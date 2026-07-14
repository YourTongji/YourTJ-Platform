// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'governance_notice_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

GovernanceNoticePage _$GovernanceNoticePageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('GovernanceNoticePage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = GovernanceNoticePage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map((e) => GovernanceNotice.fromJson(e as Map<String, dynamic>))
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$GovernanceNoticePageToJson(
  GovernanceNoticePage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
