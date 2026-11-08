use crate::domain::category::CategoryId;
use crate::domain::video::VideoRecordId;
use crate::domain::video_category::{Verdict, VerdictAuthor, VideoCategoryVerdict};
use crate::infrastructure::shared::sqlite_connection::Database;
use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Row, params, params_from_iter};
use std::collections::{BTreeSet, HashMap};

/// Each (video, category) pair's verdict, at most one per pair.
pub trait VideoCategoryVerdictRepository: Send + Sync {
    /// Every verdict of `video_ids`, by video then category.
    fn find_many(&self, video_ids: &[VideoRecordId]) -> anyhow::Result<Vec<VideoCategoryVerdict>>;
    /// Replaces the pair's verdict.
    fn save(&self, verdict: &VideoCategoryVerdict) -> anyhow::Result<()>;
    fn delete_for_video(&self, video_id: &VideoRecordId) -> anyhow::Result<()>;
    fn delete_for_category(&self, category_id: &CategoryId) -> anyhow::Result<()>;
    /// Every `unavailable` verdict.
    fn delete_unavailable(&self) -> anyhow::Result<()>;
    /// `category_id`'s `unavailable` verdicts.
    fn delete_unavailable_for_category(&self, category_id: &CategoryId) -> anyhow::Result<()>;
    /// Deletes the AI verdicts of `video_ids`; returns the records that had
    /// one.
    fn delete_ai_for_videos(
        &self,
        video_ids: &[VideoRecordId],
    ) -> anyhow::Result<Vec<VideoRecordId>>;
    /// Deletes `category_id`'s AI verdicts; returns the records that had one.
    fn delete_ai_for_category(
        &self,
        category_id: &CategoryId,
    ) -> anyhow::Result<Vec<VideoRecordId>>;
    /// How many AI verdicts each category has; categories without any are
    /// absent.
    fn count_ai_by_category(&self) -> anyhow::Result<HashMap<CategoryId, u64>>;
}

/// A `video_category_verdicts` row as read, before its values are parsed
/// into a `VideoCategoryVerdict`.
struct VerdictRow {
    video_id: String,
    category_id: String,
    verdict: String,
    decided_by: String,
    model: Option<String>,
    reason: Option<String>,
    decided_at: String,
}

pub struct SqliteVideoCategoryVerdictRepository {
    db: Database,
}

impl SqliteVideoCategoryVerdictRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl VideoCategoryVerdictRepository for SqliteVideoCategoryVerdictRepository {
    fn find_many(&self, video_ids: &[VideoRecordId]) -> anyhow::Result<Vec<VideoCategoryVerdict>> {
        let conn = self.db.read()?;
        let placeholders = vec!["?"; video_ids.len()].join(", ");
        let mut stmt = conn
            .prepare(&format!(
                "SELECT video_id, category_id, verdict, decided_by, model, reason, decided_at
                 FROM video_category_verdicts WHERE video_id IN ({placeholders})
                 ORDER BY video_id, category_id"
            ))
            .context("failed to prepare find many video category verdicts")?;
        stmt.query_map(
            params_from_iter(video_ids.iter().map(|id| id.as_str())),
            Self::read_row,
        )
        .inspect_err(|e| tracing::error!(error = %e, "failed to find video category verdicts"))
        .context("failed to find video category verdicts")?
        .map(|row| Self::row_to_verdict(row.context("failed to read video category verdict row")?))
        .collect()
    }

