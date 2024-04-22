use crate::core::{client::Client, env, macros};
use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, OnceLock, Weak},
};
use tokio::{runtime::RuntimeFlavor, sync::Mutex};

pub(crate) static CREATED_NETWORKS: OnceLock<Mutex<HashMap<String, Weak<Network>>>> =
    OnceLock::new();

fn created_networks() -> &'static Mutex<HashMap<String, Weak<Network>>> {
    CREATED_NETWORKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) struct Network {
    name: String,
    id: Option<String>,
    client: Arc<Client>,
}

impl Network {
    pub(crate) async fn new(name: impl Into<String>, client: Arc<Client>) -> Option<Arc<Self>> {
        let name = name.into();
        let mut guard = created_networks().lock().await;
        let network = if let Some(network) = guard.get(&name).and_then(Weak::upgrade) {
            network
        } else {
            if client.network_exists(&name).await {
                // Networks already exists and created outside the testcontainers
                return None;
            }

            let id = client.create_network(&name).await;

            let created = Arc::new(Self {
                name: name.clone(),
                id,
                client,
            });

            guard.insert(name, Arc::downgrade(&created));

            created
        };

        Some(network)
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        if self.client.config.command() == env::Command::Remove {
            let client = self.client.clone();
            let name = self.name.clone();

            let drop_task = async move {
                log::trace!("Drop was called for network {name}, cleaning up");
                let mut guard = created_networks().lock().await;

                // check the strong count under the lock to avoid any possible race-conditions.
                let is_network_in_use = guard
                    .get(&name)
                    .filter(|weak| weak.strong_count() > 0)
                    .is_some();

                if is_network_in_use {
                    log::trace!("Network {name} was not dropped because it is still in use");
                } else {
                    guard.remove(&name);
                    client.remove_network(&name).await;

                    log::trace!("Network {name} was successfully dropped");
                }
            };

            macros::block_on!(drop_task, "failed to remove network on drop");
        }
    }
}

impl fmt::Debug for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Network")
            .field("id", &self.id)
            .field("name", &self.name)
            .finish()
    }
}
