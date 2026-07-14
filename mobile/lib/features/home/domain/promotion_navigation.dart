abstract final class PromotionNavigation {
  static String? internalLocation(String value) {
    final Uri? uri = Uri.tryParse(value);
    if (uri == null ||
        value.contains(r'\') ||
        uri.hasScheme ||
        uri.hasAuthority ||
        uri.hasFragment ||
        !value.startsWith('/') ||
        value.startsWith('//')) {
      return null;
    }
    const Set<String> roots = <String>{
      '/',
      '/account',
      '/announcements',
      '/appeals',
      '/bookmarks',
      '/courses',
      '/forum',
      '/messages',
      '/notifications',
      '/profile',
      '/schedule',
      '/search',
      '/settings',
      '/wallet',
    };
    final String root = uri.path == '/' || uri.pathSegments.isEmpty
        ? '/'
        : '/${uri.pathSegments.first}';
    return roots.contains(root) ? uri.toString() : null;
  }
}
