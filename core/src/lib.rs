#![cfg_attr(feature = "tagged-result", feature(auto_traits, negative_impls))]

use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    path::Path,
};

pub use inventory;
pub use once_cell;

#[cfg(feature = "axum-router")]
pub use axum_handler::{Axum, HandlerAxum};
use ts_rs::TS;

const TS_REQUEST: &str = include_str!("./ts/request.ts");

pub struct Api {
    pub registered_fn_names: HashSet<&'static str>,
    #[cfg(feature = "axum-router")]
    pub axum_router: Option<axum::Router>,
}

impl Api {
    pub fn new() -> Self {
        Self {
            registered_fn_names: HashSet::new(),
            #[cfg(feature = "axum-router")]
            axum_router: Some(axum::Router::new()),
        }
    }

    /// Exports a typescript client to the given file path.
    ///
    /// Only registered functions are exported and if a registered function does not use
    /// `#[ts_export]` this function panics. If you want to modify this behavior use `export_ts_client_choice`.
    pub fn export_ts_client(
        &self,
        server_url: impl AsRef<str>,
        file_path: impl AsRef<Path>,
    ) -> std::io::Result<()> {
        self.export_ts_client_choice(server_url, file_path, true, true)
    }

    /// Exports a typescript client to the given file path.
    /// If `export_only_registered` is `true` all functions, wether registered or not,
    /// will be exported.
    pub fn export_ts_client_choice(
        &self,
        server_url: impl AsRef<str>,
        file_path: impl AsRef<Path>,
        export_only_registered: bool,
        registered_must_be_exported: bool,
    ) -> std::io::Result<()> {
        // path without file
        if let Some(parent) = file_path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut function_definitions = String::new();

        // For detecting duplicate function names, declaring exports
        let mut fn_names = HashSet::new();

        for ts_fn in inventory::iter::<LazyTsFn>().map(|f| f.0.deref()) {
            if export_only_registered && !self.registered_fn_names.contains(&ts_fn.name) {
                continue;
            }

            // Todo: All registered functions are guaranteed to have a unique name, but if `export_only_registered` is false
            // there may be a duplicate name that is being exported here.
            if fn_names.contains(ts_fn.name) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!("Duplicate function name: {}. Each exported function must have a unique name.", ts_fn.name),
                ));
            } else {
                fn_names.insert(ts_fn.name);
            }

            let fn_name = &ts_fn.name;
            let type_declarations = ts_fn
                .type_declarations
                .values()
                .map(|t| format!("export {t}"))
                .collect::<Vec<_>>()
                .join("\n");
            let params = ts_fn
                .request_types
                .iter()
                .map(|t| format!("{}: {}", t.0, prefix_type(fn_name, &t.1)))
                .collect::<Vec<_>>()
                .join(", ");
            let param_names = &ts_fn
                .request_types
                .iter()
                .map(|t| t.0)
                .collect::<Vec<_>>()
                .join(", ");
            let response_type = prefix_type(fn_name, &ts_fn.response_type);
            let server_url = server_url.as_ref();

            function_definitions += &format!(
                r#"
function {fn_name}({params}): __request.CancelablePromise<{response_type}> {{
    return __request.request(
        {{ url: '{server_url}' }},
        {{
            method: 'POST',
            url: '/{fn_name}',
            body: [{param_names}],
            mediaType: 'application/json',
        }}
    )
}}
namespace {fn_name} {{
    {type_declarations}
}}
"#
            );
        }

        // Todo: This does not quite work yet, because there may be a registered function with the same
        // name as a not registered function that is being exported. Possibly capture the full function paths instead of just the name.
        if registered_must_be_exported {
            for registered in self.registered_fn_names.iter() {
                if !fn_names.contains(registered) {
                    panic!(
                        "Function `{}` is registered but not exported. \
                        If you want to allow registered functions to not be exported, use `Api::export_ts_client_choice` with `export_only_registered` set to `false`.",
                        registered
                    );
                }
            }
        }

        let exports = format!(
            "export {{\n  {}\n}}",
            fn_names.into_iter().collect::<Vec<_>>().join(",\n  ")
        );

        let content = format!(
            "{}\n{}\nnamespace __request {{\n{}\n}}",
            exports, function_definitions, TS_REQUEST
        );
        std::fs::write(file_path, content)?;
        Ok(())
    }

    #[cfg(feature = "axum-router")]
    #[allow(clippy::extra_unused_type_parameters)]
    pub fn register_axum<Request, Response, External, F>(mut self, handler: F) -> Self
    where
        HandlerAxum<Request, Response, External, F>: ApiFn,
    {
        let fn_name = function_name(&handler);
        if self.registered_fn_names.contains(fn_name) {
            panic!(
                "Function name already registered: `{}`. Each function must have a unique name, since they are all exported from one file in typescript.",
                fn_name
            );
        }
        self.registered_fn_names.insert(fn_name);
        ApiFn::register(
            HandlerAxum {
                f: handler,
                _marker: std::marker::PhantomData,
            },
            &mut self,
        );
        self
    }

    #[cfg(feature = "axum-router")]
    pub fn axum_router(&self) -> axum::Router {
        self.axum_router.clone().unwrap()
    }
}

