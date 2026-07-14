// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'me_governance_notices_unread_count_get200_response.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MeGovernanceNoticesUnreadCountGet200Response
_$MeGovernanceNoticesUnreadCountGet200ResponseFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('MeGovernanceNoticesUnreadCountGet200Response', json, (
  $checkedConvert,
) {
  $checkKeys(json, requiredKeys: const ['count']);
  final val = MeGovernanceNoticesUnreadCountGet200Response(
    count: $checkedConvert('count', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$MeGovernanceNoticesUnreadCountGet200ResponseToJson(
  MeGovernanceNoticesUnreadCountGet200Response instance,
) => <String, dynamic>{'count': instance.count};
