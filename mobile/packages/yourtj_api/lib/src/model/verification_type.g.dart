// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'verification_type.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

VerificationType _$VerificationTypeFromJson(Map<String, dynamic> json) =>
    $checkedCreate('VerificationType', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'slug',
          'category',
          'label',
          'icon',
          'badgeVariant',
          'allowsPublicDisplay',
          'createdAt',
        ],
      );
      final val = VerificationType(
        id: $checkedConvert('id', (v) => v as String),
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
        allowsPublicDisplay: $checkedConvert(
          'allowsPublicDisplay',
          (v) => v as bool,
        ),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$VerificationTypeToJson(VerificationType instance) =>
    <String, dynamic>{
      'id': instance.id,
      'slug': instance.slug,
      'category': _$VerificationCategoryEnumMap[instance.category]!,
      'label': instance.label,
      'description': ?instance.description,
      'icon': _$VerificationIconEnumMap[instance.icon]!,
      'badgeVariant': _$VerificationBadgeVariantEnumMap[instance.badgeVariant]!,
      'allowsPublicDisplay': instance.allowsPublicDisplay,
      'createdAt': instance.createdAt,
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
