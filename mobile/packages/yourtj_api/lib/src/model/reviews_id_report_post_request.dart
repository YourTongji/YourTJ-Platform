//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'reviews_id_report_post_request.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ReviewsIdReportPostRequest {
  /// Returns a new [ReviewsIdReportPostRequest] instance.
  ReviewsIdReportPostRequest({
    required this.reason,

    required this.captchaToken,
  });

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @JsonKey(name: r'captchaToken', required: true, includeIfNull: false)
  final String captchaToken;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ReviewsIdReportPostRequest &&
          other.reason == reason &&
          other.captchaToken == captchaToken;

  @override
  int get hashCode => reason.hashCode + captchaToken.hashCode;

  factory ReviewsIdReportPostRequest.fromJson(Map<String, dynamic> json) =>
      _$ReviewsIdReportPostRequestFromJson(json);

  Map<String, dynamic> toJson() => _$ReviewsIdReportPostRequestToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
