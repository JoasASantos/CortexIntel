//! In-process event bus. Every layer (router, governor, caches, memory,
//! orchestrator, pipeline) emits structured events here; sinks are the
//! per-job log the GUI console polls, a global ring buffer (`/api/events`) and
//! stderr when verbose. Job scoping is thread-local: a job thread calls
//! [`begin_job`] once and every event it (or its callees) emits is attributed
//! to that job.

use serde::Serialize;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub ts: u64,
    pub topic: String,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job: Option<String>,
}

thread_local! {
    static CURRENT_JOB: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn ring() -> &'static Mutex<VecDeque<Event>> {
    static R: OnceLock<Mutex<VecDeque<Event>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(VecDeque::with_capacity(512)))
}

fn job_logs() -> &'static Mutex<HashMap<String, Vec<String>>> {
    static L: OnceLock<Mutex<HashMap<String, Vec<String>>>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Attribute every event emitted from this thread to `job_id`.
pub fn begin_job(job_id: &str) {
    CURRENT_JOB.with(|c| *c.borrow_mut() = Some(job_id.to_string()));
    job_logs().lock().unwrap().entry(job_id.to_string()).or_default();
}

pub fn end_job() {
    CURRENT_JOB.with(|c| *c.borrow_mut() = None);
}

pub fn current_job() -> Option<String> {
    CURRENT_JOB.with(|c| c.borrow().clone())
}

/// Emit an event. `topic` is dotted (`llm.route`, `cache.hit`, `governor.budget`,
/// `memory.retrieve`, `plan.step`, `pipeline.stage`).
pub fn emit(topic: &str, msg: impl Into<String>) {
    emit_data(topic, msg, None)
}

pub fn emit_data(topic: &str, msg: impl Into<String>, data: Option<serde_json::Value>) {
    let msg = msg.into();
    let job = current_job();
    let ev = Event { ts: now_ms(), topic: topic.to_string(), msg: msg.clone(), data, job: job.clone() };
    if std::env::var("CORTEX_VERBOSE").map(|v| v == "1").unwrap_or(false) {
        eprintln!("  · [{}] {}", topic, msg);
    }
    {
        let mut r = ring().lock().unwrap();
        if r.len() >= 512 {
            r.pop_front();
        }
        r.push_back(ev);
    }
    if let Some(j) = job {
        let mut m = job_logs().lock().unwrap();
        let v = m.entry(j).or_default();
        if v.len() < 2000 {
            v.push(format!("[{}] {}", topic, msg));
        }
    }
}

/// Log lines collected for a job so far (oldest first).
pub fn job_log(job_id: &str) -> Vec<String> {
    job_logs().lock().unwrap().get(job_id).cloned().unwrap_or_default()
}

/// Drop a finished job's log (called after the GUI fetched the final status).
pub fn forget_job(job_id: &str) {
    job_logs().lock().unwrap().remove(job_id);
}

/// Global events newer than `since_ms` (for a live activity feed).
pub fn events_since(since_ms: u64, limit: usize) -> Vec<Event> {
    let r = ring().lock().unwrap();
    r.iter().filter(|e| e.ts > since_ms).rev().take(limit).cloned().collect::<Vec<_>>().into_iter().rev().collect()
}
