//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'dm_message_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DmMessageInput {
  /// Returns a new [DmMessageInput] instance.
  DmMessageInput({required this.body, this.clientMessageId});

  @JsonKey(name: r'body', required: true, includeIfNull: false)
  final String body;

  /// Optional client-generated idempotency identity; retries with the same value must keep the same conversation and body.
  @JsonKey(name: r'clientMessageId', required: false, includeIfNull: false)
  final String? clientMessageId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DmMessageInput &&
          other.body == body &&
          other.clientMessageId == clientMessageId;

  @override
  int get hashCode => body.hashCode + clientMessageId.hashCode;

  factory DmMessageInput.fromJson(Map<String, dynamic> json) =>
      _$DmMessageInputFromJson(json);

  Map<String, dynamic> toJson() => _$DmMessageInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
