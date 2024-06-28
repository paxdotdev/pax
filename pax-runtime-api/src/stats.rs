use std::{cell::RefCell, collections::HashMap};

thread_local! {
    static STATS: RefCell<HashMap<String, f64>> = RefCell::default();
}

pub fn reset_stats() {
    STATS.with_borrow_mut(|stats| {
        stats.clear();
    });
}

pub fn stat_add(key: &str, val: f64) {
    STATS.with_borrow_mut(|stats| {
        *stats.entry(key.to_owned()).or_default() += val;
    })
}

pub fn print_stats() {
    STATS.with_borrow(|stats| {
        log::debug!("-------- Statistics -------");
        for (key, val) in stats {
            log::debug!("{}: {}", key, val);
        }
        log::debug!("---------------------------");
    })
}
