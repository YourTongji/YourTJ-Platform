import 'package:flutter/material.dart';

@immutable
class YourTjPalette extends ThemeExtension<YourTjPalette> {
  const YourTjPalette({
    required this.background,
    required this.foreground,
    required this.card,
    required this.popover,
    required this.primary,
    required this.onPrimary,
    required this.secondary,
    required this.onSecondary,
    required this.muted,
    required this.onMuted,
    required this.accent,
    required this.onAccent,
    required this.destructive,
    required this.onDestructive,
    required this.border,
    required this.input,
    required this.ring,
    required this.chartColors,
  });

  static const YourTjPalette light = YourTjPalette(
    background: Color(0xFFF8FAF8),
    foreground: Color(0xFF191C1B),
    card: Color(0xFFF2F4F2),
    popover: Color(0xFFFFFFFF),
    primary: Color(0xFF009688),
    onPrimary: Color(0xFFFFFFFF),
    secondary: Color(0xFFF0FDFA),
    onSecondary: Color(0xFF00796B),
    muted: Color(0xFFECEEEC),
    onMuted: Color(0xFF596562),
    accent: Color(0xFFECEEEC),
    onAccent: Color(0xFF191C1B),
    destructive: Color(0xFFD4183D),
    onDestructive: Color(0xFFFFFFFF),
    border: Color(0xFFE1E3E1),
    input: Color(0xFFBCC9C6),
    ring: Color(0xFF009688),
    chartColors: <Color>[
      Color(0xFF009688),
      Color(0xFFC9A227),
      Color(0xFF4F87A0),
      Color(0xFF7CA86E),
      Color(0xFFB85C38),
    ],
  );

  static const YourTjPalette dark = YourTjPalette(
    background: Color(0xFF0C1E1B),
    foreground: Color(0xFFD8EDEA),
    card: Color(0xFF132922),
    popover: Color(0xFF132922),
    primary: Color(0xFF2ECFB2),
    onPrimary: Color(0xFF071510),
    secondary: Color(0xFF1A3832),
    onSecondary: Color(0xFFA8D9D0),
    muted: Color(0xFF1A3832),
    onMuted: Color(0xFF79AAA2),
    accent: Color(0xFF1E4039),
    onAccent: Color(0xFFA8D9D0),
    destructive: Color(0xFFF04060),
    onDestructive: Color(0xFFFFFFFF),
    border: Color(0x242ECFB2),
    input: Color(0x242ECFB2),
    ring: Color(0xFF2ECFB2),
    chartColors: <Color>[
      Color(0xFF009688),
      Color(0xFFC9A227),
      Color(0xFF4F87A0),
      Color(0xFF7CA86E),
      Color(0xFFB85C38),
    ],
  );

  final Color background;
  final Color foreground;
  final Color card;
  final Color popover;
  final Color primary;
  final Color onPrimary;
  final Color secondary;
  final Color onSecondary;
  final Color muted;
  final Color onMuted;
  final Color accent;
  final Color onAccent;
  final Color destructive;
  final Color onDestructive;
  final Color border;
  final Color input;
  final Color ring;
  final List<Color> chartColors;

  @override
  YourTjPalette copyWith({
    Color? background,
    Color? foreground,
    Color? card,
    Color? popover,
    Color? primary,
    Color? onPrimary,
    Color? secondary,
    Color? onSecondary,
    Color? muted,
    Color? onMuted,
    Color? accent,
    Color? onAccent,
    Color? destructive,
    Color? onDestructive,
    Color? border,
    Color? input,
    Color? ring,
    List<Color>? chartColors,
  }) {
    return YourTjPalette(
      background: background ?? this.background,
      foreground: foreground ?? this.foreground,
      card: card ?? this.card,
      popover: popover ?? this.popover,
      primary: primary ?? this.primary,
      onPrimary: onPrimary ?? this.onPrimary,
      secondary: secondary ?? this.secondary,
      onSecondary: onSecondary ?? this.onSecondary,
      muted: muted ?? this.muted,
      onMuted: onMuted ?? this.onMuted,
      accent: accent ?? this.accent,
      onAccent: onAccent ?? this.onAccent,
      destructive: destructive ?? this.destructive,
      onDestructive: onDestructive ?? this.onDestructive,
      border: border ?? this.border,
      input: input ?? this.input,
      ring: ring ?? this.ring,
      chartColors: chartColors ?? this.chartColors,
    );
  }

  @override
  YourTjPalette lerp(covariant YourTjPalette? other, double t) {
    if (other == null) {
      return this;
    }
    return YourTjPalette(
      background: _lerpColor(background, other.background, t),
      foreground: _lerpColor(foreground, other.foreground, t),
      card: _lerpColor(card, other.card, t),
      popover: _lerpColor(popover, other.popover, t),
      primary: _lerpColor(primary, other.primary, t),
      onPrimary: _lerpColor(onPrimary, other.onPrimary, t),
      secondary: _lerpColor(secondary, other.secondary, t),
      onSecondary: _lerpColor(onSecondary, other.onSecondary, t),
      muted: _lerpColor(muted, other.muted, t),
      onMuted: _lerpColor(onMuted, other.onMuted, t),
      accent: _lerpColor(accent, other.accent, t),
      onAccent: _lerpColor(onAccent, other.onAccent, t),
      destructive: _lerpColor(destructive, other.destructive, t),
      onDestructive: _lerpColor(onDestructive, other.onDestructive, t),
      border: _lerpColor(border, other.border, t),
      input: _lerpColor(input, other.input, t),
      ring: _lerpColor(ring, other.ring, t),
      chartColors: List<Color>.generate(
        chartColors.length,
        (int index) =>
            _lerpColor(chartColors[index], other.chartColors[index], t),
        growable: false,
      ),
    );
  }
}

