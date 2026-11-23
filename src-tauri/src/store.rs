use crate::sampler::Sample;
use rusqlite::{Connection, params};
use serde::Serialize;
use std::path::Path;

pub struct Store {
    conn: Connection,
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

fn day_start(days_ago: i64) -> i64 {
    let today = chrono::Local::now().date_naive() - chrono::Duration::days(days_ago);
    today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(chrono::Local).earliest().map(|t| t.timestamp()).unwrap_or(0)
}

#[derive(Serialize)]
pub struct AlertRow {
    pub at: i64,
    pub stage: String,
    pub app: String,
    pub title: String,
    pub url: Option<String>,
    pub reason: String,
    pub task: String,
}

#[derive(Serialize, Default)]
pub struct StageCounts {
    pub rules: i64,
    pub text: i64,
    pub vision: i64,
}

#[derive(Serialize)]
pub struct DayRow {
    pub day: String,
    pub completed: i64,
    pub focus_minutes: i64,
}

#[derive(Serialize)]
pub struct Stats {
    pub completed: i64,
    pub focus_minutes: i64,
    pub alerts: i64,
    pub focused_samples: i64,
    pub distracted_samples: i64,
    pub llm_calls: i64,
    pub llm_cached: i64,
    pub stages: StageCounts,
    pub recent_alerts: Vec<AlertRow>,
    pub week: Vec<DayRow>,
}

impl Store {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS sessions(id INTEGER PRIMARY KEY, task TEXT NOT NULL, started INTEGER NOT NULL,
                ended INTEGER, planned_secs INTEGER NOT NULL, focus_secs INTEGER NOT NULL DEFAULT 0, completed INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE IF NOT EXISTS events(id INTEGER PRIMARY KEY, session_id INTEGER, at INTEGER NOT NULL, stage TEXT NOT NULL,
                result TEXT NOT NULL, app TEXT, title TEXT, url TEXT, reason TEXT);
             CREATE TABLE IF NOT EXISTS alerts(id INTEGER PRIMARY KEY, session_id INTEGER, at INTEGER NOT NULL, stage TEXT NOT NULL,
                app TEXT, title TEXT, url TEXT, reason TEXT);
             CREATE TABLE IF NOT EXISTS llm_calls(id INTEGER PRIMARY KEY, at INTEGER NOT NULL, kind TEXT NOT NULL, cached INTEGER NOT NULL, ok INTEGER NOT NULL);
             CREATE INDEX IF NOT EXISTS events_at ON events(at);",
        )?;
        Ok(Self { conn })
    }

    pub fn start_session(&self, task: &str, planned_secs: u32) -> rusqlite::Result<i64> {
        self.conn.execute("INSERT INTO sessions(task, started, planned_secs) VALUES (?1, ?2, ?3)", params![task, now(), planned_secs])?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn end_session(&self, id: i64, focus_secs: u32, completed: bool) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE sessions SET ended=?2, focus_secs=?3, completed=?4 WHERE id=?1",
            params![id, now(), focus_secs, completed as i64],
        )?;
        Ok(())
    }

    pub fn log_event(&self, session: Option<i64>, stage: &str, result: &str, s: &Sample, reason: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO events(session_id, at, stage, result, app, title, url, reason) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![session, now(), stage, result, s.app, s.title, s.url, reason],
        )?;
        Ok(())
    }

    pub fn log_alert(&self, session: Option<i64>, stage: &str, s: &Sample, reason: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO alerts(session_id, at, stage, app, title, url, reason) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![session, now(), stage, s.app, s.title, s.url, reason],
        )?;
        Ok(())
    }

    pub fn log_llm(&self, kind: &str, cached: bool, ok: bool) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO llm_calls(at, kind, cached, ok) VALUES (?1,?2,?3,?4)",
            params![now(), kind, cached as i64, ok as i64],
        )?;
        Ok(())
    }

    pub fn stats(&self) -> rusqlite::Result<Stats> {
        let since = day_start(0);
        let one = |sql: &str| -> rusqlite::Result<i64> { self.conn.query_row(sql, params![since], |r| r.get(0)) };
        let mut stages = StageCounts::default();
        let mut q = self.conn.prepare(
            "SELECT stage, COUNT(*) FROM events WHERE at>=?1 AND result IN ('focused','distracted') GROUP BY stage",
        )?;
        for row in q.query_map(params![since], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
            let (stage, n) = row?;
            match stage.as_str() {
                "rules" => stages.rules = n,
                "text" => stages.text = n,
                "vision" => stages.vision = n,
                _ => {}
            }
        }
        let mut q = self.conn.prepare(
            "SELECT a.at, a.stage, COALESCE(a.app,''), COALESCE(a.title,''), a.url, COALESCE(a.reason,''), COALESCE(s.task,'')
             FROM alerts a LEFT JOIN sessions s ON s.id=a.session_id WHERE a.at>=?1 ORDER BY a.at DESC LIMIT 50",
        )?;
        let recent_alerts = q
            .query_map(params![since], |r| {
                Ok(AlertRow { at: r.get(0)?, stage: r.get(1)?, app: r.get(2)?, title: r.get(3)?, url: r.get(4)?, reason: r.get(5)?, task: r.get(6)? })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut week = Vec::new();
        for d in (0..7).rev() {
            let (from, to) = (day_start(d), day_start(d - 1));
            let (completed, secs): (i64, i64) = self.conn.query_row(
                "SELECT COALESCE(SUM(completed),0), COALESCE(SUM(focus_secs),0) FROM sessions WHERE started>=?1 AND started<?2",
                params![from, to],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let day = chrono::DateTime::from_timestamp(from, 0)
                .map(|t| t.with_timezone(&chrono::Local).format("%m-%d").to_string())
                .unwrap_or_default();
            week.push(DayRow { day, completed, focus_minutes: secs / 60 });
        }
        Ok(Stats {
            completed: one("SELECT COALESCE(SUM(completed),0) FROM sessions WHERE started>=?1")?,
            focus_minutes: one("SELECT COALESCE(SUM(focus_secs),0) FROM sessions WHERE started>=?1")? / 60,
            alerts: one("SELECT COUNT(*) FROM alerts WHERE at>=?1")?,
            focused_samples: one("SELECT COUNT(*) FROM events WHERE at>=?1 AND result='focused'")?,
            distracted_samples: one("SELECT COUNT(*) FROM events WHERE at>=?1 AND result='distracted'")?,
            llm_calls: one("SELECT COUNT(*) FROM llm_calls WHERE at>=?1 AND cached=0")?,
            llm_cached: one("SELECT COUNT(*) FROM llm_calls WHERE at>=?1 AND cached=1")?,
            stages,
            recent_alerts,
            week,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_stats() {
        let dir = std::env::temp_dir().join(format!("pomo-test-{}", std::process::id()));
        let st = Store::open(&dir.join("t.db")).unwrap();
        let id = st.start_session("写论文", 1500).unwrap();
        let s = Sample { app: "Firefox".into(), title: "bilibili".into(), ..Default::default() };
        st.log_event(Some(id), "rules", "distracted", &s, "黑名单").unwrap();
        st.log_alert(Some(id), "rules", &s, "黑名单").unwrap();
        st.log_llm("text", false, true).unwrap();
        st.end_session(id, 1500, true).unwrap();
        let stats = st.stats().unwrap();
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.focus_minutes, 25);
        assert_eq!(stats.alerts, 1);
        assert_eq!(stats.stages.rules, 1);
        assert_eq!(stats.recent_alerts[0].task, "写论文");
        assert_eq!(stats.week.len(), 7);
        let _ = std::fs::remove_dir_all(dir);
    }
}
