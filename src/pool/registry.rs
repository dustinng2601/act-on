//! In-memory device registry, plus a small helper to load a [`Policy`]
//! from `policy.yml` and start tracking devices.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::pool::{Device, DeviceId, DeviceStatus, Policy};

#[derive(Default)]
pub struct Registry {
    inner: Arc<Mutex<HashMap<DeviceId, DeviceStatus>>>,
    pub policy: Policy,
}

impl Registry {
    /// Load a policy and initialise all devices as `Idle`.
    pub fn from_policy(policy: Policy) -> Self {
        let mut inner = HashMap::new();
        for d in &policy.devices {
            inner.insert(d.id.clone(), DeviceStatus::Idle);
        }
        Self {
            inner: Arc::new(Mutex::new(inner)),
            policy,
        }
    }

    pub fn mark_busy(&self, id: &str) {
        if let Some(v) = self.inner.lock().get_mut(id) {
            *v = DeviceStatus::Busy;
        }
    }

    pub fn mark_idle(&self, id: &str) {
        if let Some(v) = self.inner.lock().get_mut(id) {
            *v = DeviceStatus::Idle;
        }
    }

    /// Devices currently idle.
    pub fn idle_devices(&self) -> Vec<Device> {
        let map = self.inner.lock();
        self.policy
            .devices
            .iter()
            .filter(|d| matches!(map.get(&d.id), Some(DeviceStatus::Idle)))
            .cloned()
            .collect()
    }
}