@immutable
class YourTjMotion extends ThemeExtension<YourTjMotion> {
  const YourTjMotion({
    required this.fast,
    required this.normal,
    required this.slow,
  });

  static const standard = YourTjMotion(
    fast: Duration(milliseconds: 120),
    normal: Duration(milliseconds: 200),
    slow: Duration(milliseconds: 320),
  );

  final Duration fast;
  final Duration normal;
  final Duration slow;

  @override
  YourTjMotion copyWith({Duration? fast, Duration? normal, Duration? slow}) {
    return YourTjMotion(
      fast: fast ?? this.fast,
      normal: normal ?? this.normal,
      slow: slow ?? this.slow,
    );
  }

  @override
  YourTjMotion lerp(covariant YourTjMotion? other, double t) {
    if (other == null) {
      return this;
    }
    return YourTjMotion(
      fast: _lerpDuration(fast, other.fast, t),
      normal: _lerpDuration(normal, other.normal, t),
      slow: _lerpDuration(slow, other.slow, t),
    );
  }
}

abstract final class AppTheme {
  static final ThemeData light = _buildTheme(
    Brightness.light,
    YourTjPalette.light,
  );

  static final ThemeData dark = _buildTheme(
    Brightness.dark,
    YourTjPalette.dark,
  );

  static ThemeData _buildTheme(Brightness brightness, YourTjPalette palette) {
    final ColorScheme colorScheme =
        ColorScheme.fromSeed(
          seedColor: palette.primary,
          brightness: brightness,
        ).copyWith(
          primary: palette.primary,
          onPrimary: palette.onPrimary,
          primaryContainer: palette.secondary,
          onPrimaryContainer: palette.onSecondary,
          secondary: palette.onSecondary,
          onSecondary: palette.secondary,
          secondaryContainer: palette.muted,
          onSecondaryContainer: palette.foreground,
          error: palette.destructive,
          onError: palette.onDestructive,
          surface: palette.background,
          onSurface: palette.foreground,
          surfaceContainer: palette.card,
          surfaceContainerHigh: palette.muted,
          outline: palette.border,
          outlineVariant: palette.input,
        );
    final ThemeData base = ThemeData(
      useMaterial3: true,
      brightness: brightness,
      colorScheme: colorScheme,
      scaffoldBackgroundColor: palette.background,
      dividerColor: palette.border,
      splashFactory: InkSparkle.splashFactory,
      visualDensity: VisualDensity.standard,
      materialTapTargetSize: MaterialTapTargetSize.padded,
      extensions: <ThemeExtension<dynamic>>[palette, YourTjMotion.standard],
    );

    return base.copyWith(
      textTheme: base.textTheme.apply(
        bodyColor: palette.foreground,
        displayColor: palette.foreground,
      ),
      appBarTheme: AppBarTheme(
        elevation: 0,
        scrolledUnderElevation: 0,
        centerTitle: false,
        backgroundColor: palette.popover,
        foregroundColor: palette.foreground,
        surfaceTintColor: Colors.transparent,
        shape: Border(bottom: BorderSide(color: palette.border)),
      ),
      cardTheme: CardThemeData(
        elevation: 0,
        color: palette.card,
        surfaceTintColor: Colors.transparent,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(12),
          side: BorderSide(color: palette.border),
        ),
      ),
      navigationBarTheme: NavigationBarThemeData(
        height: 72,
        elevation: 0,
        backgroundColor: palette.popover,
        indicatorColor: palette.muted,
        labelTextStyle: WidgetStatePropertyAll<TextStyle>(
          base.textTheme.labelMedium!.copyWith(color: palette.foreground),
        ),
      ),
      navigationRailTheme: NavigationRailThemeData(
        elevation: 0,
        backgroundColor: palette.background,
        indicatorColor: palette.muted,
        selectedIconTheme: IconThemeData(color: palette.primary),
        selectedLabelTextStyle: base.textTheme.labelLarge?.copyWith(
          color: palette.primary,
          fontWeight: FontWeight.w600,
        ),
        unselectedIconTheme: IconThemeData(color: palette.onMuted),
        unselectedLabelTextStyle: base.textTheme.labelLarge?.copyWith(
          color: palette.onMuted,
        ),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: palette.card,
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: BorderSide(color: palette.input),
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: BorderSide(color: palette.input),
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(12),
          borderSide: BorderSide(color: palette.ring, width: 2),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          foregroundColor: palette.foreground,
          side: BorderSide(color: palette.input),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(999),
          ),
          minimumSize: const Size(44, 44),
        ),
      ),
      iconButtonTheme: IconButtonThemeData(
        style: IconButton.styleFrom(minimumSize: const Size(44, 44)),
      ),
    );
  }
}

Color _lerpColor(Color start, Color end, double t) {
  return Color.lerp(start, end, t) ?? start;
}

Duration _lerpDuration(Duration start, Duration end, double t) {
  final double milliseconds =
      start.inMilliseconds + (end.inMilliseconds - start.inMilliseconds) * t;
  return Duration(milliseconds: milliseconds.round());
}
