//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'error_error.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ErrorError {
  /// Returns a new [ErrorError] instance.
  ErrorError({required this.code, required this.message, this.details});

  @JsonKey(name: r'code', required: true, includeIfNull: false)
  final String code;

  @JsonKey(name: r'message', required: true, includeIfNull: false)
  final String message;

  @JsonKey(name: r'details', required: false, includeIfNull: false)
  final Object? details;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ErrorError &&
          other.code == code &&
          other.message == message &&
          other.details == details;

  @override
  int get hashCode => code.hashCode + message.hashCode + details.hashCode;

  factory ErrorError.fromJson(Map<String, dynamic> json) =>
      _$ErrorErrorFromJson(json);

  Map<String, dynamic> toJson() => _$ErrorErrorToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
