// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_conversation.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmConversation _$DmConversationFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('DmConversation', json, ($checkedConvert) {
  $checkKeys(
    json,
    requiredKeys: const [
      'id',
      'participantId',
      'participantHandle',
      'unreadCount',
      'isArchived',
      'isMuted',
      'isDeleted',
      'requestStatus',
      'canSend',
      'createdAt',
    ],
  );
  final val = DmConversation(
    id: $checkedConvert('id', (v) => v as String),
    participantId: $checkedConvert('participantId', (v) => v as String),
    participantHandle: $checkedConvert('participantHandle', (v) => v as String),
    participantDisplayName: $checkedConvert(
      'participantDisplayName',
      (v) => v as String?,
    ),
    participantAvatarUrl: $checkedConvert(
      'participantAvatarUrl',
      (v) => v as String?,
    ),
    lastMessageExcerpt: $checkedConvert(
      'lastMessageExcerpt',
      (v) => v as String?,
    ),
    lastMessageAt: $checkedConvert(
      'lastMessageAt',
      (v) => (v as num?)?.toInt(),
    ),
    unreadCount: $checkedConvert('unreadCount', (v) => (v as num).toInt()),
    isArchived: $checkedConvert('isArchived', (v) => v as bool),
    isMuted: $checkedConvert('isMuted', (v) => v as bool),
    isDeleted: $checkedConvert('isDeleted', (v) => v as bool),
    requestStatus: $checkedConvert(
      'requestStatus',
      (v) => $enumDecode(
        _$DmConversationRequestStatusEnumEnumMap,
        v,
        unknownValue: DmConversationRequestStatusEnum.unknownDefaultOpenApi,
      ),
    ),
    requestDirection: $checkedConvert(
      'requestDirection',
      (v) => $enumDecodeNullable(
        _$DmConversationRequestDirectionEnumEnumMap,
        v,
        unknownValue: DmConversationRequestDirectionEnum.unknownDefaultOpenApi,
      ),
    ),
    canSend: $checkedConvert('canSend', (v) => v as bool),
    createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$DmConversationToJson(
  DmConversation instance,
) => <String, dynamic>{
  'id': instance.id,
  'participantId': instance.participantId,
  'participantHandle': instance.participantHandle,
  'participantDisplayName': ?instance.participantDisplayName,
  'participantAvatarUrl': ?instance.participantAvatarUrl,
  'lastMessageExcerpt': ?instance.lastMessageExcerpt,
  'lastMessageAt': ?instance.lastMessageAt,
  'unreadCount': instance.unreadCount,
  'isArchived': instance.isArchived,
  'isMuted': instance.isMuted,
  'isDeleted': instance.isDeleted,
  'requestStatus':
      _$DmConversationRequestStatusEnumEnumMap[instance.requestStatus]!,
  'requestDirection':
      ?_$DmConversationRequestDirectionEnumEnumMap[instance.requestDirection],
  'canSend': instance.canSend,
  'createdAt': instance.createdAt,
};

const _$DmConversationRequestStatusEnumEnumMap = {
  DmConversationRequestStatusEnum.accepted: 'accepted',
  DmConversationRequestStatusEnum.pending: 'pending',
  DmConversationRequestStatusEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};

const _$DmConversationRequestDirectionEnumEnumMap = {
  DmConversationRequestDirectionEnum.incoming: 'incoming',
  DmConversationRequestDirectionEnum.outgoing: 'outgoing',
  DmConversationRequestDirectionEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
