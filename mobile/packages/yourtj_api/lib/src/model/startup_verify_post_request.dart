//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'startup_verify_post_request.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class StartupVerifyPostRequest {
  /// Returns a new [StartupVerifyPostRequest] instance.
  StartupVerifyPostRequest({required this.token});

  @JsonKey(name: r'token', required: true, includeIfNull: false)
  final String token;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is StartupVerifyPostRequest && other.token == token;

  @override
  int get hashCode => token.hashCode;

  factory StartupVerifyPostRequest.fromJson(Map<String, dynamic> json) =>
      _$StartupVerifyPostRequestFromJson(json);

  Map<String, dynamic> toJson() => _$StartupVerifyPostRequestToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
