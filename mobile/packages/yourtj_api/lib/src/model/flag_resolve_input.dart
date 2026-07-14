//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'flag_resolve_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class FlagResolveInput {
  /// Returns a new [FlagResolveInput] instance.
  FlagResolveInput({required this.action, required this.note});

  @JsonKey(
    name: r'action',
    required: true,
    includeIfNull: false,
    unknownEnumValue: FlagResolveInputActionEnum.unknownDefaultOpenApi,
  )
  final FlagResolveInputActionEnum action;

  @JsonKey(name: r'note', required: true, includeIfNull: false)
  final String note;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is FlagResolveInput && other.action == action && other.note == note;

  @override
  int get hashCode => action.hashCode + note.hashCode;

  factory FlagResolveInput.fromJson(Map<String, dynamic> json) =>
      _$FlagResolveInputFromJson(json);

  Map<String, dynamic> toJson() => _$FlagResolveInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum FlagResolveInputActionEnum {
  @JsonValue(r'uphold')
  uphold(r'uphold'),
  @JsonValue(r'reject')
  reject(r'reject'),
  @JsonValue(r'ignore')
  ignore(r'ignore'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const FlagResolveInputActionEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
