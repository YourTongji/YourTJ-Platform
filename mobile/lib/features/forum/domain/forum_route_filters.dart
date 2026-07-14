abstract final class ForumRouteFilters {
  static final RegExp _boardId = RegExp(r'^[A-Za-z0-9_-]{1,128}$');
  static final RegExp _tagSlug = RegExp(r'^[a-z0-9]+(?:-[a-z0-9]+)*$');

  static String? boardId(String? value) =>
      value != null && _boardId.hasMatch(value) ? value : null;

  static String? tagSlug(String? value) =>
      value != null && value.length <= 64 && _tagSlug.hasMatch(value)
      ? value
      : null;
}
