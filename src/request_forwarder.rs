use std::env;
use std::sync::Arc;

use hyper::{Body, Method, Response, StatusCode};

use crate::error::RouterError;
use crate::load_balancer::WorkerLoadBalancer;
use crate::model::ComponentPath;
use crate::worker::WorkerNode;

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
pub struct RequestForwarder {
    load_balancer: Arc<WorkerLoadBalancer>,
}

impl RequestForwarder {
    pub fn new() -> Self {
        // If loading from the environment variable fails, there was a user error and we should bail
        // TODO: Get this from dependency injection
        let worker_string = match env::var("V9_WORKERS") {
            Ok(value) => value,
            Err(e) => panic!("No V9_WORKERS env variable set: {:?}", e),
        };

        let workers = worker_string
            .split(';')
            .map(|worker_url| WorkerNode::new(worker_url.to_string()))
            .collect();

        Self {
            load_balancer: WorkerLoadBalancer::new(workers),
        }
    }

    fn send_request_to_worker(
        request: ComponentRequest,
        worker_url: &str,
    ) -> Result<(StatusCode, String), RouterError> {
        let mut url = format!(
            "{}/sl/{}/{}/{}",
            worker_url, request.user, request.repo, request.method
        );

        if !request.query.is_empty() {
            url = format!("{}?{}", url, request.query);
        }

        // TODO: This blocks the executor, so we probably should do something smarter than just blocking
        let client = reqwest::Client::new();
        let mut worker_resp = client
            .request(request.http_verb, &url)
            .body(request.body)
            .send()?;

        Ok((worker_resp.status(), worker_resp.text()?))
    }

    #[allow(clippy::single_match_else)]
    pub fn forward_request(&self, request: ComponentRequest) -> Result<Response<Body>, RouterError> {
        let path = ComponentPath {
            user: request.user.clone(),
            repo: request.repo.clone(),
        };

        let worker = match self.load_balancer.get_worker(&path) {
            Some(worker) => worker,
            None => return Err(RouterError::PathNotFound(format!("no such component: {}", path))),
        };

        // First attempt naively
        let (mut code, mut text) = Self::send_request_to_worker(request.clone(), worker.request_url())?;

        // If we detect stale data
        if code == StatusCode::from_u16(404).unwrap() && text.starts_with("v9: worker 404") {
            // Then retry if we can find a new worker
            if let Ok(Some(worker)) = self.load_balancer.get_worker_found_stale_data(&path) {
                let second_req = Self::send_request_to_worker(request, worker.request_url())?;
                code = second_req.0;
                text = second_req.1;
            }
        }

        Ok(Response::builder().status(code).body(Body::from(text)).unwrap())
    }
}
