use std::collections::HashMap;
use hyper::{Body, Method};
use std::sync::Arc;
use crate::model::StatusResponse;
use crate::error::RouterError;

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
    components: Vec<String>,
    is_active: bool,
}

impl WorkerNode {
    pub fn new(url: String) -> Self {
        Self {
            url,
            components: Vec::new(),
            is_active: false,
        } 
    }

    pub fn get_component_list(&self) -> Result<Vec<String>, RouterError> {
        let url = format!("{}/meta/status", self.url);
        let body = reqwest::get(&url)?.text()?;

        let response: StatusResponse = serde_json::from_str(&body)?;
        let mut component_list = Vec::new();       
        
        for component in response.active_components.iter() {
            let component_name = format!("{}/{}", component.id.path.user, component.id.path.repo);
            component_list.push(component_name);
        }

        Ok(component_list)
    }
}

#[derive(Debug)]
pub struct RequestForwarder {
    workers: Vec<Arc<WorkerNode>>,
    components_map: HashMap<String, Arc<WorkerNode>>
}

impl RequestForwarder {
    pub fn new() -> Result<Self, RouterError> {
        let mut workers = Vec::new();
        let mut components_map: HashMap<String, Arc<WorkerNode>> = HashMap::new();

        // TODO: load worker nodes from file into vector
        workers.push(Arc::new(WorkerNode::new(String::from("localhost"))));

        // - scan through server list to get active components
        for worker in workers.iter() {
            let component_list = worker.get_component_list()?;
            for component_name in component_list.iter() {
                components_map.insert(component_name.to_string(), Arc::clone(worker));
            }
        }

        Ok(
            Self {
                workers,
                components_map,
            }
        )
    }

    pub fn forward_request(&self, request: ComponentRequest) -> Body {
        Body::from("Hello!")
    }
}
