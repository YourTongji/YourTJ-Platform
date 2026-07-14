// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'announcement_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AnnouncementUpdateInput _$AnnouncementUpdateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AnnouncementUpdateInput', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'title',
      'status',
      'presentation',
      'severity',
      'priority',
      'audience',
      'requiresAck',
      'reason',
      'expectedVersion',
      'bumpRevision',
    ],
  );
  final val = AnnouncementUpdateInput(
    title: $checkedConvert('title', (v) => v as String),
    body: $checkedConvert('body', (v) => v as String?),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$AnnouncementUpdateInputStatusEnumEnumMap,
        v,
        unknownValue: AnnouncementUpdateInputStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    presentation: $checkedConvert(
      'presentation',
      (v) => $enumDecode(
        _$AnnouncementUpdateInputPresentationEnumEnumMap,
        v,
        unknownValue:
            AnnouncementUpdateInputPresentationEnum.unknownDefaultOpenApi,
      ),
    ),
    severity: $checkedConvert(
      'severity',
      (v) => $enumDecode(
        _$AnnouncementUpdateInputSeverityEnumEnumMap,
        v,
        unknownValue: AnnouncementUpdateInputSeverityEnum.unknownDefaultOpenApi,
      ),
    ),
    priority: $checkedConvert('priority', (v) => (v as num).toInt()),
    audience: $checkedConvert(
      'audience',
      (v) => $enumDecode(
        _$AnnouncementUpdateInputAudienceEnumEnumMap,
        v,
        unknownValue: AnnouncementUpdateInputAudienceEnum.unknownDefaultOpenApi,
      ),
    ),
    requiresAck: $checkedConvert('requiresAck', (v) => v as bool),
    startsAt: $checkedConvert('startsAt', (v) => (v as num?)?.toInt()),
    endsAt: $checkedConvert('endsAt', (v) => (v as num?)?.toInt()),
    reason: $checkedConvert('reason', (v) => v as String),
    expectedVersion: $checkedConvert(
      'expectedVersion',
      (v) => (v as num).toInt(),
    ),
    bumpRevision: $checkedConvert('bumpRevision', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$AnnouncementUpdateInputToJson(
  AnnouncementUpdateInput instance,
) => <String, dynamic>{
  'title': instance.title,
  'body': ?instance.body,
  'status': _$AnnouncementUpdateInputStatusEnumEnumMap[instance.status]!,
  'presentation':
      _$AnnouncementUpdateInputPresentationEnumEnumMap[instance.presentation]!,
  'severity': _$AnnouncementUpdateInputSeverityEnumEnumMap[instance.severity]!,
  'priority': instance.priority,
  'audience': _$AnnouncementUpdateInputAudienceEnumEnumMap[instance.audience]!,
  'requiresAck': instance.requiresAck,
  'startsAt': ?instance.startsAt,
  'endsAt': ?instance.endsAt,
  'reason': instance.reason,
  'expectedVersion': instance.expectedVersion,
  'bumpRevision': instance.bumpRevision,
};

const _$AnnouncementUpdateInputStatusEnumEnumMap = {
  AnnouncementUpdateInputStatusEnum.draft: 'draft',
  AnnouncementUpdateInputStatusEnum.scheduled: 'scheduled',
  AnnouncementUpdateInputStatusEnum.published: 'published',
  AnnouncementUpdateInputStatusEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AnnouncementUpdateInputPresentationEnumEnumMap = {
  AnnouncementUpdateInputPresentationEnum.card: 'card',
  AnnouncementUpdateInputPresentationEnum.banner: 'banner',
  AnnouncementUpdateInputPresentationEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AnnouncementUpdateInputSeverityEnumEnumMap = {
  AnnouncementUpdateInputSeverityEnum.info: 'info',
  AnnouncementUpdateInputSeverityEnum.success: 'success',
  AnnouncementUpdateInputSeverityEnum.warning: 'warning',
  AnnouncementUpdateInputSeverityEnum.critical: 'critical',
  AnnouncementUpdateInputSeverityEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AnnouncementUpdateInputAudienceEnumEnumMap = {
  AnnouncementUpdateInputAudienceEnum.all: 'all',
  AnnouncementUpdateInputAudienceEnum.authenticated: 'authenticated',
  AnnouncementUpdateInputAudienceEnum.staff: 'staff',
  AnnouncementUpdateInputAudienceEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
