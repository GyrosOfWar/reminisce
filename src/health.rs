use sysinfo::System;

pub struct SystemHealth {
    system: System,
}

impl SystemHealth {
    pub fn new() -> Self {
        Self {
            system: System::new(),
        }
    }
}
