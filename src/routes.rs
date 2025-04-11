// /**
// * filename : routes
// * author : HAMA
// * date: 2025. 4. 11.
// * description:
// **/
//
//
// use std::collections::HashMap;
// use std::sync::{Arc, RwLock};
//
// use axum::{
//   error_handling::HandleErrorLayer,
//   extract::{Path, Query, State},
//   http::StatusCode,
//   response::IntoResponse,
//   routing::{get, patch},
//   Json, Router,
// };
// use std::time::Duration;
// use tower::{BoxError, ServiceBuilder};
// use tower_http::trace::TraceLayer;
// use uuid::Uuid;
// use crate::handlers;
// use crate::models::Todo;
//
// pub fn createRouter(db: Arc<RwLock<HashMap<Uuid, Todo>>>) -> Router {
//
//   return Router::new()
//     .route("/todos", get(handlers::todos_index).post(handlers::todos_create))
//     .route("/todos/{id}", patch(handlers::todos_update).delete(handlers::todos_delete))
//     // Add middleware to all routes
//     .layer(
//       ServiceBuilder::new()
//         .layer(HandleErrorLayer::new(|error: BoxError| async move {
//           if error.is::<tower::timeout::error::Elapsed>() {
//             Ok(StatusCode::REQUEST_TIMEOUT)
//           } else {
//             Err((
//               StatusCode::INTERNAL_SERVER_ERROR,
//               format!("Unhandled internal error: {error}"),
//             ))
//           }
//         }))
//         .timeout(Duration::from_secs(10))
//         .layer(TraceLayer::new_for_http())
//         .into_inner(),
//     )
//     .with_state(db);
// }

/**
* filename : routes
* author : HAMA
* date: 2025. 4. 11.
* description: 트레이딩 시스템 라우터 설정
**/

use std::time::Duration;
use axum::{
  error_handling::HandleErrorLayer,
  http::StatusCode,
  response::IntoResponse,
  routing::{get, post},
  Router,
};
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;
use crate::handlers;
use crate::handlers::SharedTradingSystem;

pub fn create_router(trading_system: SharedTradingSystem) -> Router {
  
  return Router::new()
    .route("/orders", post(handlers::handle_add_order))
    .route("/execute", post(handlers::handle_execute_orders))
    // Add middleware to all routes
    .layer(
      ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|error: BoxError| async move {
          if error.is::<tower::timeout::error::Elapsed>() {
            Ok(StatusCode::REQUEST_TIMEOUT)
          } else {
            Err((
              StatusCode::INTERNAL_SERVER_ERROR,
              format!("Unhandled internal error: {error}"),
            ))
          }
        }))
        .timeout(Duration::from_secs(10))
        .layer(TraceLayer::new_for_http())
        .into_inner(),
    )
    .with_state(trading_system);
}