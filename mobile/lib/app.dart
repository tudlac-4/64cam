import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import 'src/core/theme.dart';
import 'src/providers/providers.dart';
import 'src/screens/login_screen.dart';
import 'src/screens/home_screen.dart';
import 'src/screens/live_view_screen.dart';
import 'src/screens/playback_screen.dart';

final _router = GoRouter(
  initialLocation: '/login',
  routes: [
    GoRoute(path: '/login', builder: (_, __) => const LoginScreen()),
    GoRoute(
      path: '/home',
      builder: (_, __) => const HomeScreen(),
      routes: [
        GoRoute(
          path: 'live/:cameraId',
          builder: (_, state) =>
              LiveViewScreen(cameraId: state.pathParameters['cameraId']!),
        ),
        GoRoute(
          path: 'playback/:cameraId',
          builder: (_, state) =>
              PlaybackScreen(cameraId: state.pathParameters['cameraId']!),
        ),
      ],
    ),
  ],
  redirect: (context, state) {
    // auth-guard — resolved by provider in the widget tree
    return null;
  },
);

class Cam64App extends ConsumerWidget {
  const Cam64App({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // Redirect to /login when auth state changes
    ref.listen(authProvider, (prev, next) {
      if (!next.isAuthenticated) {
        _router.go('/login');
      }
    });

    return MaterialApp.router(
      title: '64cam',
      theme: AppTheme.dark(),
      routerConfig: _router,
      debugShowCheckedModeBanner: false,
    );
  }
}
