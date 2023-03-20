use axum::http::{HeaderValue, Method};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use ts_rpc::{ts_export, ts_rs, Api, TS};

#[tokio::main]
async fn main() {
    #[derive(TS, Deserialize, Serialize)]
    struct Password {
        password: String,
    }

    #[derive(TS, Serialize)]
    struct ReturnType<T> {
        inner: T,
    };

    #[ts_export]
    async fn login(
        email: String,
        password: Password,
        // axum: Axum<(axum::http::header::HeaderMap,)>,
    ) -> ReturnType<Password> {
        ReturnType { inner: password }
    }

    let api = Api::new().register_axum(login);
    api.export_ts_client("http://localhost:3003", "../api.ts")
        .unwrap();

    let router = api.axum_router();
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3003));
    axum::Server::bind(&addr)
        .serve(
            router
                .layer(
                    CorsLayer::new()
                        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
                        .allow_methods([Method::POST])
                        .allow_headers([axum::http::header::CONTENT_TYPE]),
                )
                .into_make_service(),
        )
        .await
        .unwrap();
}
