// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'promotion_update_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PromotionUpdateInput _$PromotionUpdateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('PromotionUpdateInput', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'placement',
      'title',
      'targetUrl',
      'status',
      'priority',
      'audience',
      'reason',
      'expectedVersion',
    ],
  );
  final val = PromotionUpdateInput(
    placement: $checkedConvert(
      'placement',
      (v) => $enumDecode(
        _$PromotionUpdateInputPlacementEnumEnumMap,
        v,
        unknownValue: PromotionUpdateInputPlacementEnum.unknownDefaultOpenApi,
      ),
    ),
    title: $checkedConvert('title', (v) => v as String),
    body: $checkedConvert('body', (v) => v as String?),
    ctaLabel: $checkedConvert('ctaLabel', (v) => v as String?),
    targetUrl: $checkedConvert('targetUrl', (v) => v as String),
    assetId: $checkedConvert('assetId', (v) => v as String?),
    status: $checkedConvert(
      'status',
      (v) => $enumDecode(
        _$PromotionUpdateInputStatusEnumEnumMap,
        v,
        unknownValue: PromotionUpdateInputStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    priority: $checkedConvert('priority', (v) => (v as num).toInt()),
    audience: $checkedConvert(
      'audience',
      (v) => $enumDecode(
        _$PromotionUpdateInputAudienceEnumEnumMap,
        v,
        unknownValue: PromotionUpdateInputAudienceEnum.unknownDefaultOpenApi,
      ),
    ),
    startsAt: $checkedConvert('startsAt', (v) => (v as num?)?.toInt()),
    endsAt: $checkedConvert('endsAt', (v) => (v as num?)?.toInt()),
    reason: $checkedConvert('reason', (v) => v as String),
    expectedVersion: $checkedConvert(
      'expectedVersion',
      (v) => (v as num).toInt(),
    ),
  );
  return val;
});

Map<String, dynamic> _$PromotionUpdateInputToJson(
  PromotionUpdateInput instance,
) => <String, dynamic>{
  'placement': _$PromotionUpdateInputPlacementEnumEnumMap[instance.placement]!,
  'title': instance.title,
  'body': ?instance.body,
  'ctaLabel': ?instance.ctaLabel,
  'targetUrl': instance.targetUrl,
  'assetId': ?instance.assetId,
  'status': _$PromotionUpdateInputStatusEnumEnumMap[instance.status]!,
  'priority': instance.priority,
  'audience': _$PromotionUpdateInputAudienceEnumEnumMap[instance.audience]!,
  'startsAt': ?instance.startsAt,
  'endsAt': ?instance.endsAt,
  'reason': instance.reason,
  'expectedVersion': instance.expectedVersion,
};

const _$PromotionUpdateInputPlacementEnumEnumMap = {
  PromotionUpdateInputPlacementEnum.homeLeftPrimary: 'home-left-primary',
  PromotionUpdateInputPlacementEnum.homeLeftSecondary: 'home-left-secondary',
  PromotionUpdateInputPlacementEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$PromotionUpdateInputStatusEnumEnumMap = {
  PromotionUpdateInputStatusEnum.draft: 'draft',
  PromotionUpdateInputStatusEnum.scheduled: 'scheduled',
  PromotionUpdateInputStatusEnum.published: 'published',
  PromotionUpdateInputStatusEnum.paused: 'paused',
  PromotionUpdateInputStatusEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$PromotionUpdateInputAudienceEnumEnumMap = {
  PromotionUpdateInputAudienceEnum.all: 'all',
  PromotionUpdateInputAudienceEnum.authenticated: 'authenticated',
  PromotionUpdateInputAudienceEnum.staff: 'staff',
  PromotionUpdateInputAudienceEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
