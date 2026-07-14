//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'dm_conversation_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DmConversationInput {
  /// Returns a new [DmConversationInput] instance.
  DmConversationInput({required this.recipientHandle, this.requestMessage});

  @JsonKey(name: r'recipientHandle', required: true, includeIfNull: false)
  final String recipientHandle;

  /// Required when the recipient does not already accept direct delivery from the sender; it is the only message allowed before acceptance.
  @JsonKey(name: r'requestMessage', required: false, includeIfNull: false)
  final String? requestMessage;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DmConversationInput &&
          other.recipientHandle == recipientHandle &&
          other.requestMessage == requestMessage;

  @override
  int get hashCode => recipientHandle.hashCode + requestMessage.hashCode;

  factory DmConversationInput.fromJson(Map<String, dynamic> json) =>
      _$DmConversationInputFromJson(json);

  Map<String, dynamic> toJson() => _$DmConversationInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
