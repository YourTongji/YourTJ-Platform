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
  DmMessageInput({required this.body});

  @JsonKey(name: r'body', required: true, includeIfNull: false)
  final String body;

  @override
  bool operator ==(Object other) =>
      identical(this, other) || other is DmMessageInput && other.body == body;

  @override
  int get hashCode => body.hashCode;

  factory DmMessageInput.fromJson(Map<String, dynamic> json) =>
      _$DmMessageInputFromJson(json);

  Map<String, dynamic> toJson() => _$DmMessageInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
