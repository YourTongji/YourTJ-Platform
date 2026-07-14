//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

enum SearchResultScope {
  @JsonValue(r'course')
  course(r'course'),
  @JsonValue(r'review')
  review(r'review'),
  @JsonValue(r'thread')
  thread(r'thread'),
  @JsonValue(r'user')
  user(r'user'),
  @JsonValue(r'board')
  board(r'board'),
  @JsonValue(r'tag')
  tag(r'tag'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const SearchResultScope(this.value);

  final String value;

  @override
  String toString() => value;
}
