use std::path::PathBuf;

pub struct NodeConfig {
    pub coordinator_url:  String,
    pub data_dir:         PathBuf,
    pub mediamtx_binary:  PathBuf,
    pub mediamtx_api_port: u16,
    pub mediamtx_rtsp_port: u16,
    pub heartbeat_secs:   u64,
    /// HTTP server port for segment serving and clip export (default 8890)
    pub http_port:        u16,
}

impl NodeConfig {
    pub fn from_env() -> Self {
        let data_dir = std::env::var("DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                directories::ProjectDirs::from("", "", "64cam-node")
                    .map(|d| d.data_local_dir().to_owned())
                    .unwrap_or_else(|| PathBuf::from("."))
            });

        Self {
            coordinator_url: std::env::var("COORDINATOR_URL")
                .unwrap_or_else(|_| "http://localhost:8080".into()),
            data_dir: data_dir.clone(),
            mediamtx_binary: std::env::var("MEDIAMTX_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("mediamtx")),
            mediamtx_api_port:  env_u16("MEDIAMTX_API_PORT", 9997),
            mediamtx_rtsp_port: env_u16("MEDIAMTX_RTSP_PORT", 8554),
            heartbeat_secs:     30,
            http_port:          env_u16("NODE_HTTP_PORT", 8890),
        }
    }

    pub fn mediamtx_config_path(&self) -> PathBuf {
        self.data_dir.join("mediamtx.yml")
    }

    pub fn recordings_dir(&self) -> PathBuf {
        self.data_dir.join("recordings")
    }

    pub fn segment_queue_path(&self) -> PathBuf {
        self.data_dir.join("segment_queue.jsonl")
    }
}

fn env_u16(key: &str, default: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}
