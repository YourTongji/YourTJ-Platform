// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'announcement_create_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AnnouncementCreateInput _$AnnouncementCreateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AnnouncementCreateInput', json, ($checkedConvert) {
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
    ],
  );
  final val = AnnouncementCreateInput(
    title: $checkedConvert('title', (v) => v as String),
    body: $checkedConvert('body', (v) => v as String?),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$AnnouncementCreateInputStatusEnumEnumMap,
        v,
        unknownValue: AnnouncementCreateInputStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    presentation: $checkedConvert(
      'presentation',
      (v) => $enumDecode(
        _$AnnouncementCreateInputPresentationEnumEnumMap,
        v,
        unknownValue:
            AnnouncementCreateInputPresentationEnum.unknownDefaultOpenApi,
      ),
    ),
    severity: $checkedConvert(
      'severity',
      (v) => $enumDecode(
        _$AnnouncementCreateInputSeverityEnumEnumMap,
        v,
        unknownValue: AnnouncementCreateInputSeverityEnum.unknownDefaultOpenApi,
      ),
    ),
    priority: $checkedConvert('priority', (v) => (v as num).toInt()),
    audience: $checkedConvert(
      'audience',
      (v) => $enumDecode(
        _$AnnouncementCreateInputAudienceEnumEnumMap,
        v,
        unknownValue: AnnouncementCreateInputAudienceEnum.unknownDefaultOpenApi,
      ),
    ),
    requiresAck: $checkedConvert('requiresAck', (v) => v as bool),
    startsAt: $checkedConvert('startsAt', (v) => (v as num?)?.toInt()),
    endsAt: $checkedConvert('endsAt', (v) => (v as num?)?.toInt()),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AnnouncementCreateInputToJson(
  AnnouncementCreateInput instance,
) => <String, dynamic>{
  'title': instance.title,
  'body': ?instance.body,
  'status': _$AnnouncementCreateInputStatusEnumEnumMap[instance.status]!,
  'presentation':
      _$AnnouncementCreateInputPresentationEnumEnumMap[instance.presentation]!,
  'severity': _$AnnouncementCreateInputSeverityEnumEnumMap[instance.severity]!,
  'priority': instance.priority,
  'audience': _$AnnouncementCreateInputAudienceEnumEnumMap[instance.audience]!,
  'requiresAck': instance.requiresAck,
  'startsAt': ?instance.startsAt,
  'endsAt': ?instance.endsAt,
  'reason': instance.reason,
};

const _$AnnouncementCreateInputStatusEnumEnumMap = {
  AnnouncementCreateInputStatusEnum.draft: 'draft',
  AnnouncementCreateInputStatusEnum.scheduled: 'scheduled',
  AnnouncementCreateInputStatusEnum.published: 'published',
  AnnouncementCreateInputStatusEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AnnouncementCreateInputPresentationEnumEnumMap = {
  AnnouncementCreateInputPresentationEnum.card: 'card',
  AnnouncementCreateInputPresentationEnum.banner: 'banner',
  AnnouncementCreateInputPresentationEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AnnouncementCreateInputSeverityEnumEnumMap = {
  AnnouncementCreateInputSeverityEnum.info: 'info',
  AnnouncementCreateInputSeverityEnum.success: 'success',
  AnnouncementCreateInputSeverityEnum.warning: 'warning',
  AnnouncementCreateInputSeverityEnum.critical: 'critical',
  AnnouncementCreateInputSeverityEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AnnouncementCreateInputAudienceEnumEnumMap = {
  AnnouncementCreateInputAudienceEnum.all: 'all',
  AnnouncementCreateInputAudienceEnum.authenticated: 'authenticated',
  AnnouncementCreateInputAudienceEnum.staff: 'staff',
  AnnouncementCreateInputAudienceEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
