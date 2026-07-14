// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'moderation_preview_grant.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ModerationPreviewGrant _$ModerationPreviewGrantFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('ModerationPreviewGrant', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['token', 'expiresAt']);
  final val = ModerationPreviewGrant(
    token: $checkedConvert('token', (v) => v as String),
    expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$ModerationPreviewGrantToJson(
  ModerationPreviewGrant instance,
) => <String, dynamic>{
  'token': instance.token,
  'expiresAt': instance.expiresAt,
};
