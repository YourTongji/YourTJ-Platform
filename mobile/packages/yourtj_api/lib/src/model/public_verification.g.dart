// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'public_verification.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PublicVerification _$PublicVerificationFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PublicVerification', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'slug',
          'category',
          'label',
          'icon',
          'badgeVariant',
          'issuedAt',
        ],
      );
      final val = PublicVerification(
        slug: $checkedConvert('slug', (v) => v as String),
        category: $checkedConvert(
          'category',
          (v) => $enumDecode(
            _$VerificationCategoryEnumMap,
            v,
            unknownValue: VerificationCategory.unknownDefaultOpenApi,
          ),
        ),
        label: $checkedConvert('label', (v) => v as String),
        description: $checkedConvert('description', (v) => v as String?),
        icon: $checkedConvert(
          'icon',
          (v) => $enumDecode(
            _$VerificationIconEnumMap,
            v,
            unknownValue: VerificationIcon.unknownDefaultOpenApi,
          ),
        ),
        badgeVariant: $checkedConvert(
          'badgeVariant',
          (v) => $enumDecode(
            _$VerificationBadgeVariantEnumMap,
            v,
            unknownValue: VerificationBadgeVariant.unknownDefaultOpenApi,
          ),
        ),
        issuedAt: $checkedConvert('issuedAt', (v) => (v as num).toInt()),
        expiresAt: $checkedConvert('expiresAt', (v) => (v as num?)?.toInt()),
      );
      return val;
    });

Map<String, dynamic> _$PublicVerificationToJson(PublicVerification instance) =>
    <String, dynamic>{
      'slug': instance.slug,
      'category': _$VerificationCategoryEnumMap[instance.category]!,
      'label': instance.label,
      'description': ?instance.description,
      'icon': _$VerificationIconEnumMap[instance.icon]!,
      'badgeVariant': _$VerificationBadgeVariantEnumMap[instance.badgeVariant]!,
      'issuedAt': instance.issuedAt,
      'expiresAt': ?instance.expiresAt,
    };

const _$VerificationCategoryEnumMap = {
  VerificationCategory.identity: 'identity',
  VerificationCategory.special: 'special',
  VerificationCategory.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$VerificationIconEnumMap = {
  VerificationIcon.badgeCheck: 'badge-check',
  VerificationIcon.building2: 'building-2',
  VerificationIcon.shieldCheck: 'shield-check',
  VerificationIcon.sparkles: 'sparkles',
  VerificationIcon.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$VerificationBadgeVariantEnumMap = {
  VerificationBadgeVariant.default_: 'default',
  VerificationBadgeVariant.secondary: 'secondary',
  VerificationBadgeVariant.outline: 'outline',
  VerificationBadgeVariant.unknownDefaultOpenApi: 'unknown_default_open_api',
};
