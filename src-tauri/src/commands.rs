//! Tauri command layer: thin wrappers that translate IPC calls into service calls.

use crate::crosshair::{self, CrosshairCode};
use crate::diff::{self, DiffReport, KeyRef};
use crate::error::{AppError, AppResult};
use crate::model::{Account, Category, SettingValue, SnapshotKind, SnapshotSummary};
use crate::riot::{self, RiotStatus};
use crate::state::{lock, AppState};
use crate::transfer::{self, TransferBatch, TransferPlan, VaultSink};
use crate::vault::VaultConfig;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInfo {
    pub category: Category,
    pub label: &'static str,
    pub count: usize,
    pub unit: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    #[serde(flatten)]
    pub account: Account,
    pub rank_initial: char,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub accounts: Vec<AccountView>,
    pub categories: Vec<CategoryInfo>,
    pub config: VaultConfig,
    pub vault_path: String,
    pub riot: RiotStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrosshairView {
    pub name: String,
    pub code: String,
    pub hex: String,
    pub description: String,
    pub parsed: CrosshairCode,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyResult {
    pub backup: Option<SnapshotSummary>,
    pub applied: SnapshotSummary,
    pub changed: usize,
}

fn categories_of(state: &AppState, account_id: &str) -> AppResult<Vec<CategoryInfo>> {
    let bundle = state.vault.latest(account_id)?.map(|s| s.bundle).unwrap_or_default();
    Ok(Category::ALL
        .iter()
        .map(|c| {
            let count = bundle.count(*c);
            CategoryInfo { category: *c, label: c.label(), count, unit: c.unit(count) }
        })
        .collect())
}

fn view(name: &str, code: &str) -> AppResult<CrosshairView> {
    let parsed = crosshair::parse(code)?;
    Ok(CrosshairView {
        name: name.to_string(),
        code: crosshair::normalize(code)?,
        hex: parsed.primary.hex(),
        description: parsed.primary.describe(),
        parsed,
    })
}

#[tauri::command]
pub fn get_overview(state: State<'_, AppState>) -> AppResult<Overview> {
    let main = state.main_account().ok_or_else(|| AppError::NotFound("no accounts yet".into()))?;
    Ok(Overview {
        accounts: lock(&state.accounts).iter().map(|a| AccountView { rank_initial: a.rank_initial(), account: a.clone() }).collect(),
        categories: categories_of(&state, &main.id)?,
        config: lock(&state.config).clone(),
        vault_path: state.vault.root().display().to_string(),
        riot: riot::status(),
    })
}

#[tauri::command]
pub fn category_counts(state: State<'_, AppState>, account_id: String) -> AppResult<Vec<CategoryInfo>> {
    categories_of(&state, &account_id)
}

#[tauri::command]
pub fn add_account(state: State<'_, AppState>, riot_id: String, region: String, rank: String) -> AppResult<Account> {
    state.add_account(Account::new(&riot_id, &region, &rank, 1, false)?)
}

#[tauri::command]
pub fn compute_diff(state: State<'_, AppState>, source_id: String, target_id: String, categories: Vec<Category>) -> AppResult<DiffReport> {
    let sink = VaultSink(&state.vault);
    let (s, t) = (transfer::SettingsSink::read(&sink, &source_id)?, transfer::SettingsSink::read(&sink, &target_id)?);
    Ok(diff::diff(&s, &t, &categories))
}

/// Cherry-pick: writes only the ticked keys (all differences when `keys` is empty),
/// with an auto-backup of the target first.
#[tauri::command]
pub fn apply_selected(state: State<'_, AppState>, source_id: String, target_id: String, keys: Vec<KeyRef>) -> AppResult<ApplyResult> {
    use transfer::SettingsSink;
    state.account(&source_id)?;
    state.account(&target_id)?;
    let sink = VaultSink(&state.vault);
    let (source, target) = (sink.read(&source_id)?, sink.read(&target_id)?);
    let keys = if keys.is_empty() { diff::diff(&source, &target, &[]).actionable() } else { keys };
    let (merged, changed) = diff::cherry_pick(&source, &target, &keys)?;
    let backup = if lock(&state.config).auto_backup {
        let label = format!("Auto-backup before cherry-pick from {source_id}");
        Some(state.vault.snapshot(&target_id, &label, SnapshotKind::AutoBackup, target)?)
    } else {
        None
    };
    let label = format!("Cherry-picked {changed} settings from {source_id}");
    let applied = state.vault.snapshot(&target_id, &label, SnapshotKind::Applied, merged)?;
    state.mark_synced(&[target_id])?;
    Ok(ApplyResult { backup, applied, changed })
}

/// One-to-many: queues the plan and runs it right away.
#[tauri::command]
pub fn start_transfer(state: State<'_, AppState>, plan: TransferPlan) -> AppResult<TransferBatch> {
    let accounts = lock(&state.accounts).clone();
    let mut queue = lock(&state.queue);
    let id = queue.enqueue(plan, &accounts)?;
    queue.run_next(&state.vault, &VaultSink(&state.vault))?;
    let batch = queue.get(&id).cloned().ok_or_else(|| AppError::NotFound(format!("batch {id}")))?;
    drop(queue);
    log::info!("transfer {} finished at {:.0}%", batch.id, batch.progress() * 100.0);
    let keep = lock(&state.config).keep_auto_backups;
    let done: Vec<String> = batch.jobs.iter().filter(|j| j.status == transfer::JobStatus::Done).map(|j| j.account_id.clone()).collect();
    for id in &done {
        state.vault.prune(id, keep)?;
    }
    state.mark_synced(&done)?;
    Ok(batch)
}

#[tauri::command]
pub fn recent_transfers(state: State<'_, AppState>) -> Vec<TransferBatch> {
    lock(&state.queue).recent(10)
}

#[tauri::command]
pub fn list_backups(state: State<'_, AppState>, account_id: Option<String>) -> AppResult<Vec<SnapshotSummary>> {
    state.vault.list(account_id.as_deref())
}

#[tauri::command]
pub fn create_snapshot(state: State<'_, AppState>, account_id: String, label: String) -> AppResult<SnapshotSummary> {
    use transfer::SettingsSink;
    let bundle = VaultSink(&state.vault).read(&account_id)?;
    let label = if label.trim().is_empty() { "Manual snapshot".to_string() } else { label };
    state.vault.snapshot(&account_id, &label, SnapshotKind::Manual, bundle)
}

#[tauri::command]
pub fn restore_backup(state: State<'_, AppState>, snapshot_id: String) -> AppResult<SnapshotSummary> {
    let snap = transfer::restore(&state.vault, &snapshot_id, &VaultSink(&state.vault))?;
    state.mark_synced(&[snap.account_id.clone()])?;
    Ok(snap.summary(0))
}

#[tauri::command]
pub fn list_crosshairs(state: State<'_, AppState>, account_id: String) -> AppResult<Vec<CrosshairView>> {
    let bundle = state.vault.latest(&account_id)?.map(|s| s.bundle).unwrap_or_default();
    let Some(map) = bundle.categories.get(&Category::Crosshair) else { return Ok(Vec::new()) };
    map.iter()
        .filter_map(|(name, v)| match v {
            SettingValue::Text(code) => Some(view(name, code)),
            _ => None,
        })
        .collect()
}

#[tauri::command]
pub fn parse_crosshair(code: String) -> AppResult<CrosshairView> {
    view("Pasted code", &code)
}

#[tauri::command]
pub fn encode_crosshair(code: CrosshairCode) -> String {
    crosshair::encode(&code)
}

#[tauri::command]
pub fn riot_status() -> RiotStatus {
    riot::status()
}

#[tauri::command]
pub fn riot_probe() -> riot::ClientProbe {
    riot::probe()
}

/// Pulls the signed-in account's live settings into the vault.
#[tauri::command]
pub fn import_from_client(state: State<'_, AppState>, account_id: String) -> AppResult<SnapshotSummary> {
    use riot::RiotSettingsApi;
    state.account(&account_id)?;
    let bundle = riot::LocalClientApi::connect()?.fetch_settings()?;
    state.vault.snapshot(&account_id, "Imported from Riot Client", SnapshotKind::Imported, bundle)
}

/// Pushes a vault snapshot to the signed-in account. Refuses while the game runs.
#[tauri::command]
pub fn push_to_client(state: State<'_, AppState>, snapshot_id: String) -> AppResult<()> {
    use riot::RiotSettingsApi;
    if riot::status().game_running {
        return Err(AppError::Riot("close VALORANT before pushing settings".into()));
    }
    let snap = state.vault.load(&snapshot_id)?;
    riot::LocalClientApi::connect()?.push_settings(&snap.bundle)
}

#[tauri::command]
pub fn set_config(state: State<'_, AppState>, config: VaultConfig) -> AppResult<VaultConfig> {
    // Credentials are never stored, whatever the UI sends.
    let config = VaultConfig { store_credentials: false, ..config };
    state.vault.save_config(&config)?;
    *lock(&state.config) = config.clone();
    Ok(config)
}