/// s must start with `open`
fn find_matching_delimiter(s: &str, open: char, close: char) -> Option<usize> {
    let mut open_count = 0;
    for (i, c) in s.chars().enumerate() {
        if c == open {
            open_count += 1;
        } else if c == close {
            open_count -= 1;
        }
        if open_count == 0 {
            return Some(i);
        }
    }
    None
}

fn prefix_type(prefix: &str, mut t: &str) -> String {
    fn prefix_type_inner<'a>(prefix: &str, t: &'a str) -> (String, &'a str) {
        let t = t.trim();

        let delimiters = match t.chars().next() {
            Some('[') => Some(('[', ']')),
            Some('<') => Some(('<', '>')),
            _ => None,
        };

        if let Some(delimiters) = delimiters {
            let end = find_matching_delimiter(t, delimiters.0, delimiters.1).unwrap();

            let mut entries = Vec::new();
            let mut inner = &t[1..end];
            while !inner.is_empty() {
                let (entry, next) = prefix_type_inner(prefix, inner);
                entries.push(entry);
                inner = next.trim_start_matches(',').trim();
            }

            (
                format!("{}{}{}", delimiters.0, entries.join(", "), delimiters.1),
                &t[end + 1..],
            )
        } else {
            // we have a type ident
            let end = t
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(t.len());
            let ty = &t[..end];

            let ty = if !is_ts_intrinstic_type(ty) {
                format!("{}.{}", prefix, ty)
            } else {
                ty.to_string()
            };

            // check if we have generics and if so parse them
            if let Some(next) = t[end..].chars().next() {
                if next == '<' {
                    let (generics, next) = prefix_type_inner(prefix, &t[end..]);
                    return (format!("{}{}", ty, generics), next);
                }
            }

            (ty, &t[end..])
        }
    }

    let mut result = String::new();
    while !t.is_empty() {
        let (entry, next) = prefix_type_inner(prefix, t);
        result.push_str(&entry);
        t = next;
    }
    result
}

fn is_ts_intrinstic_type(t: &str) -> bool {
    matches!(
        t,
        "string" | "number" | "boolean" | "any" | "void" | "never" | "unknown" | "null" | "Array"
    )
}

#[test]
fn test_prefix_type() {
    assert_eq!(prefix_type("foo", "bar"), "foo.bar");
    assert_eq!(prefix_type("foo", "string"), "string");
    assert_eq!(prefix_type("foo", "bar<baz>"), "foo.bar<foo.baz>");
    assert_eq!(
        prefix_type("foo", "bar<baz, qux>"),
        "foo.bar<foo.baz, foo.qux>"
    );
    assert_eq!(
        prefix_type("foo", "bar<baz, qux<quux<number, number>>>"),
        "foo.bar<foo.baz, foo.qux<foo.quux<number, number>>>"
    );
    assert_eq!(
        prefix_type("foo", "[bar<barr>, Array<bazz>]"),
        "[foo.bar<foo.barr>, Array<foo.bazz>]"
    );
    assert_eq!(
        prefix_type("list_products", "Array<Product>"),
        "Array<list_products.Product>"
    );
}

#[test]
fn test_void() {
    println!(
        "{:?}",
        prefix_type("a", "Result<[Array<SignUp>, Array<SignUp>], string>")
    );
}

impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ApiFn {
    fn register(self, api: &mut Api);
}

#[cfg(feature = "axum-router")]
pub mod axum_handler {
    use super::{function_name, Api, ApiFn};
    use axum::extract::{FromRequest, FromRequestParts, Json};
    use axum::response::IntoResponse;
    use serde::{de::DeserializeOwned, Serialize};

    #[derive(Clone, Debug)]
    pub struct Axum<T>(pub T);

    #[derive(Debug)]
    pub struct HandlerAxum<Request, Response, External, F> {
        pub f: F,
        pub _marker: std::marker::PhantomData<(Request, Response, External)>,
    }

