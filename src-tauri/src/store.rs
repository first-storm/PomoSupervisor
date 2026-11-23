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

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct Segment {
    /// "focus" | "distract" | "away"
    pub kind: &'static str,
    pub from: i64,
    pub to: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct Distraction {
    pub at: i64,
    pub secs: i64,
    pub app: String,
    pub title: String,
    pub url: Option<String>,
    pub stage: String,
    pub reason: String,
}

/// One finished round, as the Review window shows it.
#[derive(Serialize)]
pub struct Review {
    pub id: i64,
    pub task: String,
    pub started: i64,
    pub ended: i64,
    pub planned_secs: i64,
    pub focus_secs: i64,
    pub distract_secs: i64,
    pub away_secs: i64,
    pub completed: bool,
    /// This round's number among today's rounds.
    pub ordinal: i64,
    pub alerts: i64,
    pub segments: Vec<Segment>,
    pub distractions: Vec<Distraction>,
    pub stages: StageCounts,
}

struct Ev {
    at: i64,
    stage: String,
    result: String,
    app: String,
    title: String,
    url: Option<String>,
    reason: String,
}

/// Each sample stands for the time until the next one, up to `cap` seconds; longer gaps
/// (a manual pause stops sampling) count as away. Runs of the same kind are merged.
fn build_segments(started: i64, ended: i64, evs: &[Ev], cap: i64) -> (Vec<Segment>, Vec<(usize, i64)>) {
    let mut segs: Vec<Segment> = Vec::new();
    let mut push = |kind: &'static str, from: i64, to: i64| {
        if to <= from {
            return;
        }
        match segs.last_mut() {
            Some(last) if last.kind == kind && last.to == from => last.to = to,
            _ => segs.push(Segment { kind, from, to }),
        }
    };
    if let Some(first) = evs.first() {
        push("focus", started, first.at.min(ended));
    } else {
        push("focus", started, ended);
    }
    let mut prev = "focus";
    let mut durs = Vec::with_capacity(evs.len());
    for (i, e) in evs.iter().enumerate() {
        let next = evs.get(i + 1).map(|n| n.at).unwrap_or(ended).max(e.at);
        let kind = match e.result.as_str() {
            "focused" => "focus",
            "distracted" => "distract",
            "away" => "away",
            _ => prev,
        };
        let covered = (next - e.at).min(cap);
        push(kind, e.at, e.at + covered);
        push("away", e.at + covered, next);
        durs.push((i, covered));
        prev = kind;
    }
    (segs, durs)
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

    pub fn today_completed(&self) -> rusqlite::Result<i64> {
        self.conn.query_row("SELECT COALESCE(SUM(completed),0) FROM sessions WHERE started>=?1", params![day_start(0)], |r| r.get(0))
    }

    /// The given round, or the latest finished one.
    pub fn review(&self, id: Option<i64>, sample_secs: u32) -> rusqlite::Result<Option<Review>> {
        let row = self.conn.query_row(
            "SELECT id, task, started, COALESCE(ended, ?2), planned_secs, focus_secs, completed FROM sessions
             WHERE (?1 IS NULL AND ended IS NOT NULL) OR id=?1 ORDER BY started DESC LIMIT 1",
            params![id, now()],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, i64>(6)? != 0,
                ))
            },
        );
        let (id, task, started, ended, planned_secs, focus_secs, completed) = match row {
            Ok(r) => r,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e),
        };
        let mut q = self.conn.prepare(
            "SELECT at, stage, result, COALESCE(app,''), COALESCE(title,''), url, COALESCE(reason,'') FROM events
             WHERE session_id=?1 ORDER BY at, id",
        )?;
        let evs = q
            .query_map(params![id], |r| {
                Ok(Ev { at: r.get(0)?, stage: r.get(1)?, result: r.get(2)?, app: r.get(3)?, title: r.get(4)?, url: r.get(5)?, reason: r.get(6)? })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let cap = sample_secs as i64 * 3 + 3;
        let (segments, durs) = build_segments(started, ended, &evs, cap);
        let total = |k: &str| segments.iter().filter(|s| s.kind == k).map(|s| s.to - s.from).sum::<i64>();

        let mut distractions: Vec<Distraction> = Vec::new();
        let mut in_run = false;
        for (i, covered) in durs {
            let e = &evs[i];
            if e.result != "distracted" {
                in_run = false;
                continue;
            }
            match distractions.last_mut() {
                Some(d) if in_run => d.secs += covered,
                _ => distractions.push(Distraction {
                    at: e.at,
                    secs: covered,
                    app: e.app.clone(),
                    title: e.title.clone(),
                    url: e.url.clone(),
                    stage: e.stage.clone(),
                    reason: e.reason.clone(),
                }),
            }
            in_run = true;
        }

        let mut stages = StageCounts::default();
        for e in evs.iter().filter(|e| e.result == "focused" || e.result == "distracted") {
            match e.stage.as_str() {
                "rules" => stages.rules += 1,
                "text" => stages.text += 1,
                "vision" => stages.vision += 1,
                _ => {}
            }
        }
        let day = chrono::DateTime::from_timestamp(started, 0)
            .map(|t| t.with_timezone(&chrono::Local).date_naive())
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .and_then(|d| d.and_local_timezone(chrono::Local).earliest())
            .map(|t| t.timestamp())
            .unwrap_or(0);
        // Same count as the popover's "今天第 N 个": finished rounds before this one, plus one.
        let before: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE started>=?1 AND started<?2 AND completed=1",
            params![day, started],
            |r| r.get(0),
        )?;
        let ordinal = before + 1;
        let alerts: i64 = self.conn.query_row("SELECT COUNT(*) FROM alerts WHERE session_id=?1", params![id], |r| r.get(0))?;
        let distract_secs = total("distract");
        Ok(Some(Review {
            id,
            task,
            started,
            ended,
            planned_secs,
            focus_secs: (focus_secs - distract_secs).max(0),
            distract_secs,
            away_secs: total("away"),
            completed,
            ordinal,
            alerts,
            segments,
            distractions,
            stages,
        }))
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
        let r = st.review(None, 4).unwrap().unwrap();
        assert_eq!(r.task, "写论文");
        assert_eq!(r.alerts, 1);
        assert_eq!(r.stages.rules, 1);
        assert_eq!(r.distractions.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn ev(at: i64, result: &str) -> Ev {
        Ev { at, stage: "rules".into(), result: result.into(), app: "a".into(), title: String::new(), url: None, reason: String::new() }
    }

    #[test]
    fn segments_merge_and_gaps_become_away() {
        let evs = [ev(4, "focused"), ev(8, "focused"), ev(12, "distracted"), ev(16, "none"), ev(60, "focused")];
        let (segs, _) = build_segments(0, 64, &evs, 15);
        let kinds: Vec<_> = segs.iter().map(|s| (s.kind, s.from, s.to)).collect();
        assert_eq!(kinds, vec![("focus", 0, 12), ("distract", 12, 31), ("away", 31, 60), ("focus", 60, 64)]);
    }
}
