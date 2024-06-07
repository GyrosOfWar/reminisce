use sysinfo::System;
use tokio::sync::Mutex;
use tracing::info;

const CPU_THRESHOLD: f32 = 17.5;
// 4 GB
const MEM_THRESHOLD: u64 = 4 * 1024 * 1024 * 1024;

pub struct SystemHealth {
    system: Mutex<System>,
}

impl SystemHealth {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_all();

        Self {
            system: Mutex::new(system),
        }
    }

    pub async fn load_below_threshold(&self) -> bool {
        let mut system = self.system.lock().await;
        system.refresh_all();
        let cpus = system.cpus();
        let average_cpu_load = cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32;
        info!("average cpu load: {average_cpu_load:.2}%");

        let free_memory = system.total_memory() - system.used_memory();
        let free_memory_gb = free_memory as f64 / (1024.0 * 1024.0 * 1024.0);
        info!("free memory: {free_memory_gb} GB");

        average_cpu_load < CPU_THRESHOLD && free_memory > MEM_THRESHOLD
    }
}