    fn save(&self, verdict: &VideoCategoryVerdict) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            "INSERT INTO video_category_verdicts
                (video_id, category_id, verdict, decided_by, model, reason, decided_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (video_id, category_id) DO UPDATE SET
                verdict = excluded.verdict,
                decided_by = excluded.decided_by,
                model = excluded.model,
                reason = excluded.reason,
                decided_at = excluded.decided_at",
            params![
                verdict.video_id.as_str(),
                verdict.category_id.as_str(),
                verdict.verdict.as_str(),
                verdict.decided_by.as_str(),
                verdict.model,
                verdict.reason,
                verdict.decided_at.to_rfc3339(),
            ],
        )
        .inspect_err(|e| {
            tracing::error!(video_id = %verdict.video_id, error = %e, "failed to save video category verdict")
        })
        .context("failed to save video category verdict")?;
        Ok(())
    }

    fn delete_for_video(&self, video_id: &VideoRecordId) -> anyhow::Result<()> {
        self.delete_where("video_id = ?1", &[video_id.as_str()])
    }

    fn delete_for_category(&self, category_id: &CategoryId) -> anyhow::Result<()> {
        self.delete_where("category_id = ?1", &[category_id.as_str()])
    }

    fn delete_unavailable(&self) -> anyhow::Result<()> {
        self.delete_where("verdict = ?1", &[Verdict::Unavailable.as_str()])
    }

    fn delete_unavailable_for_category(&self, category_id: &CategoryId) -> anyhow::Result<()> {
        self.delete_where(
            "verdict = ?1 AND category_id = ?2",
            &[Verdict::Unavailable.as_str(), category_id.as_str()],
        )
    }

    fn delete_ai_for_videos(
        &self,
        video_ids: &[VideoRecordId],
    ) -> anyhow::Result<Vec<VideoRecordId>> {
        let placeholders = vec!["?"; video_ids.len()].join(", ");
        let values: Vec<&str> = std::iter::once(VerdictAuthor::Ai.as_str())
            .chain(video_ids.iter().map(VideoRecordId::as_str))
            .collect();
        self.delete_returning_videos(
            &format!("decided_by = ?1 AND video_id IN ({placeholders})"),
            &values,
        )
    }

    fn delete_ai_for_category(
        &self,
        category_id: &CategoryId,
    ) -> anyhow::Result<Vec<VideoRecordId>> {
        self.delete_returning_videos(
            "decided_by = ?1 AND category_id = ?2",
            &[VerdictAuthor::Ai.as_str(), category_id.as_str()],
        )
    }

    fn count_ai_by_category(&self) -> anyhow::Result<HashMap<CategoryId, u64>> {
        let conn = self.db.read()?;
        let mut stmt = conn
            .prepare(
                "SELECT category_id, COUNT(*) FROM video_category_verdicts
                 WHERE decided_by = ?1 GROUP BY category_id",
            )
            .context("failed to prepare count of AI verdicts")?;
        stmt.query_map(params![VerdictAuthor::Ai.as_str()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .inspect_err(|e| tracing::error!(error = %e, "failed to count AI verdicts"))
        .context("failed to count AI verdicts")?
        .map(|row| {
            let (category_id, count) = row.context("failed to read AI verdict count row")?;
            Ok((CategoryId::new(category_id)?, u64::try_from(count)?))
        })
        .collect()
    }
}

impl SqliteVideoCategoryVerdictRepository {
    /// Deletes the verdicts matching `predicate`, with `values` as `?1`, `?2`…
    fn delete_where(&self, predicate: &str, values: &[&str]) -> anyhow::Result<()> {
        let conn = self.db.write()?;
        conn.execute(
            &format!("DELETE FROM video_category_verdicts WHERE {predicate}"),
            params_from_iter(values),
        )
        .inspect_err(|e| {
            tracing::error!(predicate, ?values, error = %e, "failed to delete video category verdicts")
        })
        .context("failed to delete video category verdicts")?;
        Ok(())
    }

    /// Deletes the verdicts matching `predicate`, with `values` as `?1`, `?2`…,
    /// returning their videos, each once, in id order.
    fn delete_returning_videos(
        &self,
        predicate: &str,
        values: &[&str],
    ) -> anyhow::Result<Vec<VideoRecordId>> {
        let conn = self.db.write()?;
        let video_ids = conn
            .prepare(&format!(
                "DELETE FROM video_category_verdicts WHERE {predicate} RETURNING video_id"
            ))
            .and_then(|mut stmt| {
                stmt.query_map(params_from_iter(values), |row| row.get::<_, String>(0))?
                    .collect::<rusqlite::Result<BTreeSet<_>>>()
            })
            .inspect_err(|e| {
                tracing::error!(predicate, ?values, error = %e, "failed to delete video category verdicts")
            })
            .context("failed to delete video category verdicts")?;
        video_ids
            .into_iter()
            .map(|id| Ok(VideoRecordId::new(id)?))
            .collect()
    }

    fn read_row(row: &Row<'_>) -> rusqlite::Result<VerdictRow> {
        Ok(VerdictRow {
            video_id: row.get(0)?,
            category_id: row.get(1)?,
            verdict: row.get(2)?,
            decided_by: row.get(3)?,
            model: row.get(4)?,
            reason: row.get(5)?,
            decided_at: row.get(6)?,
        })
    }

    fn row_to_verdict(row: VerdictRow) -> anyhow::Result<VideoCategoryVerdict> {
        Ok(VideoCategoryVerdict {
            video_id: VideoRecordId::new(row.video_id)?,
            category_id: CategoryId::new(row.category_id)?,
            verdict: Verdict::parse(&row.verdict)?,
            decided_by: VerdictAuthor::parse(&row.decided_by)?,
            model: row.model,
            reason: row.reason,
            decided_at: DateTime::parse_from_rfc3339(&row.decided_at)
                .context("failed to parse stored decided_at")?
                .with_timezone(&Utc),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shared::sqlite_connection::TestDatabase;

    #[test]
    fn it_should_save_and_find_verdicts_of_several_videos() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let (chess, funny, other) = (
            ai_verdict("rec1", "chess", Verdict::Yes),
            user_verdict("rec1", "funny", Verdict::No),
            ai_verdict("rec2", "chess", Verdict::Unavailable),
        );
        let not_asked = ai_verdict("rec3", "chess", Verdict::Yes);
        [&chess, &funny, &other, &not_asked]
            .iter()
            .for_each(|verdict| repository.save(verdict).unwrap());

        let found = repository.find_many(&[video_id("rec1"), video_id("rec2")]);

        assert_eq!(
            found.map_err(|e| e.to_string()),
            Ok(vec![chess, funny, other])
        );
    }

    #[test]
    fn it_should_replace_a_pairs_verdict() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let replacement = user_verdict("rec1", "chess", Verdict::No);
        repository
            .save(&ai_verdict("rec1", "chess", Verdict::Yes))
            .unwrap();

        let result = repository.save(&replacement);

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        assert_eq!(
            repository.find_many(&[video_id("rec1")]).unwrap(),
            vec![replacement]
        );
    }

    #[test]
    fn it_should_delete_a_videos_verdicts() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let kept = ai_verdict("rec2", "chess", Verdict::Yes);
        [
            ai_verdict("rec1", "chess", Verdict::Yes),
            ai_verdict("rec1", "funny", Verdict::No),
            kept.clone(),
        ]
        .iter()
        .for_each(|verdict| repository.save(verdict).unwrap());

        let result = repository.delete_for_video(&video_id("rec1"));

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        assert_eq!(
            repository
                .find_many(&[video_id("rec1"), video_id("rec2")])
                .unwrap(),
            vec![kept]
        );
    }

    #[test]
    fn it_should_delete_a_categorys_verdicts() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let kept = ai_verdict("rec1", "funny", Verdict::No);
        [
            ai_verdict("rec1", "chess", Verdict::Yes),
            kept.clone(),
            ai_verdict("rec2", "chess", Verdict::No),
        ]
        .iter()
        .for_each(|verdict| repository.save(verdict).unwrap());

        let result = repository.delete_for_category(&CategoryId::new("chess").unwrap());

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        assert_eq!(
            repository
                .find_many(&[video_id("rec1"), video_id("rec2")])
                .unwrap(),
            vec![kept]
        );
    }

    #[test]
    fn it_should_delete_only_unavailable_verdicts() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let kept = [
            ai_verdict("rec1", "chess", Verdict::Yes),
            ai_verdict("rec1", "funny", Verdict::No),
        ];
        [
            ai_verdict("rec1", "tech", Verdict::Unavailable),
            ai_verdict("rec2", "chess", Verdict::Unavailable),
        ]
        .iter()
        .chain(&kept)
        .for_each(|verdict| repository.save(verdict).unwrap());

        let result = repository.delete_unavailable();

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        assert_eq!(
            repository
                .find_many(&[video_id("rec1"), video_id("rec2")])
                .unwrap(),
            kept.to_vec()
        );
    }

    #[test]
    fn it_should_delete_the_unavailable_verdicts_of_one_category() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let kept = [
            ai_verdict("rec1", "tech", Verdict::Unavailable),
            ai_verdict("rec2", "chess", Verdict::Yes),
        ];
        [ai_verdict("rec1", "chess", Verdict::Unavailable)]
            .iter()
            .chain(&kept)
            .for_each(|verdict| repository.save(verdict).unwrap());

        let result = repository.delete_unavailable_for_category(&CategoryId::new("chess").unwrap());

        assert_eq!(result.map_err(|e| e.to_string()), Ok(()));
        assert_eq!(
            repository
                .find_many(&[video_id("rec1"), video_id("rec2")])
                .unwrap(),
            kept.to_vec()
        );
    }

    #[test]
    fn it_should_delete_only_ai_verdicts_of_the_given_videos_returning_them() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let kept = [
            user_verdict("rec1", "funny", Verdict::No),
            ai_verdict("rec3", "chess", Verdict::Yes),
            user_verdict("rec4", "chess", Verdict::Yes),
        ];
        [
            ai_verdict("rec1", "chess", Verdict::Yes),
            ai_verdict("rec1", "tech", Verdict::No),
            ai_verdict("rec2", "chess", Verdict::Unavailable),
        ]
        .iter()
        .chain(&kept)
        .for_each(|verdict| repository.save(verdict).unwrap());

        let deleted = repository.delete_ai_for_videos(&[
            video_id("rec2"),
            video_id("rec1"),
            video_id("rec4"),
        ]);

        assert_eq!(
            deleted.map_err(|e| e.to_string()),
            Ok(vec![video_id("rec1"), video_id("rec2")])
        );
        assert_eq!(
            repository
                .find_many(&["rec1", "rec2", "rec3", "rec4"].map(video_id))
                .unwrap(),
            kept.to_vec()
        );
    }

    #[test]
    fn it_should_delete_only_ai_verdicts_of_a_category_returning_their_videos() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        let kept = [
            ai_verdict("rec1", "funny", Verdict::Yes),
            user_verdict("rec3", "chess", Verdict::No),
        ];
        [
            ai_verdict("rec2", "chess", Verdict::No),
            ai_verdict("rec1", "chess", Verdict::Unavailable),
        ]
        .iter()
        .chain(&kept)
        .for_each(|verdict| repository.save(verdict).unwrap());

        let deleted = repository.delete_ai_for_category(&CategoryId::new("chess").unwrap());

        assert_eq!(
            deleted.map_err(|e| e.to_string()),
            Ok(vec![video_id("rec1"), video_id("rec2")])
        );
        assert_eq!(
            repository
                .find_many(&["rec1", "rec2", "rec3"].map(video_id))
                .unwrap(),
            kept.to_vec()
        );
    }

    #[test]
    fn it_should_count_ai_verdicts_per_category() {
        let db = TestDatabase::new();
        let repository = SqliteVideoCategoryVerdictRepository::new(db.database());
        [
            ai_verdict("rec1", "chess", Verdict::Yes),
            ai_verdict("rec2", "chess", Verdict::No),
            ai_verdict("rec3", "chess", Verdict::Unavailable),
            user_verdict("rec4", "chess", Verdict::Yes),
            ai_verdict("rec1", "funny", Verdict::No),
            user_verdict("rec1", "tech", Verdict::Yes),
        ]
        .iter()
        .for_each(|verdict| repository.save(verdict).unwrap());

        let counts = repository.count_ai_by_category();

        assert_eq!(
            counts.map_err(|e| e.to_string()),
            Ok(HashMap::from([
                (CategoryId::new("chess").unwrap(), 3),
                (CategoryId::new("funny").unwrap(), 1),
            ]))
        );
    }

    fn ai_verdict(video: &str, category: &str, verdict: Verdict) -> VideoCategoryVerdict {
        VideoCategoryVerdict {
            video_id: video_id(video),
            category_id: CategoryId::new(category).unwrap(),
            verdict,
            decided_by: VerdictAuthor::Ai,
            model: Some("qwen3:4b".to_string()),
            reason: Some("A Sicilian game.".to_string()),
            decided_at: DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
        }
    }

    fn user_verdict(video: &str, category: &str, verdict: Verdict) -> VideoCategoryVerdict {
        VideoCategoryVerdict {
            decided_by: VerdictAuthor::User,
            model: None,
            reason: None,
            ..ai_verdict(video, category, verdict)
        }
    }

    fn video_id(value: &str) -> VideoRecordId {
        VideoRecordId::new(value).unwrap()
    }
}
