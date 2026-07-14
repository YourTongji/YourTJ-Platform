//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'thread_pin_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ThreadPinInput {
  /// Returns a new [ThreadPinInput] instance.
  ThreadPinInput({this.globally});

  @JsonKey(name: r'globally', required: false, includeIfNull: false)
  final bool? globally;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ThreadPinInput && other.globally == globally;

  @override
  int get hashCode => globally.hashCode;

  factory ThreadPinInput.fromJson(Map<String, dynamic> json) =>
      _$ThreadPinInputFromJson(json);

  Map<String, dynamic> toJson() => _$ThreadPinInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
