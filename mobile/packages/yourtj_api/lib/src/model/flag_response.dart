//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'flag_response.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class FlagResponse {
  /// Returns a new [FlagResponse] instance.
  FlagResponse({
    required this.ok,

    required this.autoHidden,

    required this.autoSilenced,
  });

  @JsonKey(name: r'ok', required: true, includeIfNull: false)
  final bool ok;

  @JsonKey(name: r'autoHidden', required: true, includeIfNull: false)
  final bool autoHidden;

  @JsonKey(name: r'autoSilenced', required: true, includeIfNull: false)
  final bool autoSilenced;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is FlagResponse &&
          other.ok == ok &&
          other.autoHidden == autoHidden &&
          other.autoSilenced == autoSilenced;

  @override
  int get hashCode => ok.hashCode + autoHidden.hashCode + autoSilenced.hashCode;

  factory FlagResponse.fromJson(Map<String, dynamic> json) =>
      _$FlagResponseFromJson(json);

  Map<String, dynamic> toJson() => _$FlagResponseToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
