use axum::http::{HeaderValue, Method};
use serde::Deserialize;
use tower_http::cors::CorsLayer;
use ts_rpc::{ts_export, ts_rs, Api, TS};

#[tokio::main]
async fn main() {
    #[derive(TS, Deserialize)]
    struct Password {
        password: String,
    }

    #[ts_export]
    async fn login(
        email: String,
        password: Password,
        // axum: Axum<(axum::http::header::HeaderMap,)>,
    ) -> String {
        email
    }

    let mut api = Api::new();
    api.register_axum(login);
    api.export_ts_client("http://localhost:3003", "../api.ts")
        .unwrap();

    let router = api.axum_router.take().unwrap();
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
