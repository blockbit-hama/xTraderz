// /**
// * filename : handler
// * author : HAMA
// * date: 2025. 4. 10.
// * description: API 핸들러 모듈
// **/
//
// use std::sync::Arc;
// use tokio::sync::Mutex;
// use warp::{Rejection, Reply};
// use log::{info, error, debug};
// use crate::trading_engine::order::Order;
// use crate::trading_engine::trading::TradingSystem;
// use crate::errors::AppError;
//
// pub type SharedTradingSystem = Arc<Mutex<TradingSystem>>;
//
// // 주문 추가 핸들러
// pub async fn handle_add_order(
//   order: Order,
//   trading_system: SharedTradingSystem,
// ) -> Result<impl Reply, Rejection> {
//   info!("주문 추가 요청: ID={}, Action={}, Quantity={}, Price={}",
//         order.order_id, order.action, order.quantity, order.price);
//
//   // 주문 유효성 검증
//   if order.quantity == 0 {
//     error!("주문 수량이 0입니다: {}", order.order_id);
//     return Err(AppError::InvalidOrderField("수량은 0보다 커야 합니다".to_string()).into());
//   }
//
//   if order.price == 0 {
//     error!("주문 가격이 0입니다: {}", order.order_id);
//     return Err(AppError::InvalidOrderField("가격은 0보다 커야 합니다".to_string()).into());
//   }
//
//   if order.action != "buy" && order.action != "sell" {
//     error!("잘못된 주문 행동입니다: {}", order.action);
//     return Err(AppError::InvalidOrderField("action은 'buy' 또는 'sell'이어야 합니다".to_string()).into());
//   }
//
//   let mut system = trading_system.lock().await;
//
//   // 여기서 중복 주문 ID 체크를 추가할 수 있습니다
//   // 지금은 간단하게 가정하고 넘어갑니다
//
//   system.add_order(order.clone());
//   debug!("주문이 성공적으로 추가되었습니다: {}", order.order_id);
//
//   Ok(warp::reply::json(&serde_json::json!({
//     "status": "success",
//     "message": "주문이 성공적으로 추가되었습니다",
//     "data": {
//       "order_id": order.order_id
//     }
//   })))
// }
//
// // 주문 실행 핸들러
// pub async fn handle_execute_orders(
//   trading_system: SharedTradingSystem,
// ) -> Result<impl Reply, Rejection> {
//   info!("주문 실행 요청이 수신되었습니다");
//
//   let mut system = trading_system.lock().await;
//   let before_count = system.delayed_order_book.get_orders().len();
//
//   system.execute_order();
//
//   let after_count = system.delayed_order_book.get_orders().len();
//   let executed_count = before_count - after_count;
//
//   info!("실행된 주문 수: {}", executed_count);
//
//   Ok(warp::reply::json(&serde_json::json!({
//     "status": "success",
//     "message": "주문 실행이 완료되었습니다",
//     "data": {
//       "executed_count": executed_count,
//       "pending_count": after_count
//     }
//   })))
// }
//


/**
* filename : handlers
* author : HAMA
* date: 2025. 4. 11.
* description: API 핸들러 모듈
**/
use axum::{
  extract::{Json, State},
  http::StatusCode,
  response::IntoResponse,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::trading_engine::order::Order;
use crate::trading_engine::trading::TradingSystem;

pub type SharedTradingSystem = Arc<Mutex<TradingSystem>>;

// 주문 추가 핸들러
pub async fn handle_add_order(
  State(trading_system): State<SharedTradingSystem>,
  Json(order): Json<Order>,
) -> Result<impl IntoResponse, StatusCode> {
  tracing::info!("주문 추가 요청: ID={}, Action={}, Quantity={}, Price={}",
        order.order_id, order.action, order.quantity, order.price);
  
  // 주문 유효성 검증
  if order.quantity == 0 {
    tracing::error!("주문 수량이 0입니다: {}", order.order_id);
    return Err(StatusCode::BAD_REQUEST);
  }
  
  if order.price == 0 {
    tracing::error!("주문 가격이 0입니다: {}", order.order_id);
    return Err(StatusCode::BAD_REQUEST);
  }
  
  if order.action != "buy" && order.action != "sell" {
    tracing::error!("잘못된 주문 행동입니다: {}", order.action);
    return Err(StatusCode::BAD_REQUEST);
  }
  
  let mut system = trading_system.lock().await;
  
  // 여기서 중복 주문 ID 체크를 추가할 수 있습니다
  // 지금은 간단하게 가정하고 넘어갑니다
  
  system.add_order(order.clone());
  tracing::debug!("주문이 성공적으로 추가되었습니다: {}", order.order_id);
  
  Ok(Json(serde_json::json!({
        "status": "success",
        "message": "주문이 성공적으로 추가되었습니다",
        "data": {
            "order_id": order.order_id
        }
    })))
}

// 주문 실행 핸들러
pub async fn handle_execute_orders(
  State(trading_system): State<SharedTradingSystem>,
) -> impl IntoResponse {
  tracing::info!("주문 실행 요청이 수신되었습니다");
  
  let mut system = trading_system.lock().await;
  let before_count = system.delayed_order_book.get_orders().len();
  
  system.execute_order();
  
  let after_count = system.delayed_order_book.get_orders().len();
  let executed_count = before_count - after_count;
  
  tracing::info!("실행된 주문 수: {}", executed_count);
  
  Json(serde_json::json!({
        "status": "success",
        "message": "주문 실행이 완료되었습니다",
        "data": {
            "executed_count": executed_count,
            "pending_count": after_count
        }
    }))
}