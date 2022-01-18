use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::convert::TryInto;

pub type StatusCode = u16;
pub type Url = String;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Response {
    status: StatusCode,
    body: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Method {
    GET,
    POST,
}

impl Default for Method {
    fn default() -> Self {
        Method::GET
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct Request {
    method: Method,
    url: Url,
    body: String,
    headers: HashMap<String, String>,
}

#[async_trait]
pub trait HttpClient {
    async fn send(self, request: Request) -> Result<Response>;
}

pub struct MockClient {
    responses: HashMap<(Method, Url), Response>,
}

impl MockClient {
    pub fn new(responses: HashMap<(Method, Url), Response>) -> Self {
        MockClient { responses }
    }
}

#[async_trait]
impl HttpClient for MockClient {
    async fn send(self, request: Request) -> Result<Response> {
        Ok(self
            .responses
            .get(&(request.method, request.url))
            .expect("Mock does not contain response of requested url.")
            .clone())
    }
}

struct Client {
    client: reqwest::Client,
}

impl Client {
    pub fn new() -> Self {
        Client {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl HttpClient for Client {
    async fn send(self, request: Request) -> Result<Response> {
        let req = build_request(&self.client, request)?;
        let resp = self.client.execute(req).await?;
        Ok(Response {
            status: resp.status().as_u16(),
            body: resp.text().await?,
        })
    }
}

fn build_request(client: &reqwest::Client, request: Request) -> Result<reqwest::Request> {
    let method = match request.method {
        Method::GET => reqwest::Method::GET,
        Method::POST => reqwest::Method::POST,
    };
    Ok(client
        .request(method, request.url)
        .headers((&request.headers).try_into()?)
        .body(request.body)
        .build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn mock_client_should_return_given_response() {
        let expected_resp = Response {
            status: 200,
            body: "".to_string(),
        };
        let responses = HashMap::from([(
            (Method::GET, "http://somesite.com".into()),
            expected_resp.clone(),
        )]);
        let client = MockClient::new(responses);
        let actual_resp = client
            .send(Request {
                method: Method::GET,
                url: "http://somesite.com".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(expected_resp, actual_resp)
    }

    #[async_std::test]
    async fn mock_client_should_return_different_response() {
        let expected_resp = Response {
            status: 404,
            body: "".to_string(),
        };
        let responses = HashMap::from([(
            (Method::POST, "http://somedifferentsite.com".into()),
            expected_resp.clone(),
        )]);
        let client = MockClient::new(responses);
        let actual_resp = client
            .send(Request {
                method: Method::POST,
                url: "http://somedifferentsite.com".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(expected_resp, actual_resp)
    }

    #[async_std::test]
    #[should_panic]
    async fn mock_client_should_panic_if_mock_has_wrong_method() {
        let responses = HashMap::from([(
            (Method::POST, "http://somesite.com".into()),
            Response {
                status: 404,
                body: "".to_string(),
            },
        )]);
        let client = MockClient::new(responses);
        client
            .send(Request {
                method: Method::GET,
                url: "http://somesite.com".into(),
                ..Default::default()
            })
            .await
            .unwrap();
    }

    #[async_std::test]
    #[should_panic]
    async fn mock_client_should_panic_if_mock_has_wrong_url() {
        let responses = HashMap::from([(
            (Method::GET, "http://somesite.com".into()),
            Response {
                status: 404,
                body: "".to_string(),
            },
        )]);
        let client = MockClient::new(responses);
        client
            .send(Request {
                method: Method::GET,
                url: "http://someothersite.com".into(),
                ..Default::default()
            })
            .await
            .unwrap();
    }

    #[test]
    fn build_request_should_build_correct_url() {
        let client = reqwest::Client::new();
        let request = build_request(
            &client,
            Request {
                url: "http://somesite.com".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!("http://somesite.com/", request.url().to_string());
    }

    #[test]
    fn build_request_should_build_different_url() {
        let client = reqwest::Client::new();
        let request = build_request(
            &client,
            Request {
                url: "http://somedifferentsite.com".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!("http://somedifferentsite.com/", request.url().to_string());
    }

    #[test]
    fn build_request_should_transform_method_to_reqwest_method() {
        let client = reqwest::Client::new();
        let request = build_request(
            &client,
            Request {
                url: "http://somesite.com".to_string(),
                method: Method::GET,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reqwest::Method::GET, request.method());
    }

    #[test]
    fn build_request_should_transform_different_method_to_reqwest_method() {
        let client = reqwest::Client::new();
        let request = build_request(
            &client,
            Request {
                url: "http://somesite.com".to_string(),
                method: Method::POST,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(reqwest::Method::POST, request.method());
    }

    #[test]
    fn build_request_should_build_correct_body() {
        let client = reqwest::Client::new();
        let request = build_request(
            &client,
            Request {
                url: "http://somesite.com".to_string(),
                body: "some body".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(b"some body", request.body().unwrap().as_bytes().unwrap());
    }

    #[test]
    fn build_request_should_build_different_body() {
        let client = reqwest::Client::new();
        let request = build_request(
            &client,
            Request {
                url: "http://somesite.com".to_string(),
                body: "some different body".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            b"some different body",
            request.body().unwrap().as_bytes().unwrap()
        );
    }

    #[test]
    fn build_request_should_build_correct_headers() {
        let client = reqwest::Client::new();
        let request = build_request(
            &client,
            Request {
                url: "http://somesite.com".to_string(),
                headers: HashMap::from([(
                    "content-type".to_string(),
                    "application/json".to_string(),
                )]),
                ..Default::default()
            },
        )
        .unwrap();

        let mut header_map = reqwest::header::HeaderMap::new();
        header_map.insert(
            "content-type",
            reqwest::header::HeaderValue::from_str("application/json").unwrap(),
        );
        assert_eq!(&header_map, request.headers());
    }

    #[test]
    fn build_request_should_build_different_headers() {
        let client = reqwest::Client::new();
        let request = build_request(
            &client,
            Request {
                url: "http://somesite.com".to_string(),
                headers: HashMap::from([("accept-encoding".to_string(), "gzip".to_string())]),
                ..Default::default()
            },
        )
        .unwrap();

        let mut header_map = reqwest::header::HeaderMap::new();
        header_map.insert(
            "accept-encoding",
            reqwest::header::HeaderValue::from_str("gzip").unwrap(),
        );
        assert_eq!(&header_map, request.headers());
    }

    #[async_std::test]
    #[ignore]
    async fn real_client_should_successfully_send_request() {
        let client = Client::new();
        let actual_resp = client
            .send(Request {
                url: "https://google.com".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(actual_resp.status, 200);
        assert!(!actual_resp.body.is_empty())
    }
}
