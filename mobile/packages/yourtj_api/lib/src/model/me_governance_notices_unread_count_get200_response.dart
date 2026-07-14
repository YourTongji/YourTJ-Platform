//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'me_governance_notices_unread_count_get200_response.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class MeGovernanceNoticesUnreadCountGet200Response {
  /// Returns a new [MeGovernanceNoticesUnreadCountGet200Response] instance.
  MeGovernanceNoticesUnreadCountGet200Response({required this.count});

  // minimum: 0
  @JsonKey(name: r'count', required: true, includeIfNull: false)
  final int count;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is MeGovernanceNoticesUnreadCountGet200Response &&
          other.count == count;

  @override
  int get hashCode => count.hashCode;

  factory MeGovernanceNoticesUnreadCountGet200Response.fromJson(
    Map<String, dynamic> json,
  ) => _$MeGovernanceNoticesUnreadCountGet200ResponseFromJson(json);

  Map<String, dynamic> toJson() =>
      _$MeGovernanceNoticesUnreadCountGet200ResponseToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
