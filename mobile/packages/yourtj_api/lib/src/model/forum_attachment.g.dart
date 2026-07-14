// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'forum_attachment.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ForumAttachment _$ForumAttachmentFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ForumAttachment', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'assetId',
          'reference',
          'position',
          'alt',
          'url',
          'expiresAt',
          'width',
          'height',
        ],
      );
      final val = ForumAttachment(
        assetId: $checkedConvert('assetId', (v) => v as String),
        reference: $checkedConvert('reference', (v) => v as String),
        position: $checkedConvert('position', (v) => (v as num).toInt()),
        alt: $checkedConvert('alt', (v) => v as String),
        url: $checkedConvert('url', (v) => v as String),
        expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
        width: $checkedConvert('width', (v) => (v as num?)?.toInt()),
        height: $checkedConvert('height', (v) => (v as num?)?.toInt()),
      );
      return val;
    });

Map<String, dynamic> _$ForumAttachmentToJson(ForumAttachment instance) =>
    <String, dynamic>{
      'assetId': instance.assetId,
      'reference': instance.reference,
      'position': instance.position,
      'alt': instance.alt,
      'url': instance.url,
      'expiresAt': instance.expiresAt,
      'width': instance.width,
      'height': instance.height,
    };
