import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_webrtc/flutter_webrtc.dart';
import 'package:dio/dio.dart';

import '../providers/providers.dart';

class LiveViewScreen extends ConsumerStatefulWidget {
  final String cameraId;
  const LiveViewScreen({super.key, required this.cameraId});

  @override
  ConsumerState<LiveViewScreen> createState() => _LiveViewScreenState();
}

class _LiveViewScreenState extends ConsumerState<LiveViewScreen> {
  final _renderer = RTCVideoRenderer();
  RTCPeerConnection? _pc;
  bool _connected = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _renderer.initialize().then((_) => _startWhep());
    // Lock to landscape for immersive viewing
    SystemChrome.setPreferredOrientations([
      DeviceOrientation.landscapeLeft,
      DeviceOrientation.landscapeRight,
    ]);
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.immersive);
  }

  @override
  void dispose() {
    _renderer.dispose();
    _pc?.close();
    // Restore portrait
    SystemChrome.setPreferredOrientations([DeviceOrientation.portraitUp]);
    SystemChrome.setEnabledSystemUIMode(SystemUiMode.edgeToEdge);
    super.dispose();
  }

  Future<void> _startWhep() async {
    try {
      final auth = ref.read(authProvider);
      final baseUrl = auth.baseUrl!;
      final token   = auth.token!;

      // ICE configuration — STUN for same-LAN use.
      // For remote (cellular) access, add a TURN server here.
      final config = <String, dynamic>{
        'iceServers': [
          {'urls': 'stun:stun.l.google.com:19302'},
        ],
        'sdpSemantics': 'unified-plan',
      };

      final constraints = <String, dynamic>{
        'mandatory': {},
        'optional': [],
      };

      _pc = await createPeerConnection(config, constraints);

      _pc!.onTrack = (RTCTrackEvent event) {
        if (event.streams.isNotEmpty) {
          _renderer.srcObject = event.streams.first;
          if (mounted) setState(() => _connected = true);
        }
      };

      _pc!.onIceConnectionState = (state) {
        if (state == RTCIceConnectionState.RTCIceConnectionStateFailed) {
          if (mounted) setState(() => _error = 'ICE failed — check network');
        }
      };

      // Receive-only transceivers (we don't send any media)
      await _pc!.addTransceiver(
        kind: RTCRtpMediaType.RTCRtpMediaTypeVideo,
        init: RTCRtpTransceiverInit(
          direction: TransceiverDirection.RecvOnly,
        ),
      );
      await _pc!.addTransceiver(
        kind: RTCRtpMediaType.RTCRtpMediaTypeAudio,
        init: RTCRtpTransceiverInit(
          direction: TransceiverDirection.RecvOnly,
        ),
      );

      final offer = await _pc!.createOffer({});
      await _pc!.setLocalDescription(offer);

      // POST SDP offer to coordinator WHEP proxy
      final dio = Dio();
      final resp = await dio.post<String>(
        '$baseUrl/api/v1/cameras/${widget.cameraId}/whep',
        data: offer.sdp,
        options: Options(
          headers: {
            'Content-Type':  'application/sdp',
            'Authorization': 'Bearer $token',
          },
          responseType: ResponseType.plain,
        ),
      );

      if (resp.statusCode == 201 && resp.data != null) {
        await _pc!.setRemoteDescription(
          RTCSessionDescription(resp.data, 'answer'),
        );
      } else {
        setState(() => _error = 'WHEP error: HTTP ${resp.statusCode}');
      }
    } catch (e) {
      if (mounted) setState(() => _error = 'Connection failed: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      body: Stack(
        children: [
          // Video
          if (_error == null)
            Center(
              child: RTCVideoView(
                _renderer,
                objectFit: RTCVideoViewObjectFit.RTCVideoViewObjectFitContain,
              ),
            )
          else
            Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const Icon(Icons.error_outline,
                      color: Colors.white38, size: 48),
                  const SizedBox(height: 12),
                  Text(_error!, style: const TextStyle(color: Colors.white54)),
                  const SizedBox(height: 16),
                  TextButton(
                    onPressed: () {
                      setState(() => _error = null);
                      _startWhep();
                    },
                    child: const Text('Retry'),
                  ),
                ],
              ),
            ),

          // Loading indicator until first frame
          if (!_connected && _error == null)
            const Center(child: CircularProgressIndicator()),

          // Back button overlay
          Positioned(
            top: 16,
            left: 16,
            child: SafeArea(
              child: IconButton(
                icon: const Icon(Icons.arrow_back, color: Colors.white),
                onPressed: () => Navigator.of(context).pop(),
                style: IconButton.styleFrom(
                  backgroundColor: Colors.black45,
                ),
              ),
            ),
          ),

          // Status badge
          if (_connected)
            Positioned(
              top: 16,
              right: 16,
              child: SafeArea(
                child: Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                  decoration: BoxDecoration(
                    color: Colors.red,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: const Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.circle, size: 8, color: Colors.white),
                      SizedBox(width: 4),
                      Text('LIVE',
                        style: TextStyle(
                          color: Colors.white,
                          fontSize: 11,
                          fontWeight: FontWeight.w700,
                          letterSpacing: 1,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}
