use std::{collections::BTreeMap, sync::LazyLock};

#[cfg(feature = "router")]
use topcoat_core::context::Cx;
use topcoat_core::fnv1a::Fnv1a;
#[cfg(feature = "router")]
use topcoat_router::{
    Body, HeaderValue, Method, Methods, Path, PathBuf, Route, RouteFuture, RouteId,
    header::{CACHE_CONTROL, CONTENT_TYPE},
    response::Response,
};

use crate::JsTemplate;

const EXPORT: &str = "export default ";
const END: &str = ";\n";

/// A JavaScript module that defines [`JsTemplate`]s for the browser runtime.
///
/// Its default export maps each template's id to its function. The browser
/// runtime loads the module once and resolves every [`Js`](crate::Js) it
/// finds in the page against it, so a page needs no executable source.
#[derive(Debug)]
pub struct JsModule {
    templates: Vec<&'static JsTemplate>,
    source: String,
    path: String,
}

impl JsModule {
    /// Collects `templates` into a module, including each template once.
    ///
    /// # Panics
    ///
    /// Panics if two templates with different sources share an id.
    #[must_use]
    pub fn new(templates: impl IntoIterator<Item = &'static JsTemplate>) -> Self {
        let mut unique = BTreeMap::new();
        for template in templates {
            let first = *unique.entry(template.id()).or_insert(template);
            assert!(
                first.source() == template.source(),
                "runtime expression templates share the id {}:\n{}\n{}",
                template.id(),
                first.source(),
                template.source(),
            );
        }
        let templates: Vec<_> = unique.into_values().collect();

        let mut source = String::from(EXPORT);
        source.push_str("{\n");
        for template in &templates {
            source.push('"');
            source.push_str(template.id());
            source.push_str("\": ");
            source.push_str(template.source());
            source.push_str(",\n");
        }
        source.push('}');
        source.push_str(END);

        let path = format!(
            "/_topcoat/runtime/expressions-{:016x}.js",
            Fnv1a::<u64>::new().write(source.as_bytes()).finish(),
        );
        Self {
            templates,
            source,
            path,
        }
    }

    /// The module of every template registered in this binary.
    ///
    /// Runtime expressions register their templates when they are compiled.
    #[must_use]
    pub fn registered() -> &'static Self {
        static MODULE: LazyLock<JsModule> =
            LazyLock::new(|| JsModule::new(inventory::iter::<&'static JsTemplate>().copied()));
        &MODULE
    }

    /// The templates the module defines, ordered by id.
    #[must_use]
    pub fn templates(&self) -> &[&'static JsTemplate] {
        &self.templates
    }

    /// The module's JavaScript source.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The object literal the module exports, as a JavaScript expression.
    #[must_use]
    pub fn definitions(&self) -> &str {
        &self.source[EXPORT.len()..self.source.len() - END.len()]
    }

    /// The URL path [`runtime`](crate::RouterBuilderRuntimeExt::runtime)
    /// serves the module at.
    ///
    /// The path contains a hash of the module's source, so its contents never
    /// change for a given path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// Serves a [`JsModule`] at its [`path`](JsModule::path).
#[cfg(feature = "router")]
pub(crate) struct JsModuleRoute {
    id: RouteId,
    path: PathBuf,
    module: &'static JsModule,
}

#[cfg(feature = "router")]
impl JsModuleRoute {
    pub(crate) fn new(module: &'static JsModule) -> Self {
        Self {
            id: RouteId::new(),
            path: Path::new(module.path()).to_owned(),
            module,
        }
    }
}

#[cfg(feature = "router")]
impl Route for JsModuleRoute {
    fn id(&self) -> RouteId {
        self.id
    }

    fn methods(&self) -> Methods<'_> {
        Methods::Only(&[Method::GET])
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, _cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
        Box::pin(async {
            let mut response = Response::new(Body::from(self.module.source()));
            let headers = response.headers_mut();
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("text/javascript; charset=utf-8"),
            );
            headers.insert(
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            Ok(response)
        })
    }
}
