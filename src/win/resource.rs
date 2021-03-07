use gio::{resources_register, Resource};
use glib::Bytes;
use std::borrow::Cow;
use std::sync::Once;

static RESOURCE_PATH_PREFIX: &str = "/io/github/cat-in-136/gtk3-basic-bulk-renamer/";

static RESOURCE_INIT: Once = Once::new();

pub(super) fn init_resource() {
    RESOURCE_INIT.call_once(|| {
        let resource_bytes = Bytes::from_static(include_bytes!("resource.gresource"));
        let res = Resource::from_data(&resource_bytes).unwrap();
        resources_register(&res);
    });
}

pub(super) fn resource_path(path: &str) -> Cow<str> {
    if cfg!(test) {
        init_resource();
    }
    Cow::Owned([RESOURCE_PATH_PREFIX, path].concat())
}

#[test]
fn test_resource_path() {
    assert_eq!(
        resource_path("test.xml"),
        "/io/github/cat-in-136/gtk3-basic-bulk-renamer/test.xml"
    );
}
