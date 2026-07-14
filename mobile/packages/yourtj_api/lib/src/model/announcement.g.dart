// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'announcement.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Announcement _$AnnouncementFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Announcement', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'title',
          'status',
          'effectiveState',
          'presentation',
          'severity',
          'priority',
          'audience',
          'requiresAck',
          'version',
          'revision',
          'createdAt',
          'updatedAt',
        ],
      );
      final val = Announcement(
        id: $checkedConvert('id', (v) => v as String),
        title: $checkedConvert('title', (v) => v as String),
        body: $checkedConvert('body', (v) => v as String?),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$AnnouncementStatusEnumEnumMap,
            v,
            unknownValue: AnnouncementStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        effectiveState: $checkedConvert(
          'effectiveState',
          (v) => $enumDecode(
            _$AnnouncementEffectiveStateEnumEnumMap,
            v,
            unknownValue: AnnouncementEffectiveStateEnum.unknownDefaultOpenApi,
          ),
        ),
        presentation: $checkedConvert(
          'presentation',
          (v) => $enumDecode(
            _$AnnouncementPresentationEnumEnumMap,
            v,
            unknownValue: AnnouncementPresentationEnum.unknownDefaultOpenApi,
          ),
        ),
        severity: $checkedConvert(
          'severity',
          (v) => $enumDecode(
            _$AnnouncementSeverityEnumEnumMap,
            v,
            unknownValue: AnnouncementSeverityEnum.unknownDefaultOpenApi,
          ),
        ),
        priority: $checkedConvert('priority', (v) => (v as num).toInt()),
        audience: $checkedConvert(
          'audience',
          (v) => $enumDecode(
            _$AnnouncementAudienceEnumEnumMap,
            v,
            unknownValue: AnnouncementAudienceEnum.unknownDefaultOpenApi,
          ),
        ),
        requiresAck: $checkedConvert('requiresAck', (v) => v as bool),
        version: $checkedConvert('version', (v) => (v as num).toInt()),
        revision: $checkedConvert('revision', (v) => (v as num).toInt()),
        startsAt: $checkedConvert('startsAt', (v) => (v as num?)?.toInt()),
        endsAt: $checkedConvert('endsAt', (v) => (v as num?)?.toInt()),
        publishedAt: $checkedConvert(
          'publishedAt',
          (v) => (v as num?)?.toInt(),
        ),
        archivedAt: $checkedConvert('archivedAt', (v) => (v as num?)?.toInt()),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
        updatedAt: $checkedConvert('updatedAt', (v) => (v as num).toInt()),
        receipt: $checkedConvert(
          'receipt',
          (v) => v == null
              ? null
              : AnnouncementReceipt.fromJson(v as Map<String, dynamic>),
        ),
        receiptSummary: $checkedConvert(
          'receiptSummary',
          (v) => v == null
              ? null
              : AnnouncementReceiptSummary.fromJson(v as Map<String, dynamic>),
        ),
      );
      return val;
    });

Map<String, dynamic> _$AnnouncementToJson(
  Announcement instance,
) => <String, dynamic>{
  'id': instance.id,
  'title': instance.title,
  'body': ?instance.body,
  'status': _$AnnouncementStatusEnumEnumMap[instance.status]!,
  'effectiveState':
      _$AnnouncementEffectiveStateEnumEnumMap[instance.effectiveState]!,
  'presentation': _$AnnouncementPresentationEnumEnumMap[instance.presentation]!,
  'severity': _$AnnouncementSeverityEnumEnumMap[instance.severity]!,
  'priority': instance.priority,
  'audience': _$AnnouncementAudienceEnumEnumMap[instance.audience]!,
  'requiresAck': instance.requiresAck,
  'version': instance.version,
  'revision': instance.revision,
  'startsAt': ?instance.startsAt,
  'endsAt': ?instance.endsAt,
  'publishedAt': ?instance.publishedAt,
  'archivedAt': ?instance.archivedAt,
  'createdAt': instance.createdAt,
  'updatedAt': instance.updatedAt,
  'receipt': ?instance.receipt?.toJson(),
  'receiptSummary': ?instance.receiptSummary?.toJson(),
};

const _$AnnouncementStatusEnumEnumMap = {
  AnnouncementStatusEnum.draft: 'draft',
  AnnouncementStatusEnum.scheduled: 'scheduled',
  AnnouncementStatusEnum.published: 'published',
  AnnouncementStatusEnum.archived: 'archived',
  AnnouncementStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$AnnouncementEffectiveStateEnumEnumMap = {
  AnnouncementEffectiveStateEnum.draft: 'draft',
  AnnouncementEffectiveStateEnum.scheduled: 'scheduled',
  AnnouncementEffectiveStateEnum.active: 'active',
  AnnouncementEffectiveStateEnum.expired: 'expired',
  AnnouncementEffectiveStateEnum.archived: 'archived',
  AnnouncementEffectiveStateEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AnnouncementPresentationEnumEnumMap = {
  AnnouncementPresentationEnum.card: 'card',
  AnnouncementPresentationEnum.banner: 'banner',
  AnnouncementPresentationEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$AnnouncementSeverityEnumEnumMap = {
  AnnouncementSeverityEnum.info: 'info',
  AnnouncementSeverityEnum.success: 'success',
  AnnouncementSeverityEnum.warning: 'warning',
  AnnouncementSeverityEnum.critical: 'critical',
  AnnouncementSeverityEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$AnnouncementAudienceEnumEnumMap = {
  AnnouncementAudienceEnum.all: 'all',
  AnnouncementAudienceEnum.authenticated: 'authenticated',
  AnnouncementAudienceEnum.staff: 'staff',
  AnnouncementAudienceEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
