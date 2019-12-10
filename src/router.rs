use std::collections::HashMap;
use std::sync::Arc;

use hyper::{Body, Method, Response};
use parking_lot::RwLock;

use crate::error::RouterError;
use crate::model::StatusResponse;

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
        debug!("{:?}", response.active_components);
        let mut component_list = Vec::new();

        for component in &response.active_components {
            let component_name = format!("{}/{}", component.id.path.user, component.id.path.repo);
            debug!("{}", component_name);
            component_list.push(component_name);
        }

        debug!("{:?}", component_list);

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

        // TODO: load worker nodes from file into vector

        //Worker One
        workers.push(Arc::new(WorkerNode::new(String::from(
            "http://ec2-34-228-212-219.compute-1.amazonaws.com",
        ))));

        //Worker Two
        workers.push(Arc::new(WorkerNode::new(String::from(
            "http://ec2-54-211-200-158.compute-1.amazonaws.com",
        ))));

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

    pub fn update_workers(&self) -> Result<(), RouterError> {
        self.components_map.write().clear();
        for worker in &self.workers {
            let component_list = worker.get_component_list()?;
            for component_name in &component_list {
                debug!("{}", component_name);
                self.components_map
                    .write()
                    .insert(component_name.to_string(), Arc::clone(worker));
            }
        }

        Ok(())
    }

    pub fn forward_request(&self, request: ComponentRequest) -> Result<Response<Body>, RouterError> {
        let component_name = format!("{}/{}", request.user, request.repo);
        if !self.components_map.read().contains_key(&component_name) {
            self.update_workers()?;
        }

        debug!("Forwarding request to: {:?}", component_name);

        if !self.components_map.read().contains_key(&component_name) {
            let body = Body::from("Requested component not found\n");
            warn!("Component to forward to not found: {:?}", component_name);
            return Ok(Response::builder().status(404).body(body).unwrap());
        }

        let mut url = format!(
            "{}/sl/{}/{}",
            self.components_map.read().get(&component_name).unwrap().url,
            component_name,
            request.method
        );

        if !request.query.is_empty() {
            url = format!("{}?{}", url, request.query);
        }

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
}
