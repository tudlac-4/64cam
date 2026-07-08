import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

import '../api/client.dart';
import '../models/models.dart';
import '../state/auth_state.dart';
import '../state/dashboard_state.dart';

// ── Secure storage ────────────────────────────────────────────────────────────

final secureStorageProvider = Provider<FlutterSecureStorage>(
  (_) => const FlutterSecureStorage(),
);

// ── Auth ──────────────────────────────────────────────────────────────────────

class AuthNotifier extends Notifier<AuthState> {
  static const _keyBaseUrl = 'base_url';
  static const _keyToken   = 'token';

  @override
  AuthState build() {
    _loadFromStorage();
    return const AuthState.unauthenticated();
  }

  Future<void> _loadFromStorage() async {
    final storage = ref.read(secureStorageProvider);
    final baseUrl = await storage.read(key: _keyBaseUrl);
    final token   = await storage.read(key: _keyToken);
    if (baseUrl != null && token != null) {
      state = AuthState(baseUrl: baseUrl, token: token);
      _connectDashboard();
    }
  }

  Future<void> login(String baseUrl, String email, String password) async {
    final url     = baseUrl.trimRight().replaceAll(RegExp(r'/$'), '');
    final client  = ApiClient(baseUrl: url);
    final token   = await client.login(email, password);

    final storage = ref.read(secureStorageProvider);
    await storage.write(key: _keyBaseUrl, value: url);
    await storage.write(key: _keyToken,   value: token);

    state = AuthState(baseUrl: url, token: token);
    _connectDashboard();
  }

  Future<void> logout() async {
    final storage = ref.read(secureStorageProvider);
    await storage.deleteAll();
    state = const AuthState.unauthenticated();
  }

  void _connectDashboard() {
    final wsUrl = ref.read(apiClientProvider).dashboardWsUrl();
    ref.read(dashboardProvider.notifier).connect(wsUrl);
  }
}

final authProvider = NotifierProvider<AuthNotifier, AuthState>(AuthNotifier.new);

// ── API client (derived from auth) ────────────────────────────────────────────

final apiClientProvider = Provider<ApiClient>((ref) {
  final auth = ref.watch(authProvider);
  return ApiClient(baseUrl: auth.baseUrl ?? '', token: auth.token);
});

// ── Dashboard WS (status + events) ───────────────────────────────────────────

final dashboardProvider =
    NotifierProvider<DashboardNotifier, DashboardState>(DashboardNotifier.new);

// ── Cameras list (auto-refresh on token change) ───────────────────────────────

final camerasProvider = FutureProvider.autoDispose<List<Camera>>((ref) async {
  final client = ref.watch(apiClientProvider);
  return client.getCameras();
});

// ── Recordings for a specific camera ─────────────────────────────────────────

final recordingsProvider = FutureProvider.autoDispose
    .family<List<RecordingSegment>, ({String cameraId, DateTime from, DateTime to})>(
  (ref, args) async {
    final client = ref.watch(apiClientProvider);
    return client.getRecordings(
      args.cameraId,
      from: args.from,
      to:   args.to,
    );
  },
);
