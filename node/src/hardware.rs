use common::ws::HardwareProfile;
use sysinfo::System;

pub fn detect() -> HardwareProfile {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_cores = sys.cpus().len() as u32;
    let cpu_model = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_owned())
        .unwrap_or_default();
    let ram_total_mb = sys.total_memory() / 1024 / 1024;
    let ram_available_mb = sys.available_memory() / 1024 / 1024;

    let mut hw_accel: Vec<String> = Vec::new();
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/dev/nvidia0").exists() {
            hw_accel.push("nvidia-cuda".into());
        }
        if std::path::Path::new("/dev/dri/renderD128").exists() {
            hw_accel.push("vaapi".into());
        }
        if std::path::Path::new("/dev/video0").exists() {
            hw_accel.push("v4l2".into());
        }
    }

    HardwareProfile { cpu_cores, cpu_model, ram_total_mb, ram_available_mb, hw_accel }
}
