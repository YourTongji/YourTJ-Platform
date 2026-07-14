//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'poll_option.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PollOption {
  /// Returns a new [PollOption] instance.
  PollOption({
    required this.id,

    required this.label,

    required this.voteCount,

    required this.position,
  });

  @JsonKey(name: r'id', required: true, includeIfNull: false)
  final String id;

  @JsonKey(name: r'label', required: true, includeIfNull: false)
  final String label;

  // minimum: 0
  @JsonKey(name: r'voteCount', required: true, includeIfNull: false)
  final int voteCount;

  // minimum: 0
  @JsonKey(name: r'position', required: true, includeIfNull: false)
  final int position;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PollOption &&
          other.id == id &&
          other.label == label &&
          other.voteCount == voteCount &&
          other.position == position;

  @override
  int get hashCode =>
      id.hashCode + label.hashCode + voteCount.hashCode + position.hashCode;

  factory PollOption.fromJson(Map<String, dynamic> json) =>
      _$PollOptionFromJson(json);

  Map<String, dynamic> toJson() => _$PollOptionToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
