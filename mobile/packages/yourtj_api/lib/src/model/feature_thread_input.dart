//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'feature_thread_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class FeatureThreadInput {
  /// Returns a new [FeatureThreadInput] instance.
  FeatureThreadInput({required this.featured, required this.reason});

  @JsonKey(name: r'featured', required: true, includeIfNull: false)
  final bool featured;

  @JsonKey(name: r'reason', required: true, includeIfNull: false)
  final String reason;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is FeatureThreadInput &&
          other.featured == featured &&
          other.reason == reason;

  @override
  int get hashCode => featured.hashCode + reason.hashCode;

  factory FeatureThreadInput.fromJson(Map<String, dynamic> json) =>
      _$FeatureThreadInputFromJson(json);

  Map<String, dynamic> toJson() => _$FeatureThreadInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
