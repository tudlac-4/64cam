import 'package:intl/intl.dart';

class Camera {
  final String id;
  final String nodeId;
  final String name;
  final String rtspUrl;
  final String? streamPath;
  final bool enabled;
  final DateTime createdAt;

  const Camera({
    required this.id,
    required this.nodeId,
    required this.name,
    required this.rtspUrl,
    this.streamPath,
    required this.enabled,
    required this.createdAt,
  });

  factory Camera.fromJson(Map<String, dynamic> j) => Camera(
        id:         j['id'] as String,
        nodeId:     j['node_id'] as String,
        name:       j['name'] as String,
        rtspUrl:    j['rtsp_url'] as String,
        streamPath: j['stream_path'] as String?,
        enabled:    j['enabled'] as bool? ?? true,
        createdAt:  DateTime.parse(j['created_at'] as String),
      );
}

class CameraRtStatus {
  final String cameraId;
  final bool connected;
  final int readers;

  const CameraRtStatus({
    required this.cameraId,
    required this.connected,
    required this.readers,
  });
}

class RecordingSegment {
  final String id;
  final DateTime startedAt;
  final DateTime endedAt;
  final int durationSecs;
  final int sizeBytes;

  const RecordingSegment({
    required this.id,
    required this.startedAt,
    required this.endedAt,
    required this.durationSecs,
    required this.sizeBytes,
  });

  factory RecordingSegment.fromJson(Map<String, dynamic> j) => RecordingSegment(
        id:           j['id'] as String,
        startedAt:    DateTime.parse(j['started_at'] as String).toLocal(),
        endedAt:      DateTime.parse(j['ended_at'] as String).toLocal(),
        durationSecs: j['duration_secs'] as int? ?? 60,
        sizeBytes:    j['size_bytes'] as int? ?? 0,
      );

  String get formattedStart => DateFormat('MMM d, HH:mm:ss').format(startedAt);
  String get formattedSize  =>
      sizeBytes < 1024 * 1024
          ? '${(sizeBytes / 1024).toStringAsFixed(1)} KB'
          : '${(sizeBytes / (1024 * 1024)).toStringAsFixed(1)} MB';
}

class MotionEvent {
  final String cameraId;
  final DateTime occurredAt;
  final String source; // "onvif" | "diff"
  final double? score;

  const MotionEvent({
    required this.cameraId,
    required this.occurredAt,
    required this.source,
    this.score,
  });

  factory MotionEvent.fromJson(Map<String, dynamic> j) => MotionEvent(
        cameraId:   j['camera_id'] as String,
        occurredAt: DateTime.parse(j['occurred_at'] as String).toLocal(),
        source:     j['source'] as String? ?? 'diff',
        score:      (j['score'] as num?)?.toDouble(),
      );

  String get timeLabel => DateFormat('HH:mm:ss').format(occurredAt);
}
