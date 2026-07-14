//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'delete_account_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class DeleteAccountInput {
  /// Returns a new [DeleteAccountInput] instance.
  DeleteAccountInput({required this.confirmation});

  @JsonKey(
    name: r'confirmation',
    required: true,
    includeIfNull: false,
    unknownEnumValue: DeleteAccountInputConfirmationEnum.unknownDefaultOpenApi,
  )
  final DeleteAccountInputConfirmationEnum confirmation;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DeleteAccountInput && other.confirmation == confirmation;

  @override
  int get hashCode => confirmation.hashCode;

  factory DeleteAccountInput.fromJson(Map<String, dynamic> json) =>
      _$DeleteAccountInputFromJson(json);

  Map<String, dynamic> toJson() => _$DeleteAccountInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum DeleteAccountInputConfirmationEnum {
  @JsonValue(r'DELETE')
  DELETE(r'DELETE'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const DeleteAccountInputConfirmationEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
