// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'promotion.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Promotion _$PromotionFromJson(Map<String, dynamic> json) =>
    $checkedCreate('Promotion', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'placement',
          'title',
          'targetUrl',
          'assetDelivery',
          'status',
          'effectiveState',
          'priority',
          'audience',
          'version',
          'createdAt',
          'updatedAt',
          'trackingToken',
          'metrics',
        ],
      );
      final val = Promotion(
        id: $checkedConvert('id', (v) => v as String),
        placement: $checkedConvert(
          'placement',
          (v) => $enumDecode(
            _$PromotionPlacementEnumEnumMap,
            v,
            unknownValue: PromotionPlacementEnum.unknownDefaultOpenApi,
          ),
        ),
        title: $checkedConvert('title', (v) => v as String),
        body: $checkedConvert('body', (v) => v as String?),
        ctaLabel: $checkedConvert('ctaLabel', (v) => v as String?),
        targetUrl: $checkedConvert('targetUrl', (v) => v as String),
        assetId: $checkedConvert('assetId', (v) => v as String?),
        assetDelivery: $checkedConvert(
          'assetDelivery',
          (v) => v == null
              ? null
              : MediaDelivery.fromJson(v as Map<String, dynamic>),
        ),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$PromotionStatusEnumEnumMap,
            v,
            unknownValue: PromotionStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        effectiveState: $checkedConvert(
          'effectiveState',
          (v) => $enumDecode(
            _$PromotionEffectiveStateEnumEnumMap,
            v,
            unknownValue: PromotionEffectiveStateEnum.unknownDefaultOpenApi,
          ),
        ),
        priority: $checkedConvert('priority', (v) => (v as num).toInt()),
        audience: $checkedConvert(
          'audience',
          (v) => $enumDecode(
            _$PromotionAudienceEnumEnumMap,
            v,
            unknownValue: PromotionAudienceEnum.unknownDefaultOpenApi,
          ),
        ),
        version: $checkedConvert('version', (v) => (v as num).toInt()),
        startsAt: $checkedConvert('startsAt', (v) => (v as num?)?.toInt()),
        endsAt: $checkedConvert('endsAt', (v) => (v as num?)?.toInt()),
        archivedAt: $checkedConvert('archivedAt', (v) => (v as num?)?.toInt()),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
        updatedAt: $checkedConvert('updatedAt', (v) => (v as num).toInt()),
        trackingToken: $checkedConvert('trackingToken', (v) => v as String?),
        metrics: $checkedConvert(
          'metrics',
          (v) => v == null
              ? null
              : PromotionMetricSummary.fromJson(v as Map<String, dynamic>),
        ),
      );
      return val;
    });

Map<String, dynamic> _$PromotionToJson(Promotion instance) => <String, dynamic>{
  'id': instance.id,
  'placement': _$PromotionPlacementEnumEnumMap[instance.placement]!,
  'title': instance.title,
  'body': ?instance.body,
  'ctaLabel': ?instance.ctaLabel,
  'targetUrl': instance.targetUrl,
  'assetId': ?instance.assetId,
  'assetDelivery': instance.assetDelivery?.toJson(),
  'status': _$PromotionStatusEnumEnumMap[instance.status]!,
  'effectiveState':
      _$PromotionEffectiveStateEnumEnumMap[instance.effectiveState]!,
  'priority': instance.priority,
  'audience': _$PromotionAudienceEnumEnumMap[instance.audience]!,
  'version': instance.version,
  'startsAt': ?instance.startsAt,
  'endsAt': ?instance.endsAt,
  'archivedAt': ?instance.archivedAt,
  'createdAt': instance.createdAt,
  'updatedAt': instance.updatedAt,
  'trackingToken': instance.trackingToken,
  'metrics': instance.metrics?.toJson(),
};

const _$PromotionPlacementEnumEnumMap = {
  PromotionPlacementEnum.homeLeftPrimary: 'home-left-primary',
  PromotionPlacementEnum.homeLeftSecondary: 'home-left-secondary',
  PromotionPlacementEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$PromotionStatusEnumEnumMap = {
  PromotionStatusEnum.draft: 'draft',
  PromotionStatusEnum.scheduled: 'scheduled',
  PromotionStatusEnum.published: 'published',
  PromotionStatusEnum.paused: 'paused',
  PromotionStatusEnum.archived: 'archived',
  PromotionStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$PromotionEffectiveStateEnumEnumMap = {
  PromotionEffectiveStateEnum.draft: 'draft',
  PromotionEffectiveStateEnum.scheduled: 'scheduled',
  PromotionEffectiveStateEnum.active: 'active',
  PromotionEffectiveStateEnum.paused: 'paused',
  PromotionEffectiveStateEnum.expired: 'expired',
  PromotionEffectiveStateEnum.archived: 'archived',
  PromotionEffectiveStateEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$PromotionAudienceEnumEnumMap = {
  PromotionAudienceEnum.all: 'all',
  PromotionAudienceEnum.authenticated: 'authenticated',
  PromotionAudienceEnum.staff: 'staff',
  PromotionAudienceEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
