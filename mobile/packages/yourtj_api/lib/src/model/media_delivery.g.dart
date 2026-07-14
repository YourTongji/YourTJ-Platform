// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'media_delivery.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MediaDelivery _$MediaDeliveryFromJson(Map<String, dynamic> json) =>
    $checkedCreate('MediaDelivery', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'assetId',
          'variant',
          'url',
          'expiresAt',
          'mime',
          'width',
          'height',
        ],
      );
      final val = MediaDelivery(
        assetId: $checkedConvert('assetId', (v) => v as String),
        variant: $checkedConvert(
          'variant',
          (v) => $enumDecode(
            _$MediaDeliveryVariantEnumMap,
            v,
            unknownValue: MediaDeliveryVariant.unknownDefaultOpenApi,
          ),
        ),
        url: $checkedConvert('url', (v) => v as String),
        expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
        mime: $checkedConvert(
          'mime',
          (v) => $enumDecode(
            _$MediaDeliveryMimeEnumEnumMap,
            v,
            unknownValue: MediaDeliveryMimeEnum.unknownDefaultOpenApi,
          ),
        ),
        width: $checkedConvert('width', (v) => (v as num).toInt()),
        height: $checkedConvert('height', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$MediaDeliveryToJson(MediaDelivery instance) =>
    <String, dynamic>{
      'assetId': instance.assetId,
      'variant': _$MediaDeliveryVariantEnumMap[instance.variant]!,
      'url': instance.url,
      'expiresAt': instance.expiresAt,
      'mime': _$MediaDeliveryMimeEnumEnumMap[instance.mime]!,
      'width': instance.width,
      'height': instance.height,
    };

const _$MediaDeliveryVariantEnumMap = {
  MediaDeliveryVariant.thumb256: 'thumb_256',
  MediaDeliveryVariant.display1280: 'display_1280',
  MediaDeliveryVariant.full2048: 'full_2048',
  MediaDeliveryVariant.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$MediaDeliveryMimeEnumEnumMap = {
  MediaDeliveryMimeEnum.imageSlashWebp: 'image/webp',
  MediaDeliveryMimeEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