    impl<Request, Response, External, F: Clone> Clone for HandlerAxum<Request, Response, External, F> {
        fn clone(&self) -> Self {
            Self {
                f: self.f.clone(),
                _marker: std::marker::PhantomData,
            }
        }
    }

    #[cfg(feature = "tagged-result")]
    trait ResponseBound:
        Sync + Send + 'static + crate::specialized_serialization::SpecializedSerialize
    {
    }
    #[cfg(feature = "tagged-result")]
    impl<T> ResponseBound for T where
        T: Sync + Send + 'static + crate::specialized_serialization::SpecializedSerialize
    {
    }
    #[cfg(not(feature = "tagged-result"))]
    trait ResponseBound: Sync + Send + 'static + Serialize {}
    #[cfg(not(feature = "tagged-result"))]
    impl<T> ResponseBound for T where T: Sync + Send + 'static + Serialize {}

    impl<Response, External, F, Fut> ApiFn for HandlerAxum<(), Response, Axum<External>, F>
    where
        Response: ResponseBound,
        External: FromRequestParts<()> + Sync + Send + 'static,
        Fut: std::future::Future<Output = Response> + Send + 'static,
        F: Sync + Send + 'static + Clone + Fn(Axum<External>) -> Fut,
    {
        fn register(self, api: &mut Api) {
            let path = format!("/{}", function_name(&self.f));

            let handler = move |request: axum::http::Request<hyper::Body>| async {
                let this = self;

                let (mut parts, body) = request.into_parts();
                let external: External = FromRequestParts::from_request_parts(&mut parts, &())
                    .await
                    .map_err(|e: <External as FromRequestParts<()>>::Rejection| {
                        e.into_response()
                    })?;

                let res = (this.f)(Axum(external)).await;
                #[cfg(feature = "tagged-result")]
                let res = res.boxed();

                Ok::<_, axum::response::Response>(Json(res))
            };

            let router = api.axum_router.take().unwrap();
            api.axum_router
                .replace(router.route(&path, axum::routing::post(handler)));
        }
    }

    impl<Response, F, Fut> ApiFn for HandlerAxum<(), Response, (), F>
    where
        Response: ResponseBound,
        Fut: std::future::Future<Output = Response> + Send + 'static,
        F: Sync + Send + 'static + Clone + Fn() -> Fut,
    {
        fn register(self, api: &mut Api) {
            let path = format!("/{}", function_name(&self.f));

            let handler = move || async {
                let this = self;
                let res = (this.f)().await;
                #[cfg(feature = "tagged-result")]
                let res = res.boxed();

                Ok::<_, axum::response::Response>(Json(res))
            };

            let router = api.axum_router.take().unwrap();
            api.axum_router
                .replace(router.route(&path, axum::routing::post(handler)));
        }
    }

    macro_rules! impl_api_fn {
    ($($t:ident),* | $($a:tt),*) => {
        impl<$($t,)* Response, External, F, Fut> ApiFn
            for HandlerAxum<($($t,)*), Response, Axum<External>, F>
        where
            $($t: Sync + Send + 'static + DeserializeOwned,)*
            Response: ResponseBound,
            External: Sync + Send + 'static + FromRequestParts<()>,
            Fut: std::future::Future<Output = Response> + Send + 'static,
            F: Sync + Send + 'static + Clone + Fn($($t,)* Axum<External>) -> Fut,
        {
            fn register(self, api: &mut Api) {
                let path = format!("/{}", function_name(&self.f));

                let handler = move |request: axum::http::Request<hyper::Body>| async {
                    let this = self;

                    let (mut parts, body) = request.into_parts();
                    let external = FromRequestParts::from_request_parts(&mut parts, &())
                        .await
                        .map_err(|e: <External as FromRequestParts<()>>::Rejection| {
                            e.into_response()
                        })?;

                    let request = axum::http::Request::from_parts(parts, body);
                    let params: ($($t,)*) = Json::from_request(request, &())
                        .await
                        .map_err(|e| e.into_response())?
                        .0;

                    let res = (this.f)($(params.$a,)* Axum(external)).await;
                    #[cfg(feature = "tagged-result")]
                    let res = res.boxed();

                    Ok::<_, axum::response::Response>(Json(res))
                };

                let router = api.axum_router.take().unwrap();
                api.axum_router
                    .replace(router.route(&path, axum::routing::post(handler)));
            }
        }

        impl<$($t,)* Response, F, Fut> ApiFn
        for HandlerAxum<($($t,)*), Response, (), F>
        where
            $($t: Sync + Send + 'static + DeserializeOwned,)*
            Response: ResponseBound,
            Fut: std::future::Future<Output = Response> + Send + 'static,
            F: Sync + Send + 'static + Clone + Fn($($t,)*) -> Fut,
        {
            fn register(self, api: &mut Api) {
                let path = format!("/{}", function_name(&self.f));

                let handler = move |request: axum::http::Request<hyper::Body>| async {
                    let this = self;

                    let params: ($($t,)*) = Json::from_request(request, &())
                        .await
                        .map_err(|e| e.into_response())?
                        .0;

                    let res = (this.f)($(params.$a,)*).await;
                    #[cfg(feature = "tagged-result")]
                    let res = res.boxed();

                    Ok::<_, axum::response::Response>(Json(res))
                };

                let router = api.axum_router.take().unwrap();
                api.axum_router
                    .replace(router.route(&path, axum::routing::post(handler)));
            }
        }
        };
    }

    impl_api_fn!(T | 0);
    impl_api_fn!(T0, T1 | 0, 1);
    impl_api_fn!(T0, T1, T2 | 0, 1, 2);
    impl_api_fn!(T0, T1, T2, T3 | 0, 1, 2, 3);
    impl_api_fn!(T0, T1, T2, T3, T4 | 0, 1, 2, 3, 4);
    impl_api_fn!(T0, T1, T2, T3, T4, T5 | 0, 1, 2, 3, 4, 5);
    impl_api_fn!(T0, T1, T2, T3, T4, T5, T6 | 0, 1, 2, 3, 4, 5, 6);
}

fn function_name<F: ?Sized>(f: &F) -> &'static str {
    std::any::type_name::<F>().split("::").last().unwrap()
}

fn type_name_of_val<T: ?Sized>(_val: &T) -> &'static str {
    std::any::type_name::<T>()
}

