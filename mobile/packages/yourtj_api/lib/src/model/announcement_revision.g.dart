// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'announcement_revision.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AnnouncementRevision _$AnnouncementRevisionFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AnnouncementRevision', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'announcementId',
      'version',
      'revision',
      'title',
      'status',
      'presentation',
      'severity',
      'priority',
      'audience',
      'requiresAck',
      'createdAt',
    ],
  );
  final val = AnnouncementRevision(
    announcementId: $checkedConvert('announcementId', (v) => v as String),
    version: $checkedConvert('version', (v) => (v as num).toInt()),
    revision: $checkedConvert('revision', (v) => (v as num).toInt()),
    title: $checkedConvert('title', (v) => v as String),
    body: $checkedConvert('body', (v) => v as String?),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$AnnouncementRevisionStatusEnumEnumMap,
        v,
        unknownValue: AnnouncementRevisionStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    presentation: $checkedConvert(
      'presentation',
      (v) => $enumDecode(
        _$AnnouncementRevisionPresentationEnumEnumMap,
        v,
        unknownValue:
            AnnouncementRevisionPresentationEnum.unknownDefaultOpenApi,
      ),
    ),
    severity: $checkedConvert(
      'severity',
      (v) => $enumDecode(
        _$AnnouncementRevisionSeverityEnumEnumMap,
        v,
        unknownValue: AnnouncementRevisionSeverityEnum.unknownDefaultOpenApi,
      ),
    ),
    priority: $checkedConvert('priority', (v) => (v as num).toInt()),
    audience: $checkedConvert(
      'audience',
      (v) => $enumDecode(
        _$AnnouncementRevisionAudienceEnumEnumMap,
        v,
        unknownValue: AnnouncementRevisionAudienceEnum.unknownDefaultOpenApi,
      ),
    ),
    requiresAck: $checkedConvert('requiresAck', (v) => v as bool),
    startsAt: $checkedConvert('startsAt', (v) => (v as num?)?.toInt()),
    endsAt: $checkedConvert('endsAt', (v) => (v as num?)?.toInt()),
    createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$AnnouncementRevisionToJson(
  AnnouncementRevision instance,
) => <String, dynamic>{
  'announcementId': instance.announcementId,
  'version': instance.version,
  'revision': instance.revision,
  'title': instance.title,
  'body': ?instance.body,
  'status': _$AnnouncementRevisionStatusEnumEnumMap[instance.status]!,
  'presentation':
      _$AnnouncementRevisionPresentationEnumEnumMap[instance.presentation]!,
  'severity': _$AnnouncementRevisionSeverityEnumEnumMap[instance.severity]!,
  'priority': instance.priority,
  'audience': _$AnnouncementRevisionAudienceEnumEnumMap[instance.audience]!,
  'requiresAck': instance.requiresAck,
  'startsAt': ?instance.startsAt,
  'endsAt': ?instance.endsAt,
  'createdAt': instance.createdAt,
};

const _$AnnouncementRevisionStatusEnumEnumMap = {
  AnnouncementRevisionStatusEnum.draft: 'draft',
  AnnouncementRevisionStatusEnum.scheduled: 'scheduled',
  AnnouncementRevisionStatusEnum.published: 'published',
  AnnouncementRevisionStatusEnum.archived: 'archived',
  AnnouncementRevisionStatusEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AnnouncementRevisionPresentationEnumEnumMap = {
  AnnouncementRevisionPresentationEnum.card: 'card',
  AnnouncementRevisionPresentationEnum.banner: 'banner',
  AnnouncementRevisionPresentationEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AnnouncementRevisionSeverityEnumEnumMap = {
  AnnouncementRevisionSeverityEnum.info: 'info',
  AnnouncementRevisionSeverityEnum.success: 'success',
  AnnouncementRevisionSeverityEnum.warning: 'warning',
  AnnouncementRevisionSeverityEnum.critical: 'critical',
  AnnouncementRevisionSeverityEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AnnouncementRevisionAudienceEnumEnumMap = {
  AnnouncementRevisionAudienceEnum.all: 'all',
  AnnouncementRevisionAudienceEnum.authenticated: 'authenticated',
  AnnouncementRevisionAudienceEnum.staff: 'staff',
  AnnouncementRevisionAudienceEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
