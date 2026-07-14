//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'deactivate_account_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DeactivateAccountInput {
  /// Returns a new [DeactivateAccountInput] instance.
  DeactivateAccountInput({required this.confirmation});

  @JsonKey(
    name: r'confirmation',
    required: true,
    includeIfNull: false,
    unknownEnumValue:
        DeactivateAccountInputConfirmationEnum.unknownDefaultOpenApi,
  )
  final DeactivateAccountInputConfirmationEnum confirmation;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DeactivateAccountInput && other.confirmation == confirmation;

  @override
  int get hashCode => confirmation.hashCode;

  factory DeactivateAccountInput.fromJson(Map<String, dynamic> json) =>
      _$DeactivateAccountInputFromJson(json);

  Map<String, dynamic> toJson() => _$DeactivateAccountInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum DeactivateAccountInputConfirmationEnum {
  @JsonValue(r'DEACTIVATE')
  DEACTIVATE(r'DEACTIVATE'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const DeactivateAccountInputConfirmationEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
