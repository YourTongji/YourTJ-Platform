//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'create_review_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class CreateReviewInput {
  /// Returns a new [CreateReviewInput] instance.
  CreateReviewInput({
    required this.rating,

    this.comment,

    this.semester,

    this.score,

    required this.captchaToken,
  });

  // minimum: 0
  // maximum: 5
  @JsonKey(name: r'rating', required: true, includeIfNull: false)
  final int rating;

  @JsonKey(name: r'comment', required: false, includeIfNull: false)
  final String? comment;

  @JsonKey(name: r'semester', required: false, includeIfNull: false)
  final String? semester;

  @JsonKey(name: r'score', required: false, includeIfNull: false)
  final String? score;

  @JsonKey(name: r'captchaToken', required: true, includeIfNull: false)
  final String captchaToken;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is CreateReviewInput &&
          other.rating == rating &&
          other.comment == comment &&
          other.semester == semester &&
          other.score == score &&
          other.captchaToken == captchaToken;

  @override
  int get hashCode =>
      rating.hashCode +
      comment.hashCode +
      semester.hashCode +
      score.hashCode +
      captchaToken.hashCode;

  factory CreateReviewInput.fromJson(Map<String, dynamic> json) =>
      _$CreateReviewInputFromJson(json);

  Map<String, dynamic> toJson() => _$CreateReviewInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
