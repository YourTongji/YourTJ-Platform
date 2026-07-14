// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_conversation_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmConversationInput _$DmConversationInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DmConversationInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['recipientHandle']);
      final val = DmConversationInput(
        recipientHandle: $checkedConvert('recipientHandle', (v) => v as String),
        requestMessage: $checkedConvert('requestMessage', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$DmConversationInputToJson(
  DmConversationInput instance,
) => <String, dynamic>{
  'recipientHandle': instance.recipientHandle,
  'requestMessage': ?instance.requestMessage,
};
