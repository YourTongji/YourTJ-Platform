//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'featured_thread_response.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class FeaturedThreadResponse {
  /// Returns a new [FeaturedThreadResponse] instance.
  FeaturedThreadResponse({this.id, this.featuredAt});

  @JsonKey(name: r'id', required: false, includeIfNull: false)
  final String? id;

  @JsonKey(name: r'featuredAt', required: false, includeIfNull: false)
  final int? featuredAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is FeaturedThreadResponse &&
          other.id == id &&
          other.featuredAt == featuredAt;

  @override
  int get hashCode => id.hashCode + featuredAt.hashCode;

  factory FeaturedThreadResponse.fromJson(Map<String, dynamic> json) =>
      _$FeaturedThreadResponseFromJson(json);

  Map<String, dynamic> toJson() => _$FeaturedThreadResponseToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
