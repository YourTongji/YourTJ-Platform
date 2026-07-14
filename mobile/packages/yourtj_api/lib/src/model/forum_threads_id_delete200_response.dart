//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'forum_threads_id_delete200_response.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ForumThreadsIdDelete200Response {
  /// Returns a new [ForumThreadsIdDelete200Response] instance.
  ForumThreadsIdDelete200Response({required this.ok});

  @JsonKey(name: r'ok', required: true, includeIfNull: false)
  final bool ok;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ForumThreadsIdDelete200Response && other.ok == ok;

  @override
  int get hashCode => ok.hashCode;

  factory ForumThreadsIdDelete200Response.fromJson(Map<String, dynamic> json) =>
      _$ForumThreadsIdDelete200ResponseFromJson(json);

  Map<String, dynamic> toJson() =>
      _$ForumThreadsIdDelete200ResponseToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
