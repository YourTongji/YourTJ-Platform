//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'review_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ReviewInput {
  /// Returns a new [ReviewInput] instance.
  ReviewInput({required this.rating, this.comment, this.semester, this.score});

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

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ReviewInput &&
          other.rating == rating &&
          other.comment == comment &&
          other.semester == semester &&
          other.score == score;

  @override
  int get hashCode =>
      rating.hashCode + comment.hashCode + semester.hashCode + score.hashCode;

  factory ReviewInput.fromJson(Map<String, dynamic> json) =>
      _$ReviewInputFromJson(json);

  Map<String, dynamic> toJson() => _$ReviewInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
