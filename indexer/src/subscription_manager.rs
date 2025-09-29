use std::collections::{HashSet, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};
use chrono::{DateTime, Utc, Duration as ChronoDuration};

#[derive(Debug, Clone)]
pub struct AddressInfo {
    pub address: String,
    pub user_id: Option<uuid::Uuid>,
    pub added_at: DateTime<Utc>,
    pub last_seen: Option<DateTime<Utc>>,
    pub balance: i64,
    pub is_active: bool,
}

#[derive(Debug)]
pub struct SubscriptionStats {
    pub total_addresses: usize,
    pub active_addresses: usize,
    pub inactive_addresses: usize,
    pub addresses_with_balance: usize,
    pub total_balance: i64,
    pub newest_address_age: Option<ChronoDuration>,
    pub oldest_address_age: Option<ChronoDuration>,
}

pub struct SubscriptionManager {
    addresses: Arc<RwLock<HashMap<String, AddressInfo>>>,
    recently_added: Arc<RwLock<HashSet<String>>>,
    batch_size: usize,
    max_addresses: usize,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            addresses: Arc::new(RwLock::new(HashMap::new())),
            recently_added: Arc::new(RwLock::new(HashSet::new())),
            batch_size: 1000, // Process addresses in batches
            max_addresses: 100_000, // Reasonable limit for memory usage
        }
    }

    pub fn with_limits(batch_size: usize, max_addresses: usize) -> Self {
        Self {
            addresses: Arc::new(RwLock::new(HashMap::new())),
            recently_added: Arc::new(RwLock::new(HashSet::new())),
            batch_size,
            max_addresses,
        }
    }

    /// Add a single address to monitoring
    pub async fn add_address(&self, address: String) -> bool {
        self.add_address_with_user(address, None).await
    }

    /// Add address with associated user ID
    pub async fn add_address_with_user(&self, address: String, user_id: Option<uuid::Uuid>) -> bool {
        let mut addresses = self.addresses.write().await;
        let mut recently_added = self.recently_added.write().await;

        // Check if we're at the limit
        if addresses.len() >= self.max_addresses && !addresses.contains_key(&address) {
            warn!("Address limit reached ({}), cannot add: {}", self.max_addresses, address);
            return false;
        }

        let is_new = !addresses.contains_key(&address);
        
        let address_info = AddressInfo {
            address: address.clone(),
            user_id,
            added_at: Utc::now(),
            last_seen: None,
            balance: 0,
            is_active: true,
        };

        addresses.insert(address.clone(), address_info);
        
        if is_new {
            recently_added.insert(address.clone());
            debug!("Added new address to monitoring: {}", address);
        } else {
            debug!("Updated existing address: {}", address);
        }

        is_new
    }

    /// Add multiple addresses in batch
    pub async fn add_addresses(&self, new_addresses: Vec<String>) -> Vec<String> {
        let mut added_addresses = Vec::new();
        
        for chunk in new_addresses.chunks(self.batch_size) {
            let mut addresses = self.addresses.write().await;
            let mut recently_added = self.recently_added.write().await;

            for address in chunk {
                // Check limit
                if addresses.len() >= self.max_addresses {
                    warn!("Address limit reached, stopping batch addition");
                    break;
                }

                if !addresses.contains_key(address) {
                    let address_info = AddressInfo {
                        address: address.clone(),
                        user_id: None,
                        added_at: Utc::now(),
                        last_seen: None,
                        balance: 0,
                        is_active: true,
                    };

                    addresses.insert(address.clone(), address_info);
                    recently_added.insert(address.clone());
                    added_addresses.push(address.clone());
                }
            }
        }

        info!("Added {} new addresses to monitoring", added_addresses.len());
        added_addresses
    }

    /// Remove an address from monitoring
    pub async fn remove_address(&self, address: &str) -> bool {
        let mut addresses = self.addresses.write().await;
        let mut recently_added = self.recently_added.write().await;

        recently_added.remove(address);
        
        match addresses.remove(address) {
            Some(_) => {
                debug!("Removed address from monitoring: {}", address);
                true
            }
            None => {
                debug!("Address not found for removal: {}", address);
                false
            }
        }
    }

    /// Get all currently monitored addresses
    pub async fn get_all_addresses(&self) -> Vec<String> {
        let addresses = self.addresses.read().await;
        addresses.keys().cloned().collect()
    }

    /// Get recently added addresses (for subscription updates)
    pub async fn get_recently_added(&self) -> Vec<String> {
        let recently_added = self.recently_added.read().await;
        recently_added.iter().cloned().collect()
    }

    /// Clear recently added addresses (call after successful subscription update)
    pub async fn clear_recently_added(&self) {
        let mut recently_added = self.recently_added.write().await;
        let count = recently_added.len();
        recently_added.clear();
        debug!("Cleared {} recently added addresses", count);
    }

    /// Update address balance and activity
    pub async fn update_address_info(&self, address: &str, balance: i64) -> bool {
        let mut addresses = self.addresses.write().await;
        
        match addresses.get_mut(address) {
            Some(info) => {
                info.balance = balance;
                info.last_seen = Some(Utc::now());
                info.is_active = true;
                debug!("Updated balance for {}: {} lamports", address, balance);
                true
            }
            None => {
                debug!("Address not found for balance update: {}", address);
                false
            }
        }
    }

    /// Mark addresses as inactive if not seen recently
    pub async fn mark_inactive_addresses(&self, inactive_threshold: ChronoDuration) -> usize {
        let mut addresses = self.addresses.write().await;
        let cutoff = Utc::now() - inactive_threshold;
        let mut marked_inactive = 0;

        for (address, info) in addresses.iter_mut() {
            if info.is_active {
                let last_activity = info.last_seen.unwrap_or(info.added_at);
                if last_activity < cutoff {
                    info.is_active = false;
                    marked_inactive += 1;
                    debug!("Marked address as inactive: {}", address);
                }
            }
        }

        if marked_inactive > 0 {
            info!("Marked {} addresses as inactive", marked_inactive);
        }

        marked_inactive
    }

    /// Get address count
    pub async fn get_address_count(&self) -> usize {
        let addresses = self.addresses.read().await;
        addresses.len()
    }

    /// Get active address count
    pub async fn get_active_address_count(&self) -> usize {
        let addresses = self.addresses.read().await;
        addresses.iter().filter(|(_, info)| info.is_active).count()
    }

    /// Get subscription statistics
    pub async fn get_stats(&self) -> SubscriptionStats {
        let addresses = self.addresses.read().await;
        let now = Utc::now();
        
        let total_addresses = addresses.len();
        let active_addresses = addresses.iter().filter(|(_, info)| info.is_active).count();
        let inactive_addresses = total_addresses - active_addresses;
        let addresses_with_balance = addresses.iter().filter(|(_, info)| info.balance > 0).count();
        let total_balance = addresses.iter().map(|(_, info)| info.balance).sum();

        let mut newest_age = None;
        let mut oldest_age = None;

        if !addresses.is_empty() {
            let newest_time = addresses.iter().map(|(_, info)| info.added_at).max().unwrap();
            let oldest_time = addresses.iter().map(|(_, info)| info.added_at).min().unwrap();
            
            newest_age = Some(now - newest_time);
            oldest_age = Some(now - oldest_time);
        }

        SubscriptionStats {
            total_addresses,
            active_addresses,
            inactive_addresses,
            addresses_with_balance,
            total_balance,
            newest_address_age: newest_age,
            oldest_address_age: oldest_age,
        }
    }

    /// Check if address is being monitored
    pub async fn is_monitoring(&self, address: &str) -> bool {
        let addresses = self.addresses.read().await;
        addresses.contains_key(address)
    }

    /// Get addresses by user ID
    pub async fn get_user_addresses(&self, user_id: &uuid::Uuid) -> Vec<String> {
        let addresses = self.addresses.read().await;
        addresses
            .iter()
            .filter(|(_, info)| info.user_id.as_ref() == Some(user_id))
            .map(|(addr, _)| addr.clone())
            .collect()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}