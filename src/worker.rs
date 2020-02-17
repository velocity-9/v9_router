use std::time::Duration;

use reqwest::Client;

use crate::error::RouterError;
use crate::model::{ComponentPath, StatusResponse};

const WORKER_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub struct WorkerNode {
    client: Client,
    url: String,
}

impl WorkerNode {
    pub fn new(url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(WORKER_TIMEOUT)
            .build()
            .unwrap();

        Self { client, url }
    }

    pub fn get_component_list(&self) -> Result<Vec<ComponentPath>, RouterError> {
        let url = format!("{}/meta/status", self.url);

        let body = self.client.get(&url).send()?.text()?;

        let response: StatusResponse = serde_json::from_str(&body)?;
        debug!(
            "Active components from worker response: {:?}",
            response.active_components
        );
        let mut component_list = Vec::new();

        for component in &response.active_components {
            debug!("Adding new component name: {:?}", component.id.path);
            component_list.push(component.id.path.clone());
        }

        debug!(
            "Final component list from get_component_list: {:?}",
            component_list
        );

        Ok(component_list)
    }

    pub fn request_url(&self) -> &str {
        &self.url
    }
}
