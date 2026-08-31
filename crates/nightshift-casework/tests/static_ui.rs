use std::{collections::BTreeMap, fs};

use nightshift_casework::{server::Api, static_ui::StaticUi};

fn fixture() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join(".vite")).unwrap();
    fs::create_dir(directory.path().join("assets")).unwrap();
    fs::write(
        directory.path().join("index.html"),
        br#"<!doctype html><script type="module" src="/assets/index-Ab12.js"></script>"#,
    )
    .unwrap();
    fs::write(
        directory.path().join(".vite/manifest.json"),
        br#"{"index.html":{"file":"assets/index-Ab12.js","css":["assets/index-Cd34.css"]}}"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("assets/index-Ab12.js"),
        b"export {};\n",
    )
    .unwrap();
    fs::write(directory.path().join("assets/index-Cd34.css"), b"body {}\n").unwrap();
    directory
}

#[test]
fn serves_only_preloaded_assets_and_declared_spa_routes() {
    let directory = fixture();
    fs::write(
        directory.path().join("assets/unlisted.js"),
        b"not reachable",
    )
    .unwrap();
    let api = Api::new(BTreeMap::new()).with_static_ui(StaticUi::load(directory.path()).unwrap());
    let run = "0".repeat(64);

    let index = api.response("GET", "/");
    assert_eq!(index.status, 200);
    assert_eq!(index.content_type, "text/html; charset=utf-8");
    assert_eq!(index.etag.as_deref().unwrap().len(), 66);
    assert_eq!(
        api.response("GET", "/assets/index-Ab12.js").content_type,
        "text/javascript; charset=utf-8"
    );
    assert_eq!(api.response("GET", &format!("/runs/{run}/raw")).status, 200);
    assert_eq!(api.response("GET", "/operational-conditions").status, 200);
    assert_eq!(
        api.response("GET", &format!("/operational-conditions/{run}"))
            .status,
        200
    );
    assert_eq!(
        api.response(
            "GET",
            &format!("/operational-conditions/{run}/questions/question%3Aone"),
        )
        .status,
        200
    );
    assert_eq!(
        api.response("GET", &format!("/operational-conditions/{run}/raw"))
            .status,
        200
    );

    for path in [
        "/assets/unlisted.js",
        "/assets/../index.html",
        "/../index.html",
        &format!("/runs/{run}/undeclared"),
        &format!("/operational-conditions/{run}/raw/monitor"),
        &format!("/operational-conditions/{run}/questions/one/extra"),
    ] {
        assert_eq!(api.response("GET", path).status, 404, "{path}");
    }
}

#[test]
fn write_methods_are_405_for_ui_and_assets() {
    let directory = fixture();
    let api = Api::new(BTreeMap::new()).with_static_ui(StaticUi::load(directory.path()).unwrap());
    let condition = format!("/operational-conditions/{}", "0".repeat(64));
    for method in ["POST", "PUT", "PATCH", "DELETE"] {
        assert_eq!(api.response(method, "/").status, 405);
        assert_eq!(api.response(method, "/assets/index-Ab12.js").status, 405);
        assert_eq!(api.response(method, &condition).status, 405);
    }
    assert_eq!(api.response("HEAD", &condition).status, 405);
}
