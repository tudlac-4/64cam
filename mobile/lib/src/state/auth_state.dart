class AuthState {
  final String? baseUrl;
  final String? token;

  const AuthState({this.baseUrl, this.token});
  const AuthState.unauthenticated() : baseUrl = null, token = null;

  bool get isAuthenticated => token != null && baseUrl != null;

  AuthState copyWith({String? baseUrl, String? token}) => AuthState(
        baseUrl: baseUrl ?? this.baseUrl,
        token:   token   ?? this.token,
      );

  AuthState cleared() => const AuthState.unauthenticated();
}
