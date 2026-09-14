use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/// Expiring LRU cache. Churn can evict an idle mapping, never permanently close admission.
pub struct Identities {
    entries: HashMap<String, (String, Instant)>,
    capacity: usize,
}
impl Identities {
    pub fn new(capacity: usize) -> Self {
        Self { entries: HashMap::new(), capacity }
    }
    pub fn insert(&mut self, transport: String, application: String, now: Instant) {
        self.entries
            .retain(|_, (_, seen)| now.duration_since(*seen) < Duration::from_secs(300));
        if self.capacity == 0 {
            return;
        }
        if !self.entries.contains_key(&transport)
            && self.entries.len() >= self.capacity
            && let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, (_, seen))| *seen)
                .map(|(key, _)| key.clone())
        {
            self.entries.remove(&oldest);
        }
        self.entries.insert(transport, (application, now));
    }
    pub fn matches(&mut self, transport: &str, application: &str, now: Instant) -> bool {
        if let Some((identity, seen)) = self.entries.get_mut(transport)
            && now.duration_since(*seen) < Duration::from_secs(300)
            && identity == application
        {
            *seen = now;
            return true;
        }
        false
    }
    pub fn remove(&mut self, transport: &str) {
        self.entries.remove(transport);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn churn_expiry_and_disconnect_do_not_close_admission() {
        let now = Instant::now();
        let mut ids = Identities::new(2);
        ids.insert("old".into(), "a".into(), now);
        ids.insert("active".into(), "b".into(), now + Duration::from_secs(1));
        assert!(ids.matches("active", "b", now + Duration::from_secs(2)));
        ids.insert("new".into(), "c".into(), now + Duration::from_secs(3));
        assert!(ids.matches("new", "c", now + Duration::from_secs(4)));
        assert!(ids.matches("active", "b", now + Duration::from_secs(4)));
        assert!(!ids.matches("old", "a", now + Duration::from_secs(4)));
        assert!(!ids.matches("active", "b", now + Duration::from_secs(305)));
        ids.remove("new");
        assert!(!ids.matches("new", "c", now + Duration::from_secs(5)));
    }
}
