use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use hyper::{Body, Method, Response};
use parking_lot::RwLock;

use crate::error::RouterError;
use crate::model::StatusResponse;

#[derive(Clone)]
pub struct ComponentRequest {
    http_verb: Method,
    query: String,
    body: String,
    user: String,
    repo: String,
    method: String,
}

impl ComponentRequest {
    pub fn new(
        http_verb: Method,
        query: String,
        body: String,
        user: String,
        repo: String,
        method: String,
    ) -> Self {
        Self {
            http_verb,
            query,
            body,
            user,
            repo,
            method,
        }
    }
}

#[derive(Debug)]
pub struct WorkerNode {
    url: String,
}

impl WorkerNode {
    pub fn new(url: String) -> Self {
        Self { url }
    }

    pub fn get_component_list(&self) -> Result<Vec<String>, RouterError> {
        let url = format!("{}/meta/status", self.url);
        let body = reqwest::get(&url)?.text()?;

        let response: StatusResponse = serde_json::from_str(&body)?;
        debug!(
            "Active components from worker response: {:?}",
            response.active_components
        );
        let mut component_list = Vec::new();

        for component in &response.active_components {
            let component_name = format!("{}/{}", component.id.path.user, component.id.path.repo);
            debug!("Adding new component name: {:?}", component_name);
            component_list.push(component_name);
        }

        debug!(
            "Final component list from get_component_list: {:?}",
            component_list
        );

        Ok(component_list)
    }
}

#[derive(Debug)]
pub struct RequestForwarder {
    workers: Vec<Arc<WorkerNode>>,
    components_map: RwLock<HashMap<String, Arc<WorkerNode>>>,
}

impl RequestForwarder {
    pub fn new() -> Self {
        let mut workers = Vec::new();
        let components_map = RwLock::new(HashMap::new());

        // If loading from the environment variable fails, there was a user error and we should
        // bail

        let worker_string = match env::var("V9_WORKERS") {
            Ok(value) => value,
            Err(e) => panic!("No V9_WORKERS env variable set: {:?}", e),
        };

        let env_workers: Vec<&str> = worker_string.split(';').collect();

        for worker in &env_workers {
            workers.push(Arc::new(WorkerNode::new(worker.to_string())));
        }

        let request_forwarder = Self {
            workers,
            components_map,
        };

        // Scan through server list to get initial active components
        if let Err(e) = request_forwarder.update_workers() {
            error!("Initial worker update failed: {:?}", e);
        }

        request_forwarder
    }

    fn update_workers(&self) -> Result<(), RouterError> {
        let mut locked_map = self.components_map.write();
        locked_map.clear();
        for worker in &self.workers {
            let component_list = worker.get_component_list()?;
            for component_name in &component_list {
                debug!("Adding component to map: {:?}", component_name);
                locked_map.insert(component_name.to_string(), Arc::clone(worker));
            }
        }

        Ok(())
    }

    fn find_appropriate_worker(&self, component_name: &str) -> Option<Arc<WorkerNode>> {
        // We want to keep the happy path free of lock contention, so we clone the Arc<WorkerNode>
        // (Otherwise we'd have to keep a read lock for the duration of each call to the component, which
        // could cause unnecessary contention for other threads trying to call `update_workers`)
        self.components_map.read().get(component_name).cloned()
    }

    fn send_request_to_worker(
        request: ComponentRequest,
        component_name: &str,
        worker_url: &str,
    ) -> Result<Response<Body>, RouterError> {
        let mut url = format!("{}/sl/{}/{}", worker_url, component_name, request.method);

        if !request.query.is_empty() {
            url = format!("{}?{}", url, request.query);
        }

        // TODO: This blocks the executor
        let client = reqwest::Client::new();
        let mut worker_resp = client
            .request(request.http_verb, &url)
            .body(request.body)
            .send()?;

        Ok(Response::builder()
            .status(worker_resp.status())
            .body(Body::from(worker_resp.text()?))
            .unwrap())
    }

    #[allow(clippy::single_match_else)]
    pub fn forward_request(&self, request: ComponentRequest) -> Result<Response<Body>, RouterError> {
        // NOTE: Several times here we drop the lock, which can lead to other threads modifying the
        // component map from under us. This doesn't cause correctness problems, but it is somewhat
        // sloppy. We should investigate better ways to do this in the future
        // (The obvious alternative is to take the lock for the duration of the forwarding, but then
        // we hold the lock across a call to a component -- which could take a long time!)

        let component_name = format!("{}/{}", request.user, request.repo);

        // First try and find an appropriate worker
        let mut worker_to_call_option = self.find_appropriate_worker(&component_name);

        if worker_to_call_option.is_none() {
            // If we didn't find a worker, update our worker index, then try again
            self.update_workers()?;
            worker_to_call_option = self.find_appropriate_worker(&component_name);
        }

        let worker_to_call = match worker_to_call_option {
            Some(worker) => worker,
            None => {
                // We still can't find a worker -- so there is probably no one running this component
                return Ok(Response::builder()
                    .status(404)
                    .body(Body::from("No worker running component"))
                    .unwrap());
            }
        };

        // Now we have a worker to call, so make the call
        let mut worker_resp =
            Self::send_request_to_worker(request.clone(), &component_name, &worker_to_call.url)?;

        // Now, if the worker 404s, it's possible that our component map is stale
        // TODO: Add a worker mechanism to distinguish missing components from 404s generated by working components
        if worker_resp.status() == 404 {
            // So let's update and try to find a better worker
            self.update_workers()?;

            // Since we updated, we need a new worker
            let worker_to_call = match self.find_appropriate_worker(&component_name) {
                Some(worker) => worker,
                None => {
                    // If we now lack any worker, don't bother retrying
                    return Ok(worker_resp);
                }
            };

            let backup_call =
                Self::send_request_to_worker(request, &component_name, &worker_to_call.url);

            match backup_call {
                Ok(resp) => {
                    // If the second call succeeded, then go with that, since its more up to date
                    worker_resp = resp;
                }
                Err(e) => {
                    // If it failed, log and move on -- we've done all we can
                    warn!("Retry call to serverless component failed {}", e);
                }
            }
        }

        Ok(worker_resp)
    }
}
