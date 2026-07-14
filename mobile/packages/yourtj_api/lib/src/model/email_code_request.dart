//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/email_code_purpose.dart';
import 'package:json_annotation/json_annotation.dart';

part 'email_code_request.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class EmailCodeRequest {
  /// Returns a new [EmailCodeRequest] instance.
  EmailCodeRequest({
    required this.email,

    required this.captchaToken,

    this.purpose,
  });

  @JsonKey(name: r'email', required: true, includeIfNull: false)
  final String email;

  @JsonKey(name: r'captchaToken', required: true, includeIfNull: false)
  final String captchaToken;

  /// Optional only for rolling compatibility. Appeal and recovery codes are consumed only by their purpose-bound routes.
  @JsonKey(
    name: r'purpose',
    required: false,
    includeIfNull: false,
    unknownEnumValue: EmailCodePurpose.unknownDefaultOpenApi,
  )
  final EmailCodePurpose? purpose;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is EmailCodeRequest &&
          other.email == email &&
          other.captchaToken == captchaToken &&
          other.purpose == purpose;

  @override
  int get hashCode => email.hashCode + captchaToken.hashCode + purpose.hashCode;

  factory EmailCodeRequest.fromJson(Map<String, dynamic> json) =>
      _$EmailCodeRequestFromJson(json);

  Map<String, dynamic> toJson() => _$EmailCodeRequestToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
