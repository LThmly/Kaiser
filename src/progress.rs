use crate::model::Library;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// UTC calendar days keep persisted activity consistent across machines and time zones.
#[derive(Clone, Serialize, Deserialize)]
pub struct StudyDay {
    pub day: u64,
    pub cards: Vec<u64>,
}

pub fn today() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86_400
}

pub fn record(library: &mut Library, day: u64, card: u64) -> bool {
    if let Some(activity) = library
        .review_activity
        .iter_mut()
        .find(|activity| activity.day == day)
    {
        if activity.cards.contains(&card) {
            return false;
        }
        activity.cards.push(card);
    } else {
        library.review_activity.push(StudyDay {
            day,
            cards: vec![card],
        });
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviews_deduplicate_persist_and_old_libraries_start_empty() {
        let mut library = Library::demo();
        assert!(record(&mut library, 20, 4));
        assert!(!record(&mut library, 20, 4));
        assert!(record(&mut library, 21, 4));
        let raw = serde_json::to_value(&library).unwrap();
        let restored: Library = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(restored.review_activity.len(), 2);
        assert_eq!(restored.review_activity[1].cards, vec![4]);
        let mut old = raw;
        old.as_object_mut().unwrap().remove("review_activity");
        let old: Library = serde_json::from_value(old).unwrap();
        assert!(old.review_activity.is_empty());
    }
}
