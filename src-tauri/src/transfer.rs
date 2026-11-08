//! One-to-many transfer queue with automatic backups and restore.
//!
//! A `TransferPlan` (one source, N targets, a set of categories) becomes a
//! `TransferBatch` of per-target jobs. Each job is: snapshot the target
//! (auto-backup) -> merge the selected categories from the source -> write.
//! Reading and writing an account's live settings goes through the
//! `SettingsSink` trait so the Riot client integration can be swapped in later.

use crate::diff;
use crate::error::{AppError, AppResult};
use crate::model::{now_unix, Account, Category, SettingsBundle, Snapshot, SnapshotKind};
use crate::vault::{new_id, Vault};
use serde::{Deserialize, Serialize};

/// Where live settings come from and go to.
pub trait SettingsSink {
    fn read(&self, account_id: &str) -> AppResult<SettingsBundle>;
    fn write(&self, account_id: &str, bundle: &SettingsBundle) -> AppResult<()>;
}

/// Default sink: treats the newest vault snapshot as the account's live state
/// and records writes as `Applied` snapshots.
/// TODO(owner): replace with a sink backed by `riot::RiotSettingsApi` so writes
/// reach the player's cloud settings once the account is signed in.
pub struct VaultSink<'a>(pub &'a Vault);

impl SettingsSink for VaultSink<'_> {
    fn read(&self, account_id: &str) -> AppResult<SettingsBundle> {
        Ok(self.0.latest(account_id)?.map(|s| s.bundle).unwrap_or_default())
    }

    fn write(&self, account_id: &str, bundle: &SettingsBundle) -> AppResult<()> {
        self.0.snapshot(account_id, "Applied by ValSync", SnapshotKind::Applied, bundle.clone()).map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobStatus {
    Queued,
    BackingUp,
    Applying,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetJob {
    pub account_id: String,
    pub riot_id: String,
    pub status: JobStatus,
    pub backup_id: Option<String>,
    pub applied: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferPlan {
    pub source_id: String,
    pub target_ids: Vec<String>,
    pub categories: Vec<Category>,
    #[serde(default = "yes")]
    pub auto_backup: bool,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferBatch {
    pub id: String,
    pub source_id: String,
    pub categories: Vec<Category>,
    pub auto_backup: bool,
    pub jobs: Vec<TargetJob>,
    pub created_at: u64,
    pub finished_at: Option<u64>,
}

impl TransferBatch {
    pub fn progress(&self) -> f32 {
        if self.jobs.is_empty() {
            return 1.0;
        }
        let done = self.jobs.iter().filter(|j| matches!(j.status, JobStatus::Done | JobStatus::Failed)).count();
        done as f32 / self.jobs.len() as f32
    }

    pub fn is_finished(&self) -> bool {
        self.finished_at.is_some()
    }
}

#[derive(Default)]
pub struct TransferQueue {
    batches: Vec<TransferBatch>,
}

impl TransferQueue {
    /// Validates a plan against known accounts and queues it.
    pub fn enqueue(&mut self, plan: TransferPlan, accounts: &[Account]) -> AppResult<String> {
        let find = |id: &str| accounts.iter().find(|a| a.id == id);
        find(&plan.source_id).ok_or_else(|| AppError::NotFound(format!("source account {}", plan.source_id)))?;
        if plan.categories.is_empty() {
            return Err(AppError::Transfer("select at least one category".into()));
        }
        let mut jobs: Vec<TargetJob> = Vec::new();
        for id in &plan.target_ids {
            if *id == plan.source_id {
                return Err(AppError::Transfer("the source account cannot also be a target".into()));
            }
            if jobs.iter().any(|j| j.account_id == *id) {
                continue;
            }
            let acc = find(id).ok_or_else(|| AppError::NotFound(format!("target account {id}")))?;
            jobs.push(TargetJob {
                account_id: acc.id.clone(),
                riot_id: acc.riot_id.clone(),
                status: JobStatus::Queued,
                backup_id: None,
                applied: 0,
                error: None,
            });
        }
        if jobs.is_empty() {
            return Err(AppError::Transfer("select at least one target account".into()));
        }
        let mut categories = plan.categories;
        categories.sort();
        categories.dedup();
        let batch = TransferBatch {
            id: new_id(),
            source_id: plan.source_id,
            categories,
            auto_backup: plan.auto_backup,
            jobs,
            created_at: now_unix(),
            finished_at: None,
        };
        let id = batch.id.clone();
        self.batches.push(batch);
        Ok(id)
    }

    pub fn get(&self, id: &str) -> Option<&TransferBatch> {
        self.batches.iter().find(|b| b.id == id)
    }

    pub fn recent(&self, n: usize) -> Vec<TransferBatch> {
        self.batches.iter().rev().take(n).cloned().collect()
    }

    /// Runs every job of the first unfinished batch. A failing target does not
    /// stop the others; its error is recorded on the job.
    pub fn run_next(&mut self, vault: &Vault, sink: &dyn SettingsSink) -> AppResult<Option<TransferBatch>> {
        let Some(batch) = self.batches.iter_mut().find(|b| !b.is_finished()) else {
            return Ok(None);
        };
        let source = sink.read(&batch.source_id)?;
        let label = format!("Auto-backup before transfer from {}", batch.source_id);
        for job in batch.jobs.iter_mut() {
            match run_job(job, &source, &batch.categories, batch.auto_backup, &label, vault, sink) {
                Ok(()) => job.status = JobStatus::Done,
                Err(e) => {
                    log::warn!("transfer to {} failed: {e}", job.account_id);
                    job.status = JobStatus::Failed;
                    job.error = Some(e.to_string());
                }
            }
        }
        batch.finished_at = Some(now_unix());
        Ok(Some(batch.clone()))
    }
}

fn run_job(
    job: &mut TargetJob,
    source: &SettingsBundle,
    cats: &[Category],
    auto_backup: bool,
    label: &str,
    vault: &Vault,
    sink: &dyn SettingsSink,
) -> AppResult<()> {
    let current = sink.read(&job.account_id)?;
    if auto_backup {
        job.status = JobStatus::BackingUp;
        let s = vault.snapshot(&job.account_id, label, SnapshotKind::AutoBackup, current.clone())?;
        job.backup_id = Some(s.id);
    }
    job.status = JobStatus::Applying;
    let (merged, changed) = diff::apply_categories(source, &current, cats);
    if changed > 0 {
        sink.write(&job.account_id, &merged)?;
    }
    job.applied = changed;
    Ok(())
}

/// Restores a snapshot onto its account. The current state is backed up first,
/// so a restore can itself be undone from the timeline.
pub fn restore(vault: &Vault, snapshot_id: &str, sink: &dyn SettingsSink) -> AppResult<Snapshot> {
    let snap = vault.load(snapshot_id)?;
    let current = sink.read(&snap.account_id)?;
    vault.snapshot(&snap.account_id, &format!("Auto-backup before restoring \"{}\"", snap.label), SnapshotKind::AutoBackup, current)?;
    sink.write(&snap.account_id, &snap.bundle)?;
    Ok(snap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SettingValue;
    use crate::vault::tests::temp_vault;
    use std::cell::RefCell;
    use std::collections::HashMap;

    #[derive(Default)]
    struct MemorySink(RefCell<HashMap<String, SettingsBundle>>);

    impl SettingsSink for MemorySink {
        fn read(&self, id: &str) -> AppResult<SettingsBundle> {
            if id == "broken-0000" {
                return Err(AppError::Riot("client offline".into()));
            }
            Ok(self.0.borrow().get(id).cloned().unwrap_or_default())
        }
        fn write(&self, id: &str, b: &SettingsBundle) -> AppResult<()> {
            self.0.borrow_mut().insert(id.to_string(), b.clone());
            Ok(())
        }
    }

    fn accounts() -> Vec<Account> {
        ["Nightjar#EUW1", "Kestrel#2290", "quietpeak#NA5", "broken#0000"]
            .iter()
            .enumerate()
            .map(|(i, r)| Account::new(r, "EU", "Gold 2", 40, i == 0).unwrap())
            .collect()
    }

    fn sens(v: f64) -> SettingsBundle {
        let mut b = SettingsBundle::default();
        b.set(Category::Sensitivity, "MouseSensitivity", SettingValue::Float(v));
        b.set(Category::Hud, "ShowCorpses", SettingValue::Bool(v > 0.4));
        b
    }

    fn plan(targets: &[&str]) -> TransferPlan {
        TransferPlan {
            source_id: "nightjar-euw1".into(),
            target_ids: targets.iter().map(|s| s.to_string()).collect(),
            categories: vec![Category::Sensitivity],
            auto_backup: true,
        }
    }

    #[test]
    fn enqueue_validates_plan() {
        let mut q = TransferQueue::default();
        let acc = accounts();
        assert!(q.enqueue(plan(&[]), &acc).is_err());
        assert!(q.enqueue(plan(&["nightjar-euw1"]), &acc).is_err());
        assert!(q.enqueue(plan(&["nobody-0001"]), &acc).is_err());
        let id = q.enqueue(plan(&["kestrel-2290", "kestrel-2290", "quietpeak-na5"]), &acc).unwrap();
        assert_eq!(q.get(&id).unwrap().jobs.len(), 2, "duplicates are collapsed");
    }

    #[test]
    fn one_to_many_backs_up_and_applies() {
        let vault = temp_vault("transfer");
        let sink = MemorySink::default();
        sink.write("nightjar-euw1", &sens(0.32)).unwrap();
        sink.write("kestrel-2290", &sens(0.55)).unwrap();
        sink.write("quietpeak-na5", &sens(0.32)).unwrap();
        let mut q = TransferQueue::default();
        q.enqueue(plan(&["kestrel-2290", "quietpeak-na5", "broken-0000"]), &accounts()).unwrap();
        let b = q.run_next(&vault, &sink).unwrap().unwrap();
        assert!(b.is_finished() && b.progress() == 1.0);
        assert_eq!(b.jobs[0].status, JobStatus::Done);
        assert_eq!(b.jobs[0].applied, 1);
        assert_eq!(b.jobs[1].applied, 0, "already identical");
        assert_eq!(b.jobs[2].status, JobStatus::Failed);
        let k = sink.read("kestrel-2290").unwrap();
        assert_eq!(k.get(Category::Sensitivity, "MouseSensitivity"), Some(&SettingValue::Float(0.32)));
        assert_eq!(k.get(Category::Hud, "ShowCorpses"), Some(&SettingValue::Bool(true)), "HUD not selected");
        let backup = vault.load(b.jobs[0].backup_id.as_ref().unwrap()).unwrap();
        assert_eq!(backup.bundle, sens(0.55));
        assert!(q.run_next(&vault, &sink).unwrap().is_none());

        let restored = restore(&vault, &backup.id, &sink).unwrap();
        assert_eq!(sink.read("kestrel-2290").unwrap(), restored.bundle);
        std::fs::remove_dir_all(vault.root()).ok();
    }
}
