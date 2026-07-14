// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'upload_intent_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UploadIntentInput _$UploadIntentInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UploadIntentInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['kind', 'contentType']);
      final val = UploadIntentInput(
        kind: $checkedConvert(
          'kind',
          (v) => $enumDecode(
            _$UploadIntentInputKindEnumEnumMap,
            v,
            unknownValue: UploadIntentInputKindEnum.unknownDefaultOpenApi,
          ),
        ),
        contentType: $checkedConvert('contentType', (v) => v as String),
        usage: $checkedConvert(
          'usage',
          (v) => $enumDecodeNullable(
            _$MediaUsageEnumMap,
            v,
            unknownValue: MediaUsage.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$UploadIntentInputToJson(UploadIntentInput instance) =>
    <String, dynamic>{
      'kind': _$UploadIntentInputKindEnumEnumMap[instance.kind]!,
      'contentType': instance.contentType,
      'usage': ?_$MediaUsageEnumMap[instance.usage],
    };

const _$UploadIntentInputKindEnumEnumMap = {
  UploadIntentInputKindEnum.image: 'image',
  UploadIntentInputKindEnum.file: 'file',
  UploadIntentInputKindEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};

const _$MediaUsageEnumMap = {
  MediaUsage.profileAvatar: 'profile_avatar',
  MediaUsage.profileBanner: 'profile_banner',
  MediaUsage.forumThread: 'forum_thread',
  MediaUsage.forumComment: 'forum_comment',
  MediaUsage.unknownDefaultOpenApi: 'unknown_default_open_api',
};
