import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:video_player/video_player.dart';

import '../models/models.dart';
import '../providers/providers.dart';

class PlaybackScreen extends ConsumerStatefulWidget {
  final String cameraId;
  const PlaybackScreen({super.key, required this.cameraId});

  @override
  ConsumerState<PlaybackScreen> createState() => _PlaybackScreenState();
}

class _PlaybackScreenState extends ConsumerState<PlaybackScreen> {
  VideoPlayerController? _vpc;
  RecordingSegment? _playing;
  bool _showControls = true;

  // Query window: last 24 hours
  late final DateTime _to   = DateTime.now();
  late final DateTime _from = _to.subtract(const Duration(hours: 24));

  @override
  void dispose() {
    _vpc?.dispose();
    SystemChrome.setPreferredOrientations([DeviceOrientation.portraitUp]);
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
    super.dispose();
  }

  Future<void> _play(RecordingSegment seg) async {
    await _vpc?.dispose();
    setState(() { _playing = seg; _vpc = null; });

    final auth = ref.read(authProvider);
    final url  = ref.read(apiClientProvider)
        .segmentUrl(widget.cameraId, seg.id);

    final controller = VideoPlayerController.networkUrl(
      Uri.parse(url),
      httpHeaders: {'Authorization': 'Bearer ${auth.token}'},
    );

    await controller.initialize();
    controller.addListener(() {
      if (mounted) setState(() {});
    });
    await controller.play();

    if (mounted) {
      setState(() => _vpc = controller);
      // Enter landscape for better viewing
      SystemChrome.setPreferredOrientations([
        DeviceOrientation.landscapeLeft,
        DeviceOrientation.landscapeRight,
        DeviceOrientation.portraitUp,
      ]);
    }
  }

  void _toggleControls() => setState(() => _showControls = !_showControls);

  @override
  Widget build(BuildContext context) {
    final recordingsAsync = ref.watch(recordingsProvider((
      cameraId: widget.cameraId,
      from:     _from,
      to:       _to,
    )));

    return Scaffold(
      appBar: _playing == null
          ? AppBar(title: const Text('Recordings'))
          : null,
      body: Column(
        children: [
          // Player area
          if (_playing != null)
            GestureDetector(
              onTap: _toggleControls,
              child: Container(
                color: Colors.black,
                child: AspectRatio(
                  aspectRatio: _vpc?.value.isInitialized == true
                      ? _vpc!.value.aspectRatio
                      : 16 / 9,
                  child: Stack(
                    alignment: Alignment.center,
                    children: [
                      if (_vpc != null && _vpc!.value.isInitialized)
                        VideoPlayer(_vpc!)
                      else
                        const CircularProgressIndicator(),
                      if (_vpc != null && _showControls) ...[
                        // Play/pause overlay
                        IconButton(
                          icon: Icon(
                            _vpc!.value.isPlaying
                                ? Icons.pause_circle
                                : Icons.play_circle,
                            size: 56,
                            color: Colors.white70,
                          ),
                          onPressed: () {
                            _vpc!.value.isPlaying
                                ? _vpc!.pause()
                                : _vpc!.play();
                          },
                        ),
                        // Progress bar
                        Positioned(
                          bottom: 0,
                          left: 0,
                          right: 0,
                          child: Column(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              VideoProgressIndicator(
                                _vpc!,
                                allowScrubbing: true,
                                colors: VideoProgressColors(
                                  playedColor: Theme.of(context).colorScheme.primary,
                                  bufferedColor: Colors.white30,
                                  backgroundColor: Colors.white10,
                                ),
                              ),
                              Padding(
                                padding: const EdgeInsets.symmetric(
                                    horizontal: 8, vertical: 4),
                                child: Row(
                                  mainAxisAlignment:
                                      MainAxisAlignment.spaceBetween,
                                  children: [
                                    Text(
                                      _formatDuration(_vpc!.value.position),
                                      style: const TextStyle(
                                          color: Colors.white70, fontSize: 12),
                                    ),
                                    Text(
                                      _formatDuration(_vpc!.value.duration),
                                      style: const TextStyle(
                                          color: Colors.white70, fontSize: 12),
                                    ),
                                  ],
                                ),
                              ),
                            ],
                          ),
                        ),
                        // Back button
                        Positioned(
                          top: 8,
                          left: 8,
                          child: SafeArea(
                            child: IconButton(
                              icon: const Icon(Icons.arrow_back,
                                  color: Colors.white),
                              onPressed: () {
                                _vpc?.dispose();
                                setState(() { _vpc = null; _playing = null; });
                                SystemChrome.setPreferredOrientations(
                                    [DeviceOrientation.portraitUp]);
                              },
                              style: IconButton.styleFrom(
                                  backgroundColor: Colors.black45),
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
              ),
            ),

          // Recording list
          Expanded(
            child: recordingsAsync.when(
              loading: () => const Center(child: CircularProgressIndicator()),
              error:   (e, _) => Center(child: Text('$e')),
              data:    (segments) {
                if (segments.isEmpty) {
                  return Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Icon(Icons.videocam_off,
                            size: 48, color: Colors.white24),
                        const SizedBox(height: 12),
                        Text('No recordings in the last 24 hours',
                            style: TextStyle(
                                color: Colors.white.withOpacity(0.35))),
                      ],
                    ),
                  );
                }
                return ListView.separated(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  itemCount: segments.length,
                  separatorBuilder: (_, __) => const Divider(height: 1),
                  itemBuilder: (context, i) {
                    final seg      = segments[i];
                    final isActive = _playing?.id == seg.id;
                    return ListTile(
                      selected:       isActive,
                      selectedColor:  Theme.of(context).colorScheme.primary,
                      leading: Icon(
                        isActive ? Icons.play_circle : Icons.videocam_outlined,
                        color: isActive
                            ? Theme.of(context).colorScheme.primary
                            : null,
                      ),
                      title: Text(seg.formattedStart),
                      subtitle: Text(
                        '${_formatDuration(Duration(seconds: seg.durationSecs))}'
                        '  ·  ${seg.formattedSize}',
                      ),
                      onTap: () => _play(seg),
                    );
                  },
                );
              },
            ),
          ),
        ],
      ),
    );
  }

  static String _formatDuration(Duration d) {
    final h = d.inHours;
    final m = d.inMinutes.remainder(60).toString().padLeft(2, '0');
    final s = d.inSeconds.remainder(60).toString().padLeft(2, '0');
    return h > 0 ? '$h:$m:$s' : '$m:$s';
  }
}
