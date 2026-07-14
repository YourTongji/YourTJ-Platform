// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'my_upload.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

MyUpload _$MyUploadFromJson(Map<String, dynamic> json) => $checkedCreate(
  'MyUpload',
  json,
  ($checkedConvert) {
    $checkKeys(
      json,
      requiredKeys: const [
        'id',
        'kind',
        'usage',
        'bytes',
        'mime',
        'status',
        'deliveryState',
        'imageWidth',
        'imageHeight',
        'createdAt',
      ],
    );
    final val = MyUpload(
      id: $checkedConvert('id', (v) => v as String),
      kind: $checkedConvert(
        'kind',
        (v) => $enumDecode(
          _$MyUploadKindEnumEnumMap,
          v,
          unknownValue: MyUploadKindEnum.unknownDefaultOpenApi,
        ),
      ),
      usage: $checkedConvert(
        'usage',
        (v) => $enumDecodeNullable(
          _$MediaUsageEnumMap,
          v,
          unknownValue: MediaUsage.unknownDefaultOpenApi,
        ),
      ),
      bytes: $checkedConvert('bytes', (v) => (v as num).toInt()),
      mime: $checkedConvert('mime', (v) => v as String),
      status: $checkedConvert(
        'status',
        (v) => $enumDecode(
          _$MyUploadStatusEnumEnumMap,
          v,
          unknownValue: MyUploadStatusEnum.unknownDefaultOpenApi,
        ),
      ),
      deliveryState: $checkedConvert(
        'deliveryState',
        (v) => $enumDecode(
          _$MediaDeliveryStateEnumMap,
          v,
          unknownValue: MediaDeliveryState.unknownDefaultOpenApi,
        ),
      ),
      imageWidth: $checkedConvert('imageWidth', (v) => (v as num?)?.toInt()),
      imageHeight: $checkedConvert('imageHeight', (v) => (v as num?)?.toInt()),
      createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
    );
    return val;
  },
);

Map<String, dynamic> _$MyUploadToJson(MyUpload instance) => <String, dynamic>{
  'id': instance.id,
  'kind': _$MyUploadKindEnumEnumMap[instance.kind]!,
  'usage': _$MediaUsageEnumMap[instance.usage],
  'bytes': instance.bytes,
  'mime': instance.mime,
  'status': _$MyUploadStatusEnumEnumMap[instance.status]!,
  'deliveryState': _$MediaDeliveryStateEnumMap[instance.deliveryState]!,
  'imageWidth': instance.imageWidth,
  'imageHeight': instance.imageHeight,
  'createdAt': instance.createdAt,
};

const _$MyUploadKindEnumEnumMap = {
  MyUploadKindEnum.image: 'image',
  MyUploadKindEnum.file: 'file',
  MyUploadKindEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$MediaUsageEnumMap = {
  MediaUsage.profileAvatar: 'profile_avatar',
  MediaUsage.profileBanner: 'profile_banner',
  MediaUsage.forumThread: 'forum_thread',
  MediaUsage.forumComment: 'forum_comment',
  MediaUsage.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$MyUploadStatusEnumEnumMap = {
  MyUploadStatusEnum.pending: 'pending',
  MyUploadStatusEnum.clean: 'clean',
  MyUploadStatusEnum.quarantined: 'quarantined',
  MyUploadStatusEnum.blocked: 'blocked',
  MyUploadStatusEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$MediaDeliveryStateEnumMap = {
  MediaDeliveryState.unpublished: 'unpublished',
  MediaDeliveryState.processing: 'processing',
  MediaDeliveryState.published: 'published',
  MediaDeliveryState.failed: 'failed',
  MediaDeliveryState.blocked: 'blocked',
  MediaDeliveryState.unknownDefaultOpenApi: 'unknown_default_open_api',
};
