use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use http_cache_reqwest::{Cache, CacheMode, CACacheManager, HttpCache};

pub fn get_cached_http_client() -> ClientWithMiddleware {
  ClientBuilder::new(Client::new())
    .with(Cache(HttpCache {
      mode: CacheMode::Default,
      manager: CACacheManager::default(),
      options: None,
    }))
    .build()
}