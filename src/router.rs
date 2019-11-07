use hyper::{Body, Method};

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

    pub fn forward_component_request(&self) -> Body {
        debug!("{}", &self.http_verb);
        debug!("{}", &self.query);
        debug!("{}", &self.body);
        debug!("{}", &self.user);
        debug!("{}", &self.repo);
        debug!("{}", &self.method);
        Body::from("Hello!")
    }
}
