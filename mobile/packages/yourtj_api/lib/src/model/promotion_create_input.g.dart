// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'promotion_create_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PromotionCreateInput _$PromotionCreateInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('PromotionCreateInput', json, ($checkedConvert) {
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
    ],
  );
  final val = PromotionCreateInput(
    placement: $checkedConvert(
      'placement',
      (v) => $enumDecode(
        _$PromotionCreateInputPlacementEnumEnumMap,
        v,
        unknownValue: PromotionCreateInputPlacementEnum.unknownDefaultOpenApi,
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
        _$PromotionCreateInputStatusEnumEnumMap,
        v,
        unknownValue: PromotionCreateInputStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    priority: $checkedConvert('priority', (v) => (v as num).toInt()),
    audience: $checkedConvert(
      'audience',
      (v) => $enumDecode(
        _$PromotionCreateInputAudienceEnumEnumMap,
        v,
        unknownValue: PromotionCreateInputAudienceEnum.unknownDefaultOpenApi,
      ),
    ),
    startsAt: $checkedConvert('startsAt', (v) => (v as num?)?.toInt()),
    endsAt: $checkedConvert('endsAt', (v) => (v as num?)?.toInt()),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$PromotionCreateInputToJson(
  PromotionCreateInput instance,
) => <String, dynamic>{
  'placement': _$PromotionCreateInputPlacementEnumEnumMap[instance.placement]!,
  'title': instance.title,
  'body': ?instance.body,
  'ctaLabel': ?instance.ctaLabel,
  'targetUrl': instance.targetUrl,
  'assetId': ?instance.assetId,
  'status': _$PromotionCreateInputStatusEnumEnumMap[instance.status]!,
  'priority': instance.priority,
  'audience': _$PromotionCreateInputAudienceEnumEnumMap[instance.audience]!,
  'startsAt': ?instance.startsAt,
  'endsAt': ?instance.endsAt,
  'reason': instance.reason,
};

const _$PromotionCreateInputPlacementEnumEnumMap = {
  PromotionCreateInputPlacementEnum.homeLeftPrimary: 'home-left-primary',
  PromotionCreateInputPlacementEnum.homeLeftSecondary: 'home-left-secondary',
  PromotionCreateInputPlacementEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$PromotionCreateInputStatusEnumEnumMap = {
  PromotionCreateInputStatusEnum.draft: 'draft',
  PromotionCreateInputStatusEnum.scheduled: 'scheduled',
  PromotionCreateInputStatusEnum.published: 'published',
  PromotionCreateInputStatusEnum.paused: 'paused',
  PromotionCreateInputStatusEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$PromotionCreateInputAudienceEnumEnumMap = {
  PromotionCreateInputAudienceEnum.all: 'all',
  PromotionCreateInputAudienceEnum.authenticated: 'authenticated',
  PromotionCreateInputAudienceEnum.staff: 'staff',
  PromotionCreateInputAudienceEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
