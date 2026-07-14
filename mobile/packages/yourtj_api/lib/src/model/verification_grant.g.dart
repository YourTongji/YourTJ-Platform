// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'verification_grant.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

VerificationGrant _$VerificationGrantFromJson(Map<String, dynamic> json) =>
    $checkedCreate('VerificationGrant', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'accountId',
          'verificationTypeId',
          'slug',
          'category',
          'label',
          'icon',
          'badgeVariant',
          'displayOnProfile',
          'status',
          'issuedBy',
          'issuedAt',
          'issueReason',
          'hasEvidence',
        ],
      );
      final val = VerificationGrant(
        id: $checkedConvert('id', (v) => v as String),
        accountId: $checkedConvert('accountId', (v) => v as String),
        verificationTypeId: $checkedConvert(
          'verificationTypeId',
          (v) => v as String,
        ),
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
        displayOnProfile: $checkedConvert('displayOnProfile', (v) => v as bool),
        status: $checkedConvert(
          'status',
          (v) => $enumDecode(
            _$VerificationGrantStatusEnumEnumMap,
            v,
            unknownValue: VerificationGrantStatusEnum.unknownDefaultOpenApi,
          ),
        ),
        issuedBy: $checkedConvert('issuedBy', (v) => v as String?),
        issuedAt: $checkedConvert('issuedAt', (v) => (v as num).toInt()),
        expiresAt: $checkedConvert('expiresAt', (v) => (v as num?)?.toInt()),
        issueReason: $checkedConvert('issueReason', (v) => v as String),
        hasEvidence: $checkedConvert('hasEvidence', (v) => v as bool),
        revokedBy: $checkedConvert('revokedBy', (v) => v as String?),
        revokedAt: $checkedConvert('revokedAt', (v) => (v as num?)?.toInt()),
        revokeReason: $checkedConvert('revokeReason', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$VerificationGrantToJson(VerificationGrant instance) =>
    <String, dynamic>{
      'id': instance.id,
      'accountId': instance.accountId,
      'verificationTypeId': instance.verificationTypeId,
      'slug': instance.slug,
      'category': _$VerificationCategoryEnumMap[instance.category]!,
      'label': instance.label,
      'icon': _$VerificationIconEnumMap[instance.icon]!,
      'badgeVariant': _$VerificationBadgeVariantEnumMap[instance.badgeVariant]!,
      'displayOnProfile': instance.displayOnProfile,
      'status': _$VerificationGrantStatusEnumEnumMap[instance.status]!,
      'issuedBy': instance.issuedBy,
      'issuedAt': instance.issuedAt,
      'expiresAt': ?instance.expiresAt,
      'issueReason': instance.issueReason,
      'hasEvidence': instance.hasEvidence,
      'revokedBy': ?instance.revokedBy,
      'revokedAt': ?instance.revokedAt,
      'revokeReason': ?instance.revokeReason,
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

const _$VerificationGrantStatusEnumEnumMap = {
  VerificationGrantStatusEnum.active: 'active',
  VerificationGrantStatusEnum.expired: 'expired',
  VerificationGrantStatusEnum.revoked: 'revoked',
  VerificationGrantStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
