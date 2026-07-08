import 'package:dio/dio.dart';

import '../models/models.dart';

class ApiClient {
  final String baseUrl;
  final String? token;
  late final Dio _dio;

  ApiClient({required this.baseUrl, this.token}) {
    _dio = Dio(BaseOptions(
      baseUrl: '$baseUrl/api/v1',
      connectTimeout: const Duration(seconds: 10),
      receiveTimeout: const Duration(seconds: 30),
      headers: {
        if (token != null) 'Authorization': 'Bearer $token',
        'Content-Type': 'application/json',
      },
    ));
  }

  // ── Auth ──────────────────────────────────────────────────────────────────

  Future<String> login(String email, String password) async {
    final resp = await _dio.post<Map<String, dynamic>>(
      '/auth/login',
      data: {'email': email, 'password': password},
    );
    return resp.data!['access_token'] as String;
  }

  // ── Cameras ───────────────────────────────────────────────────────────────

  Future<List<Camera>> getCameras() async {
    final resp = await _dio.get<List<dynamic>>('/cameras');
    return resp.data!
        .cast<Map<String, dynamic>>()
        .map(Camera.fromJson)
        .toList();
  }

  // ── Recordings ────────────────────────────────────────────────────────────

  Future<List<RecordingSegment>> getRecordings(
    String cameraId, {
    required DateTime from,
    required DateTime to,
  }) async {
    final resp = await _dio.get<List<dynamic>>(
      '/cameras/$cameraId/recordings',
      queryParameters: {
        'from': from.toUtc().toIso8601String(),
        'to':   to.toUtc().toIso8601String(),
      },
    );
    return resp.data!
        .cast<Map<String, dynamic>>()
        .map(RecordingSegment.fromJson)
        .toList();
  }

  /// URL to stream a single recorded segment through the coordinator proxy.
  /// Token is in the query param because VideoPlayerController can't set headers.
  String segmentUrl(String cameraId, String recordingId) =>
      '$baseUrl/api/v1/cameras/$cameraId/segments/$recordingId'
      '?token=${Uri.encodeComponent(token ?? '')}';

  /// WebSocket URL for the dashboard event/status feed.
  String dashboardWsUrl() =>
      '${baseUrl.replaceAll('https://', 'wss://').replaceAll('http://', 'ws://')}'
      '/api/v1/ws/dashboard?token=${Uri.encodeComponent(token ?? '')}';
}
