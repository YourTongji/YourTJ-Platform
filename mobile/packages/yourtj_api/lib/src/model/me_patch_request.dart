//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'me_patch_request.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MePatchRequest {
  /// Returns a new [MePatchRequest] instance.
  MePatchRequest({this.handle});

  @JsonKey(name: r'handle', required: false, includeIfNull: false)
  final String? handle;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MePatchRequest && other.handle == handle;

  @override
  int get hashCode => handle.hashCode;

  factory MePatchRequest.fromJson(Map<String, dynamic> json) =>
      _$MePatchRequestFromJson(json);

  Map<String, dynamic> toJson() => _$MePatchRequestToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
