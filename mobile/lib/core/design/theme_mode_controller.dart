import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

abstract interface class ThemeModePreferences {
  Future<String?> read();

  Future<void> write(String value);
}

enum ThemeModePersistenceFailure { load, save }

class ThemeModeController extends ChangeNotifier {
  factory ThemeModeController({
    ThemeModePreferences? preferences,
    ThemeMode initialMode = ThemeMode.system,
  }) => ThemeModeController._(preferences, initialMode);

  ThemeModeController._(this._preferences, this._mode);

  ThemeModePreferences? _preferences;
  ThemeMode _mode;
  Future<void> _writeQueue = Future<void>.value();
  ThemeModePersistenceFailure? _persistenceFailure;
  int _operationRevision = 0;
  bool _isPersisting = false;
  bool _isDisposed = false;

  ThemeMode get mode => _mode;
  ThemeModePersistenceFailure? get persistenceFailure => _persistenceFailure;
  bool get isPersisting => _isPersisting;

  Future<void> restore() async {
    final int revision = ++_operationRevision;
    _isPersisting = true;
    _notifyStateChanged();
    try {
      final String? value = await _resolvedPreferences.read();
      if (_isDisposed || revision != _operationRevision) {
        return;
      }
      final ThemeMode restored = switch (value) {
        'light' => ThemeMode.light,
        'dark' => ThemeMode.dark,
        _ => ThemeMode.system,
      };
      _mode = restored;
      _persistenceFailure = null;
    } on Object {
      if (!_isDisposed && revision == _operationRevision) {
        _persistenceFailure = ThemeModePersistenceFailure.load;
      }
    } finally {
      if (!_isDisposed && revision == _operationRevision) {
        _isPersisting = false;
        _notifyStateChanged();
      }
    }
  }

  Future<void> setMode(ThemeMode mode) async {
    final int revision = ++_operationRevision;
    _mode = mode;
    _isPersisting = true;
    _notifyStateChanged();
    final Future<void> write = _writeQueue.then((_) async {
      try {
        await _resolvedPreferences.write(mode.name);
        if (!_isDisposed && revision == _operationRevision) {
          _persistenceFailure = null;
        }
      } on Object {
        if (!_isDisposed && revision == _operationRevision) {
          _persistenceFailure = ThemeModePersistenceFailure.save;
        }
      } finally {
        if (!_isDisposed && revision == _operationRevision) {
          _isPersisting = false;
          _notifyStateChanged();
        }
      }
    });
    _writeQueue = write;
    await write;
  }

  Future<void> retryPersistence() {
    if (_isPersisting) {
      return Future<void>.value();
    }
    return switch (_persistenceFailure) {
      ThemeModePersistenceFailure.load => restore(),
      ThemeModePersistenceFailure.save => setMode(_mode),
      null => Future<void>.value(),
    };
  }

  void _notifyStateChanged() {
    if (!_isDisposed) {
      notifyListeners();
    }
  }

  ThemeModePreferences get _resolvedPreferences =>
      _preferences ??= SharedPreferencesThemeModePreferences();

  @override
  void dispose() {
    _isDisposed = true;
    super.dispose();
  }
}

class SharedPreferencesThemeModePreferences implements ThemeModePreferences {
  SharedPreferencesThemeModePreferences([SharedPreferencesAsync? preferences])
    : _preferences = preferences ?? SharedPreferencesAsync();

  static const String _key = 'yourtj.theme-mode.v1';

  final SharedPreferencesAsync _preferences;

  @override
  Future<String?> read() => _preferences.getString(_key);

  @override
  Future<void> write(String value) => _preferences.setString(_key, value);
}

class ThemeModeScope extends InheritedNotifier<ThemeModeController> {
  const ThemeModeScope({
    required ThemeModeController controller,
    required super.child,
    super.key,
  }) : super(notifier: controller);

  static ThemeModeController of(BuildContext context) {
    final ThemeModeScope? scope = context
        .dependOnInheritedWidgetOfExactType<ThemeModeScope>();
    assert(scope != null, 'ThemeModeScope is missing above this context');
    return scope!.notifier!;
  }
}