#[derive(Debug, Clone)]
pub struct TsFn {
    pub name: &'static str,
    // .. -> type declaration in typescript
    pub type_declarations: HashMap<ts_rs::Id, String>,
    // parameter name -> typescript type name with generics filled in
    pub request_types: Vec<(&'static str, String)>,
    // typescript type name with generics filled in
    pub response_type: String,
}

pub struct LazyTsFn(pub &'static once_cell::sync::Lazy<TsFn>);
inventory::collect!(LazyTsFn);

// use once_cell::sync::Lazy;

// static __A: Lazy<&'static str> = Lazy::new(|| "hello");
// inventory::submit! {
//     &__A.deref()
// }

impl TsFn {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            type_declarations: HashMap::new(),
            request_types: Vec::new(),
            response_type: String::new(),
        }
    }

    pub fn add_request_type<T: TS>(&mut self, param_name: &'static str) {
        self.add_type_definitions::<T>();
        self.request_types
            .push((param_name, T::name_with_generics()));
    }

    pub fn add_type_definitions<T: ts_rs::TS>(&mut self) {
        if let Some(decl) = T::decl() {
            self.type_declarations.insert(T::id(), decl);
        }
        self.type_declarations.extend(
            T::dependencies()
                .0
                .values()
                .map(|d| (d.id, d.ts_declaration.clone())),
        );
    }

    pub fn set_response_type<T: ts_rs::TS>(&mut self) {
        self.add_type_definitions::<T>();
        self.response_type = T::name_with_generics();
    }
}

#[cfg(feature = "tagged-result")]
mod specialized_serialization {
    // Specialized serialization for Result<T, E>
    #[derive(Debug, serde::Serialize, serde::Deserialize, ts_rs::TS)]
    #[serde(tag = "result", content = "value")]
    enum Result<T, E> {
        Ok(T),
        Err(E),
    }

    auto trait NotResult {}

    impl<T, E> !NotResult for std::result::Result<T, E> {}

    pub trait SpecializedSerialize {
        fn boxed(self) -> Box<dyn erased_serde::Serialize>;
    }

    impl<T: serde::Serialize + NotResult + 'static> SpecializedSerialize for T {
        fn boxed(self) -> Box<dyn erased_serde::Serialize> {
            Box::new(self)
        }
    }

    impl<T: serde::Serialize + 'static, E: serde::Serialize + 'static> SpecializedSerialize
        for std::result::Result<T, E>
    {
        fn boxed(self) -> Box<dyn erased_serde::Serialize> {
            let tagged_result = match self {
                Ok(ok) => Result::Ok(ok),
                Err(err) => Result::Err(err),
            };
            Box::new(tagged_result)
        }
    }
}

#[test]
fn test_ts_fn() {
    #[derive(TS)]
    struct Response {
        content: String,
        ms: u32,
    }

    fn hello(name: Response) {}
}
