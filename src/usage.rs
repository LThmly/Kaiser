use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    pub max_input_bytes: usize,
    pub history_bytes: usize,
    pub source_bytes: usize,
    pub answer_words: usize,
    pub daily_request_limit: u64,
    pub cache_entries: usize,
    pub cache_ttl_hours: u64,
    pub max_summary_requests: usize,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            max_input_bytes: 24_000,
            history_bytes: 8_000,
            source_bytes: 12_000,
            answer_words: 150,
            daily_request_limit: 50,
            cache_entries: 64,
            cache_ttl_hours: 24,
            max_summary_requests: 16,
        }
    }
}
impl Policy {
    pub fn validate(&self) -> Result<(), String> {
        for (valid, field, requirement) in [
            (
                (4096..=262144).contains(&self.max_input_bytes),
                "max_input_bytes",
                "must be between 4096 and 262144 bytes",
            ),
            (
                self.history_bytes <= self.max_input_bytes,
                "history_bytes",
                "must not exceed max_input_bytes",
            ),
            (
                self.source_bytes <= self.max_input_bytes,
                "source_bytes",
                "must not exceed max_input_bytes",
            ),
            (
                (30..=4000).contains(&self.answer_words),
                "answer_words",
                "must be between 30 and 4000 words",
            ),
            (
                self.cache_entries <= 512,
                "cache_entries",
                "must not exceed 512 entries",
            ),
            (
                self.cache_ttl_hours <= 720,
                "cache_ttl_hours",
                "must not exceed 720 hours",
            ),
            (
                (1..=64).contains(&self.max_summary_requests),
                "max_summary_requests",
                "must be between 1 and 64 requests",
            ),
        ] {
            if !valid {
                return Err(format!("model.usage.{field} {requirement}"));
            }
        }
        Ok(())
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub cached_input: u64,
    pub reasoning: u64,
}
impl Tokens {
    pub fn responses(value: &serde_json::Value) -> Option<Self> {
        Some(Self {
            input: value["input_tokens"].as_u64()?,
            output: value["output_tokens"].as_u64()?,
            cached_input: value["input_tokens_details"]["cached_tokens"]
                .as_u64()
                .unwrap_or(0),
            reasoning: value["output_tokens_details"]["reasoning_tokens"]
                .as_u64()
                .unwrap_or(0),
        })
    }
    pub fn completions(value: &serde_json::Value) -> Option<Self> {
        Some(Self {
            input: value["prompt_tokens"].as_u64()?,
            output: value["completion_tokens"].as_u64()?,
            cached_input: value["prompt_tokens_details"]["cached_tokens"]
                .as_u64()
                .unwrap_or(0),
            reasoning: value["completion_tokens_details"]["reasoning_tokens"]
                .as_u64()
                .unwrap_or(0),
        })
    }
}
#[derive(Default, Serialize, Deserialize)]
struct Day {
    requests: u64,
    completed: u64,
    cache_hits: u64,
    unreported: u64,
    tokens: Tokens,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pause {
    pub message: String,
    pub until: Option<u64>,
}
#[derive(Default, Serialize, Deserialize)]
struct Ledger {
    days: BTreeMap<u64, Day>,
    paused: BTreeMap<String, Pause>,
}
struct CacheEntry {
    key: String,
    text: String,
    at: u64,
}
struct State {
    policy: Policy,
    root: Option<PathBuf>,
    ledger: Ledger,
    cache: VecDeque<CacheEntry>,
    in_flight: bool,
}
#[derive(Clone)]
pub struct Runtime(Arc<Mutex<State>>);
pub struct InFlight(Runtime);
impl Drop for InFlight {
    fn drop(&mut self) {
        self.0
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .in_flight = false;
    }
}
impl Default for Runtime {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(State {
            policy: Policy::default(),
            root: None,
            ledger: Ledger::default(),
            cache: VecDeque::new(),
            in_flight: false,
        })))
    }
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn digest(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl Runtime {
    pub fn enter(&self) -> Result<InFlight, String> {
        let mut state = self.0.lock().map_err(|_| "Usage state is unavailable")?;
        if state.in_flight {
            return Err(
                "An AI request is still running. Wait before sending another question.".into(),
            );
        }
        state.in_flight = true;
        Ok(InFlight(self.clone()))
    }
    pub fn configure(&self, policy: Policy, root: Option<PathBuf>) -> Result<(), String> {
        policy.validate()?;
        let mut state = self.0.lock().map_err(|_| "Usage state is unavailable")?;
        state.policy = policy;
        state.root = root;
        state.cache.clear();
        Ok(())
    }
    pub fn set_policy(&self, policy: Policy) -> Result<(), String> {
        policy.validate()?;
        let mut state = self.0.lock().map_err(|_| "Usage state is unavailable")?;
        state.policy = policy;
        state.cache.clear();
        Ok(())
    }
    pub fn policy(&self) -> Policy {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .policy
            .clone()
    }
    pub fn cached(&self, key: &str) -> Result<Option<String>, String> {
        let mut state = self.0.lock().map_err(|_| "Usage state is unavailable")?;
        let ttl = state.policy.cache_ttl_hours.saturating_mul(3600);
        if state.policy.cache_entries == 0 || ttl == 0 {
            return Ok(None);
        }
        state
            .cache
            .retain(|entry| now().saturating_sub(entry.at) < ttl);
        let Some(index) = state.cache.iter().position(|entry| entry.key == key) else {
            return Ok(None);
        };
        let entry = state.cache.remove(index).unwrap();
        let text = entry.text.clone();
        state.cache.push_back(entry);
        Self::update(&mut state, |ledger| {
            ledger.days.entry(now() / 86400).or_default().cache_hits += 1;
            Ok(())
        })?;
        Ok(Some(text))
    }
    pub fn reserve(&self, scope: &str) -> Result<u64, String> {
        let mut state = self.0.lock().map_err(|_| "Usage state is unavailable")?;
        let limit = state.policy.daily_request_limit;
        let day = now() / 86400;
        Self::update(&mut state, |ledger| {
            if let Some(pause) = ledger.paused.get(scope)
                && pause.until.is_none_or(|until| until > now())
            {
                return Err(format!(
                    "{} AI requests are paused; review :usage, then use :ai-resume when ready.",
                    pause.message
                ));
            }
            let stats = ledger.days.entry(day).or_default();
            if limit > 0 && stats.requests >= limit {
                return Err(format!(
                    "Kaiser's daily limit of {limit} AI requests was reached (UTC). Local study remains available."
                ));
            }
            stats.requests += 1;
            stats.unreported += 1;
            Ok(())
        })?;
        Ok(day)
    }
    pub fn ensure_budget(&self, scope: &str, requests: usize) -> Result<(), String> {
        let mut state = self.0.lock().map_err(|_| "Usage state is unavailable")?;
        let limit = state.policy.daily_request_limit;
        Self::update(&mut state, |ledger| {
            if let Some(pause) = ledger.paused.get(scope)
                && pause.until.is_none_or(|until| until > now())
            {
                return Err(format!(
                    "{} Review :usage, then :ai-resume when ready.",
                    pause.message
                ));
            }
            let used = ledger
                .days
                .get(&(now() / 86400))
                .map_or(0, |day| day.requests);
            if limit > 0 && used.saturating_add(requests as u64) > limit {
                return Err(format!(
                    "This job needs up to {requests} AI requests; {} remain in Kaiser's daily budget. Narrow the scope or adjust the local limit.",
                    limit.saturating_sub(used)
                ));
            }
            Ok(())
        })
    }
    pub fn remember(&self, key: String, text: String) {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if state.policy.cache_entries == 0
            || state.policy.cache_ttl_hours == 0
            || text.len() > 65536
        {
            return;
        }
        state.cache.retain(|entry| entry.key != key);
        state.cache.push_back(CacheEntry {
            key,
            text,
            at: now(),
        });
        while state.cache.len() > state.policy.cache_entries {
            state.cache.pop_front();
        }
    }
    pub fn finish(
        &self,
        day: u64,
        scope: &str,
        completed: bool,
        tokens: Option<&Tokens>,
        pause: Option<Pause>,
        cache: Option<(String, String)>,
    ) -> Result<(), String> {
        let mut state = self.0.lock().map_err(|_| "Usage state is unavailable")?;
        Self::update(&mut state, |ledger| {
            let stats = ledger.days.entry(day).or_default();
            if completed {
                stats.completed += 1;
            }
            if let Some(tokens) = tokens {
                stats.unreported = stats.unreported.saturating_sub(1);
                stats.tokens.input = stats.tokens.input.saturating_add(tokens.input);
                stats.tokens.output = stats.tokens.output.saturating_add(tokens.output);
                stats.tokens.cached_input = stats
                    .tokens
                    .cached_input
                    .saturating_add(tokens.cached_input);
                stats.tokens.reasoning = stats.tokens.reasoning.saturating_add(tokens.reasoning);
            }
            if let Some(pause) = pause {
                ledger.paused.insert(scope.to_owned(), pause);
            }
            Ok(())
        })?;
        if completed
            && state.policy.cache_entries > 0
            && state.policy.cache_ttl_hours > 0
            && let Some((key, text)) = cache
            && text.len() <= 65536
        {
            state.cache.retain(|entry| entry.key != key);
            state.cache.push_back(CacheEntry {
                key,
                text,
                at: now(),
            });
            while state.cache.len() > state.policy.cache_entries {
                state.cache.pop_front();
            }
        }
        Ok(())
    }
    pub fn resume(&self, scope: &str) -> Result<(), String> {
        let mut state = self.0.lock().map_err(|_| "Usage state is unavailable")?;
        Self::update(&mut state, |ledger| {
            ledger.paused.remove(scope);
            Ok(())
        })
    }
    pub fn clear_cache(&self) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .cache
            .clear();
    }
    pub fn describe(&self, scope: Option<&str>) -> Result<String, String> {
        let mut state = self.0.lock().map_err(|_| "Usage state is unavailable")?;
        if let Some(root) = &state.root {
            state.ledger = read(root)?;
        }
        let empty = Day::default();
        let today = state.ledger.days.get(&(now() / 86400)).unwrap_or(&empty);
        let limit = if state.policy.daily_request_limit == 0 {
            "off".into()
        } else {
            state.policy.daily_request_limit.to_string()
        };
        let paused = scope
            .and_then(|s| state.ledger.paused.get(s))
            .filter(|p| p.until.is_none_or(|until| until > now()))
            .map(|p| p.message.as_str())
            .unwrap_or("None");
        Ok(format!(
            "Today · UTC\n\nAI request attempts     {}\nCompleted answers       {}\nDaily request limit     {limit}\nLocal cache hits        {}\n\nReported input tokens   {}\nReported output tokens  {}\nCached input subset     {}\nReasoning output subset {}\nUnreported attempts     {}\n\nPause: {paused}\n\nInput size cap          {} bytes\nHistory / sources       {} / {} bytes\nAnswer target           {} words\n\nToken counts come from provider responses. Unreported attempts may have consumed usage. These figures do not show your remaining ChatGPT allowance.",
            today.requests,
            today.completed,
            today.cache_hits,
            today.tokens.input,
            today.tokens.output,
            today.tokens.cached_input,
            today.tokens.reasoning,
            today.unreported,
            state.policy.max_input_bytes,
            state.policy.history_bytes,
            state.policy.source_bytes,
            state.policy.answer_words
        ))
    }
    fn update<T>(
        state: &mut State,
        change: impl FnOnce(&mut Ledger) -> Result<T, String>,
    ) -> Result<T, String> {
        let _lock = if let Some(root) = &state.root {
            fs::create_dir_all(root).map_err(|_| "Cannot create AI usage storage")?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(root, fs::Permissions::from_mode(0o700))
                    .map_err(|_| "Cannot protect AI usage storage")?;
            }
            let mut options = OpenOptions::new();
            options.read(true).write(true).create(true).truncate(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let file = options
                .open(root.join("usage.lock"))
                .map_err(|_| "Cannot open AI usage lock")?;
            file.try_lock()
                .map_err(|_| "Another Kaiser process is updating AI usage. Try again.")?;
            state.ledger = read(root)?;
            Some(file)
        } else {
            None
        };
        let result = change(&mut state.ledger)?;
        let today = now() / 86400;
        state
            .ledger
            .days
            .retain(|day, _| day.saturating_add(90) >= today);
        if let Some(root) = &state.root {
            let temp = root.join(format!("usage-{}.tmp", std::process::id()));
            let mut options = OpenOptions::new();
            options.write(true).create(true).truncate(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temp).map_err(|_| "Cannot save AI usage")?;
            let bytes =
                serde_json::to_vec(&state.ledger).map_err(|_| "Cannot serialize AI usage")?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| "Cannot save AI usage")?;
            fs::rename(temp, root.join("usage.json")).map_err(|_| "Cannot replace AI usage")?;
        }
        Ok(result)
    }
}
fn read(root: &std::path::Path) -> Result<Ledger, String> {
    match fs::read(root.join("usage.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
            "AI usage storage is invalid; requests are blocked until it is repaired".into()
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Ledger::default()),
        Err(_) => Err("Cannot read AI usage storage".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn in_flight_guard_survives_provider_switches_and_releases_on_drop() {
        let runtime = Runtime::default();
        let guard = runtime.enter().unwrap();
        let another_provider = runtime.clone();
        assert!(another_provider.enter().is_err());
        runtime.set_policy(Policy::default()).unwrap();
        assert!(another_provider.enter().is_err());
        drop(guard);
        assert!(another_provider.enter().is_ok());
    }
    #[test]
    fn request_cap_and_pauses_persist_while_cache_hits_do_not_spend_requests() {
        let root = std::env::temp_dir().join(format!("kaiser-usage-{}", std::process::id()));
        let runtime = Runtime::default();
        let policy = Policy {
            daily_request_limit: 2,
            ..Default::default()
        };
        runtime
            .configure(policy.clone(), Some(root.clone()))
            .unwrap();
        let day = runtime.reserve("account").unwrap();
        runtime
            .finish(
                day,
                "account",
                true,
                Some(&Tokens {
                    input: 100,
                    output: 20,
                    cached_input: 30,
                    reasoning: 5,
                }),
                None,
                Some(("key".into(), "answer".into())),
            )
            .unwrap();
        assert_eq!(runtime.cached("key").unwrap().as_deref(), Some("answer"));
        let day = runtime.reserve("account").unwrap();
        runtime
            .finish(
                day,
                "account",
                false,
                None,
                Some(Pause {
                    message: "Plan limit".into(),
                    until: None,
                }),
                None,
            )
            .unwrap();
        let restarted = Runtime::default();
        restarted.configure(policy, Some(root.clone())).unwrap();
        assert!(restarted.reserve("account").unwrap_err().contains("paused"));
        restarted.resume("account").unwrap();
        assert!(
            restarted
                .reserve("account")
                .unwrap_err()
                .contains("daily limit")
        );
        let ledger = read(&root).unwrap();
        let stats = ledger.days.get(&day).unwrap();
        assert_eq!(stats.requests, 2);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.tokens.input, 100);
        assert_eq!(stats.tokens.output, 20);
        assert_eq!(stats.unreported, 1);
        assert!(
            runtime
                .describe(Some("account"))
                .unwrap()
                .contains("do not show your remaining")
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(root.join("usage.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        fs::write(root.join("usage.json"), "broken").unwrap();
        assert!(restarted.reserve("another-account").is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cache_evicts_expires_disables_and_daily_counts_reset_without_guessing_plan_reset() {
        let runtime = Runtime::default();
        runtime
            .set_policy(Policy {
                cache_entries: 1,
                ..Default::default()
            })
            .unwrap();
        let day = runtime.reserve("first").unwrap();
        runtime
            .finish(
                day,
                "first",
                true,
                None,
                None,
                Some(("a".into(), "first".into())),
            )
            .unwrap();
        runtime
            .finish(
                day,
                "first",
                true,
                None,
                None,
                Some(("b".into(), "second".into())),
            )
            .unwrap();
        assert!(runtime.cached("a").unwrap().is_none());
        {
            let mut state = runtime.0.lock().unwrap();
            state.cache[0].at = 1;
            let old = state.ledger.days.remove(&day).unwrap();
            state.ledger.days.insert(day - 1, old);
            state.ledger.paused.insert(
                "limited".into(),
                Pause {
                    message: "Plan limit".into(),
                    until: None,
                },
            );
            state.ledger.paused.insert(
                "cooldown".into(),
                Pause {
                    message: "Cooldown".into(),
                    until: Some(1),
                },
            );
        }
        assert!(runtime.cached("b").unwrap().is_none());
        runtime.reserve("cooldown").unwrap();
        assert!(runtime.reserve("limited").is_err());
        runtime
            .set_policy(Policy {
                cache_entries: 0,
                ..Default::default()
            })
            .unwrap();
        assert!(runtime.cached("b").unwrap().is_none());
        assert!(
            Policy {
                max_input_bytes: 0,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
}
