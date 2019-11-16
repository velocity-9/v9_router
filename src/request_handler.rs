use std::str;
use std::sync::Arc;

use hyper::rt::{Future, Stream};
use hyper::{Body, Method, Request, Response, Uri};

use crate::error::RouterError;
use crate::router::ComponentRequest;

// Warning: This method is somewhat complicated, since it needs to deal with async stuff
// TODO: Consider making this a method on a struct somewhere
// TODO: Deal with panics bubbling up to this level
pub fn global_request_entrypoint(
    handler: Arc<HttpRequestHandler>,
    req: Request<Body>,
) -> impl Future<Item = Response<Body>, Error = hyper::error::Error> + Send {
    debug!("{:?}", req);

    // Pull the verb, uri, and query stuff out of the request
    // (It's okay to do this, since it's all quite quick to execute)
    let http_verb = req.method().clone();
    let uri = req.uri().clone();
    let query = uri.query().unwrap_or("").to_string();

    // Then get a future representing the body (this is a future, since hyper may not of received the whole body yet)
    let body_future = req.into_body().concat2().map(|c| {
        // Convert the Chunk into a rust "String", wrapping any error in our error type
        str::from_utf8(&c)
            .map(str::to_owned)
            .map_err(RouterError::from)
    });

    // Next we want to an operation on the body. This needs to happen in a future for two reasons
    // 1) We want to handle many requests at once, so we don't want to block a thread
    // 2) Hyper literally doesn't let you deal with the body unless you're inside a future context (there is no API to escape this)
    // Note: We already have a result (body_result) here, since we might get an Utf8 decode error above
    body_future.map(move |body_result| {
        debug!("body = {:?}", body_result);

        let resp: Response<Body> = body_result
            // Delegate to the handler to actually deal with this request
            .and_then(|body| handler.handle(http_verb, uri, query, body))
            .unwrap_or_else(|e| {
                warn!("Forced to convert error {:?} into a http response", e);
                e.into()
            });

        if resp.status() == 532 {
            error!("INTERNAL ROUTER ERROR -- {:?}", resp);
        } else {
            debug!("{:?}", resp);
        }

        resp
    })
}

#[derive(Debug)]
pub struct HttpRequestHandler {
    test_output: String,
}

impl HttpRequestHandler {
    pub fn new() -> Self {
        Self {
            test_output: String::from("this is a test"),
        }
    }

    fn handle(
        &self,
        http_verb: Method,
        uri: Uri,
        query: String,
        body: String,
    ) -> Result<Response<Body>, RouterError> {
        // Get the uri path, and then split it around slashes into components
        // Note: All URIs start with a slash, so we skip the first entry in the split (which is always just "")
        let path_components: Vec<&str> = uri.path().split('/').skip(1).collect();

        let user = path_components[1].to_string();
        let repo = path_components[2].to_string();
        let method = path_components[3].to_string();

        let request = ComponentRequest::new(http_verb, query, body, user, repo, method);
        let result_body = request.forward_component_request();

        //TODO: This should return the actual response from the server
        Ok(Response::builder().status(200).body(result_body).unwrap())
    }
}
