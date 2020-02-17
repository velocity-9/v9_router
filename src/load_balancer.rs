use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use parking_lot::RwLock;

use crate::error::RouterError;
use crate::model::ComponentPath;
use crate::worker::WorkerNode;

// This is sensitive to how quickly the deployment manager makes changes
const MAP_UPDATE_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct WorkerLoadBalancer {
    workers: Vec<Arc<WorkerNode>>,
    component_map: RwLock<ComponentMap>,
}

#[derive(Debug, Default)]
struct ComponentMap {
    seq_num: u64,
    map: HashMap<ComponentPath, LoadBalancingData>,
}

#[derive(Debug, Default)]
struct LoadBalancingData {
    counter: AtomicUsize,
    workers: Vec<Arc<WorkerNode>>,
}

impl WorkerLoadBalancer {
    pub fn new(workers: Vec<WorkerNode>) -> Arc<WorkerLoadBalancer> {
        let load_balancer = Arc::new(WorkerLoadBalancer {
            workers: workers.into_iter().map(Arc::new).collect(),
            component_map: RwLock::new(ComponentMap::default()),
        });

        if let Err(e) = load_balancer.update_component_map() {
            warn!("Initial load balancer update failed: {}", e);
        }

        // This is the background updater thread
        let background_handle = Arc::downgrade(&load_balancer);
        thread::spawn(move || {
            while let Some(load_balancer) = background_handle.upgrade() {
                thread::sleep(MAP_UPDATE_DELAY);

                if let Err(e) = load_balancer.update_component_map() {
                    warn!("Load balancer update failed: {}", e);
                }
            }
        });

        load_balancer
    }

    fn update_component_map(&self) -> Result<(), RouterError> {
        // We measure a `seq_num` so we don't f****** smoke someone else's update
        let seq_num = self.component_map.read().seq_num;

        // Create a new map to replace the old one
        let mut new_map: HashMap<ComponentPath, LoadBalancingData> = HashMap::new();
        for worker in &self.workers {
            // TODO: Add partial updates if one of these requests fail
            let components_on_worker = worker.get_component_list()?;

            for component in components_on_worker {
                let map_entry = new_map.entry(component);
                let balancing_data = map_entry.or_default();
                balancing_data.workers.push(worker.clone());
            }
        }

        let mut component_map = self.component_map.write();

        // If the sequence numbers don't match up than we lost an update race, we should forget about updating the table
        // (they did the work at the same time nessesarily, so our data is not "more fresh")

        // So only update if they do match
        if component_map.seq_num == seq_num {
            component_map.seq_num += 1;
            component_map.map = new_map
        }

        Ok(())
    }

    pub fn get_worker_found_stale_data(
        &self,
        path: &ComponentPath,
    ) -> Result<Option<Arc<WorkerNode>>, RouterError> {
        self.update_component_map()?;

        Ok(self.get_worker(path))
    }

    pub fn get_worker(&self, path: &ComponentPath) -> Option<Arc<WorkerNode>> {
        let component_map = self.component_map.read();

        // We update every 5 seconds, and literally missing data should only happen in a few cases
        // 1) Initial deployment hasn't finished yet (nothing to be done)
        // 2) Initial deployment is done but we haven't picked it up yet (simply a five second delay to the user)
        // 3) There is no instance up due to bad deployment manager code (this is a DM bug)
        // 4) A bug somewhere else (nothing to be done)
        // None of these cases is worth a retry
        component_map.map.get(path).and_then(|load_balancing_data| {
            let idx = load_balancing_data.counter.fetch_add(1, Ordering::SeqCst);
            if load_balancing_data.workers.is_empty() {
                None
            } else {
                Some(load_balancing_data.workers[idx % load_balancing_data.workers.len()].clone())
            }
        })
    }
}
