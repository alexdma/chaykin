/**
 * Utility functions.
 */
use gemini::request::{Gemini, GeminiRequest, InvalidRequest, Request};
use regex::Regex;
use std::borrow::Cow;

fn uri_converter<'a>(uri: &'a String, re: &str, proto: &'a str) -> Cow<'a, str> {
    let re = Regex::new(re).unwrap();
    let newuri = re.replace(uri, format!("{proto}://"));
    newuri
}

pub fn to_gemini_uri(http_uri: &String) -> String {
    let gemurl = uri_converter(http_uri, r"http(s)?://", "gemini");
    gemurl.into_owned()
}

pub fn to_http_uri(gem_uri: &String) -> String {
    let httpurl = uri_converter(gem_uri, r"gemini://", "http");
    httpurl.into_owned()
}

/// Tries to create a Gemini request from the supplied string
pub fn client(uri: &str) -> Result<GeminiRequest, InvalidRequest> {
    let req = Request::<Gemini>::from_uri(uri).expect("Could not get a Gemini request!");
    req.into_gemini_request()
}

#[test]
fn test_url_convert() {
    const GEMURL_HTTP: &str = "http://gemini.circumlunar.space";
    let gemurl_gemini = to_gemini_uri(&String::from(GEMURL_HTTP));
    let req = client(gemurl_gemini.as_str());
    assert!(req
        .expect("Could not get a Gemini request!")
        .is_gemini_request());
}
