//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'account_lifecycle_mutation_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AccountLifecycleMutationInput {
  /// Returns a new [AccountLifecycleMutationInput] instance.
  AccountLifecycleMutationInput({required this.confirmation});

  @JsonKey(
    name: r'confirmation',
    required: true,
    includeIfNull: false,
    unknownEnumValue:
        AccountLifecycleMutationInputConfirmationEnum.unknownDefaultOpenApi,
  )
  final AccountLifecycleMutationInputConfirmationEnum confirmation;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AccountLifecycleMutationInput &&
          other.confirmation == confirmation;

  @override
  int get hashCode => confirmation.hashCode;

  factory AccountLifecycleMutationInput.fromJson(Map<String, dynamic> json) =>
      _$AccountLifecycleMutationInputFromJson(json);

  Map<String, dynamic> toJson() => _$AccountLifecycleMutationInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum AccountLifecycleMutationInputConfirmationEnum {
  @JsonValue(r'DEACTIVATE')
  DEACTIVATE(r'DEACTIVATE'),
  @JsonValue(r'DELETE')
  DELETE(r'DELETE'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const AccountLifecycleMutationInputConfirmationEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
