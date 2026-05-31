use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct SessionIndexEntry {
    id: String,
    thread_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodexThreadSource {
    User,
    Subagent,
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodexSessionMeta {
    pub id: String,
    pub thread_source: CodexThreadSource,
    pub cwd: Option<PathBuf>,
    pub parent_thread_id: Option<String>,
}

pub fn thread_name_for_session(session_id: &str) -> Option<String> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return None;
    }

    let path = dirs::home_dir()?.join(".codex").join("session_index.jsonl");
    thread_name_from_index(&path, session_id)
}

fn thread_name_from_index(path: &Path, session_id: &str) -> Option<String> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut latest = None;

    for line in reader.lines().map_while(Result::ok) {
        let Ok(entry) = serde_json::from_str::<SessionIndexEntry>(&line) else {
            continue;
        };
        if entry.id != session_id {
            continue;
        }
        if let Some(thread_name) = entry
            .thread_name
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
        {
            latest = Some(thread_name);
        }
    }

    latest
}

pub fn session_meta_from_transcript(path: &Path) -> Option<CodexSessionMeta> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    for line in reader.lines().map_while(Result::ok).take(32) {
        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if entry.get("type").and_then(Value::as_str) != Some("session_meta") {
            continue;
        }

        let payload = entry.get("payload")?;
        let id = payload.get("id")?.as_str()?.trim().to_string();
        if id.is_empty() {
            return None;
        }

        let thread_source = match payload
            .get("thread_source")
            .and_then(Value::as_str)
            .unwrap_or("")
        {
            "user" => CodexThreadSource::User,
            "subagent" => CodexThreadSource::Subagent,
            other => CodexThreadSource::Unknown(other.to_string()),
        };
        let cwd = payload
            .get("cwd")
            .and_then(Value::as_str)
            .map(PathBuf::from);
        let parent_thread_id = payload
            .pointer("/source/subagent/thread_spawn/parent_thread_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|parent| !parent.is_empty())
            .map(ToString::to_string);

        return Some(CodexSessionMeta {
            id,
            thread_source,
            cwd,
            parent_thread_id,
        });
    }

    None
}

pub fn session_meta_for_session(session_id: &str) -> Option<CodexSessionMeta> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return None;
    }

    let root = dirs::home_dir()?.join(".codex").join("sessions");
    session_meta_for_session_in_root(&root, session_id)
}

fn session_meta_for_session_in_root(root: &Path, session_id: &str) -> Option<CodexSessionMeta> {
    let path = find_transcript_for_session(root, session_id, 0)?;
    session_meta_from_transcript(&path)
}

fn find_transcript_for_session(root: &Path, session_id: &str, depth: usize) -> Option<PathBuf> {
    if depth > 6 {
        return None;
    }

    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_transcript_for_session(&path, session_id, depth + 1) {
                return Some(found);
            }
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if file_name.ends_with(".jsonl") && file_name.contains(session_id) {
            return Some(path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_latest_thread_name_for_session() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session_index.jsonl");
        std::fs::write(
            &path,
            r#"{"id":"s1","thread_name":"调研红绿黄灯项目"}
{"id":"other","thread_name":"其他任务"}
{"id":"s1","thread_name":"红绿灯 MVP"}
"#,
        )
        .unwrap();

        assert_eq!(
            thread_name_from_index(&path, "s1"),
            Some("红绿灯 MVP".into())
        );
    }

    #[test]
    fn ignores_missing_blank_and_malformed_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session_index.jsonl");
        std::fs::write(
            &path,
            r#"{"id":"s1","thread_name":"   "}
not-json
{"id":"other","thread_name":"其他任务"}
"#,
        )
        .unwrap();

        assert_eq!(thread_name_from_index(&path, "s1"), None);
    }

    #[test]
    fn reads_subagent_parent_from_transcript_meta() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rollout.jsonl");
        std::fs::write(
            &path,
            r#"{"timestamp":"2026-05-29T09:23:50.505Z","type":"session_meta","payload":{"id":"child-1","cwd":"/tmp/project","source":{"subagent":{"thread_spawn":{"parent_thread_id":"parent-1","depth":1,"agent_nickname":"Banach","agent_role":"worker"}}},"thread_source":"subagent"}}"#,
        )
        .unwrap();

        let meta = session_meta_from_transcript(&path).unwrap();

        assert_eq!(meta.id, "child-1");
        assert_eq!(meta.thread_source, CodexThreadSource::Subagent);
        assert_eq!(meta.parent_thread_id.as_deref(), Some("parent-1"));
        assert_eq!(meta.cwd.as_deref(), Some(Path::new("/tmp/project")));
    }

    #[test]
    fn reads_user_thread_from_transcript_meta() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rollout.jsonl");
        std::fs::write(
            &path,
            r#"{"timestamp":"2026-05-28T13:15:21.733Z","type":"session_meta","payload":{"id":"parent-1","cwd":"/tmp/project","source":"vscode","thread_source":"user"}}"#,
        )
        .unwrap();

        let meta = session_meta_from_transcript(&path).unwrap();

        assert_eq!(meta.id, "parent-1");
        assert_eq!(meta.thread_source, CodexThreadSource::User);
        assert_eq!(meta.parent_thread_id, None);
    }

    #[test]
    fn finds_session_meta_by_session_id_in_sessions_root() {
        let dir = tempfile::tempdir().unwrap();
        let transcript_dir = dir.path().join("2026").join("05").join("31");
        std::fs::create_dir_all(&transcript_dir).unwrap();
        let path = transcript_dir.join("rollout-2026-05-31T10-00-00-child-1.jsonl");
        std::fs::write(
            &path,
            r#"{"timestamp":"2026-05-31T10:00:00.000Z","type":"session_meta","payload":{"id":"child-1","cwd":"/tmp/project","source":{"subagent":{"thread_spawn":{"parent_thread_id":"parent-1","depth":1,"agent_nickname":"Banach","agent_role":"worker"}}},"thread_source":"subagent"}}"#,
        )
        .unwrap();

        let meta = session_meta_for_session_in_root(dir.path(), "child-1").unwrap();

        assert_eq!(meta.id, "child-1");
        assert_eq!(meta.thread_source, CodexThreadSource::Subagent);
        assert_eq!(meta.parent_thread_id.as_deref(), Some("parent-1"));
    }
}
