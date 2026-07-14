//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'activity_weights.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ActivityWeights {
  /// Returns a new [ActivityWeights] instance.
  ActivityWeights({
    required this.thread,

    required this.comment,

    required this.like,

    required this.checkIn,
  });

  // minimum: 0
  // maximum: 1000
  @JsonKey(name: r'thread', required: true, includeIfNull: false)
  final int thread;

  // minimum: 0
  // maximum: 1000
  @JsonKey(name: r'comment', required: true, includeIfNull: false)
  final int comment;

  // minimum: 0
  // maximum: 1000
  @JsonKey(name: r'like', required: true, includeIfNull: false)
  final int like;

  // minimum: 0
  // maximum: 1000
  @JsonKey(name: r'checkIn', required: true, includeIfNull: false)
  final int checkIn;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ActivityWeights &&
          other.thread == thread &&
          other.comment == comment &&
          other.like == like &&
          other.checkIn == checkIn;

  @override
  int get hashCode =>
      thread.hashCode + comment.hashCode + like.hashCode + checkIn.hashCode;

  factory ActivityWeights.fromJson(Map<String, dynamic> json) =>
      _$ActivityWeightsFromJson(json);

  Map<String, dynamic> toJson() => _$ActivityWeightsToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
