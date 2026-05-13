use std::collections::HashSet;
use std::sync::Mutex;

pub const MAX_PINS: usize = 30;

#[derive(thiserror::Error, Debug)]
pub enum RegistryError {
    #[error("pin limit reached ({0})")]
    Full(usize),
}

pub struct PinRegistry {
    next_id: Mutex<u32>,
    active: Mutex<HashSet<String>>,
}

impl PinRegistry {
    pub fn new() -> Self {
        Self {
            next_id: Mutex::new(0),
            active: Mutex::new(HashSet::new()),
        }
    }

    pub fn reserve(&self) -> Result<String, RegistryError> {
        let mut active = self.active.lock().unwrap();
        if active.len() >= MAX_PINS {
            return Err(RegistryError::Full(MAX_PINS));
        }
        let mut id = self.next_id.lock().unwrap();
        let label = format!("pin-{}", *id);
        *id = id.wrapping_add(1);
        active.insert(label.clone());
        Ok(label)
    }

    pub fn release(&self, label: &str) {
        self.active.lock().unwrap().remove(label);
    }

    pub fn len(&self) -> usize {
        self.active.lock().unwrap().len()
    }
}

impl Default for PinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_returns_unique_monotonic_labels() {
        let r = PinRegistry::new();
        let a = r.reserve().unwrap();
        let b = r.reserve().unwrap();
        assert_eq!(a, "pin-0");
        assert_eq!(b, "pin-1");
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn release_drops_from_active() {
        let r = PinRegistry::new();
        let a = r.reserve().unwrap();
        r.release(&a);
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn reserve_caps_at_max() {
        let r = PinRegistry::new();
        for _ in 0..MAX_PINS {
            r.reserve().unwrap();
        }
        assert!(matches!(r.reserve(), Err(RegistryError::Full(_))));
    }
}
