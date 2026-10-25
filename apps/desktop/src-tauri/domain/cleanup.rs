//! Closed cleanup rules; the registry grants no filesystem permission.
use super::{
    contracts::{CleanupRule, SafetyClass},
    policy::{Denial, FileMetadata},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleId {
    UserTemp,
    ChromeHttpCache,
    EdgeHttpCache,
}

impl RuleId {
    pub const ALL: [Self; 3] = [Self::UserTemp, Self::ChromeHttpCache, Self::EdgeHttpCache];
    pub fn key(self) -> &'static str {
        match self {
            Self::UserTemp => "user-temp-v1",
            Self::ChromeHttpCache => "chrome-http-cache-v1",
            Self::EdgeHttpCache => "edge-http-cache-v1",
        }
    }
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rule| rule.key() == key)
    }
    pub fn descriptor(self) -> CleanupRule {
        let (name, description, safety) = match self {
            Self::UserTemp => (
                "Временные файлы пользователя",
                "Только обычные .tmp/.temp файлы в пользовательском Temp, не изменявшиеся минимум 7 дней. Используемые, защищённые и исключённые объекты, данные браузеров и другие форматы пропускаются.",
                SafetyClass::Safe,
            ),
            Self::ChromeHttpCache => (
                "HTTP-кеш Google Chrome",
                "Только Cache стандартного профиля, при закрытом Chrome. Ресурсы потребуется загрузить заново; офлайн-копии могут стать недоступны. Cookies, пароли и сессии не входят.",
                SafetyClass::Review,
            ),
            Self::EdgeHttpCache => (
                "HTTP-кеш Microsoft Edge",
                "Только Cache стандартного профиля, при закрытом Edge. Ресурсы потребуется загрузить заново; офлайн-копии могут стать недоступны. Cookies, пароли и сессии не входят.",
                SafetyClass::Review,
            ),
        };
        CleanupRule {
            id: self.key().into(),
            name: name.into(),
            description: description.into(),
            safety,
            min_age_hours: self.min_age_hours(),
        }
    }
    fn min_age_hours(self) -> u32 {
        match self {
            Self::UserTemp => 168,
            _ => 24,
        }
    }
    pub fn cache_relative(self) -> Option<&'static str> {
        match self {
            Self::UserTemp => None,
            Self::ChromeHttpCache => Some(r"Google\Chrome\User Data\Default\Cache"),
            Self::EdgeHttpCache => Some(r"Microsoft\Edge\User Data\Default\Cache"),
        }
    }
    pub fn process_name(self) -> Option<&'static str> {
        match self {
            Self::UserTemp => None,
            Self::ChromeHttpCache => Some("chrome.exe"),
            Self::EdgeHttpCache => Some("msedge.exe"),
        }
    }
    pub fn check_age(self, metadata: &FileMetadata, now: i64) -> Result<(), Denial> {
        let cutoff = now
            .checked_sub(i64::from(self.min_age_hours()) * 3600 * 10_000_000)
            .ok_or(Denial::TooRecent)?;
        // FILETIME is UTC in 100 ns ticks; future/invalid evidence never becomes eligible.
        if metadata.created <= 0
            || metadata.modified <= 0
            || metadata.changed <= 0
            || metadata.created > cutoff
            || metadata.modified > cutoff
            || metadata.changed > cutoff
        {
            return Err(Denial::TooRecent);
        }
        Ok(())
    }
}

pub fn rules() -> Vec<CleanupRule> {
    RuleId::ALL.into_iter().map(RuleId::descriptor).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::policy::FileIdentity;
    #[test]
    fn registry_is_closed_and_all_three_timestamps_must_be_old() {
        let now = 200 * 3600 * 10_000_000_i64;
        let mut metadata = FileMetadata {
            identity: FileIdentity {
                volume: 1,
                id: [0; 16],
            },
            size: 1,
            allocated_size: 4096,
            created: 1,
            modified: 1,
            changed: 1,
            attributes: 0,
            links: 1,
        };
        assert_eq!(rules().len(), 3);
        assert_eq!(RuleId::UserTemp.check_age(&metadata, now), Ok(()));
        for field in 0..3 {
            let mut recent = metadata.clone();
            match field {
                0 => recent.created = now,
                1 => recent.modified = now,
                _ => recent.changed = now,
            }
            assert_eq!(
                RuleId::UserTemp.check_age(&recent, now),
                Err(Denial::TooRecent)
            );
        }
        metadata.created = 0;
        assert_eq!(
            RuleId::UserTemp.check_age(&metadata, now),
            Err(Denial::TooRecent)
        );
        assert!(
            RuleId::ChromeHttpCache
                .cache_relative()
                .unwrap()
                .ends_with(r"Default\Cache")
        );
        assert_eq!(
            RuleId::ChromeHttpCache.descriptor().safety,
            SafetyClass::Review
        );
    }
}
