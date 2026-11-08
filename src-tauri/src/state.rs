//! Managed application state shared by all Tauri commands.

use crate::error::{AppError, AppResult};
use crate::model::{now_unix, Account, SnapshotKind};
use crate::sample;
use crate::transfer::TransferQueue;
use crate::vault::{Vault, VaultConfig};
use std::sync::{Mutex, MutexGuard};

pub struct AppState {
    pub vault: Vault,
    pub accounts: Mutex<Vec<Account>>,
    pub queue: Mutex<TransferQueue>,
    pub config: Mutex<VaultConfig>,
}

/// Locks a mutex, recovering the data if a previous holder panicked.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl AppState {
    pub fn init() -> AppResult<AppState> {
        let vault = Vault::open(Vault::default_root())?;
        let config = vault.load_config();
        let mut accounts = vault.load_accounts()?;
        if accounts.is_empty() {
            accounts = sample::accounts();
            vault.save_accounts(&accounts)?;
            for (i, a) in accounts.iter().enumerate() {
                vault.snapshot(&a.id, "Imported from this PC", SnapshotKind::Imported, sample::bundle_for(i as u32))?;
            }
            log::info!("seeded {} demo accounts into {}", accounts.len(), vault.root().display());
        }
        Ok(AppState { vault, accounts: Mutex::new(accounts), queue: Mutex::new(TransferQueue::default()), config: Mutex::new(config) })
    }

    pub fn account(&self, id: &str) -> AppResult<Account> {
        lock(&self.accounts).iter().find(|a| a.id == id).cloned().ok_or_else(|| AppError::NotFound(format!("account {id}")))
    }

    pub fn main_account(&self) -> Option<Account> {
        let accounts = lock(&self.accounts);
        accounts.iter().find(|a| a.is_main).or(accounts.first()).cloned()
    }

    /// Stamps `last_synced` on the given accounts and persists the list.
    pub fn mark_synced(&self, ids: &[String]) -> AppResult<()> {
        let mut accounts = lock(&self.accounts);
        let now = now_unix();
        for a in accounts.iter_mut().filter(|a| ids.contains(&a.id)) {
            a.last_synced = Some(now);
        }
        self.vault.save_accounts(&accounts)
    }

    pub fn add_account(&self, account: Account) -> AppResult<Account> {
        let mut accounts = lock(&self.accounts);
        if accounts.iter().any(|a| a.id == account.id) {
            return Err(AppError::Transfer(format!("{} is already in the list", account.riot_id)));
        }
        if account.is_main {
            accounts.iter_mut().for_each(|a| a.is_main = false);
        }
        accounts.push(account.clone());
        self.vault.save_accounts(&accounts)?;
        Ok(account)
    }
}
