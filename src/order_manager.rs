/**
* filename : order_manager
* author : HAMA
* date: 2025. 4. 28.
* description: 
**/

use warp::{Filter, Rejection, Reply, http::StatusCode};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use crate::models::{Order, OrderStatus, OrderType, Side, OrderMessage, Execution};
use chrono::Utc;

#[derive(Deserialize)]
struct NewOrder {
  symbol: String,
  side: Side,
  price: u64,
  order_type: OrderType,
  quantity: u64
}

#[derive(Deserialize)]
struct CancelOrder {
  order_id: String
}

#[derive(Deserialize)]
struct ExecQuery {
  symbol: Option<String>,
  order_id: Option<String>,
  start_time: Option<String>,
  end_time: Option<String>
}

pub fn routes(order_tx: Sender<OrderMessage>, exec_store: Arc<tokio::sync::Mutex<Vec<Execution>>>) -> impl Filter<Extract = impl Reply> + Clone {
  let post_order = warp::path!("v1" / "order")
    .and(warp::post())
    .and(warp::body::json())
    .and(with_tx(order_tx.clone()))
    .and_then(handle_post_order);
  
  let cancel_order = warp::path!("v1" / "order" / "cancel")
    .and(warp::post())
    .and(warp::body::json())
    .and(with_tx(order_tx.clone()))
    .and_then(handle_cancel_order);
  
  let get_executions = warp::path!("v1" / "execution")
    .and(warp::get())
    .and(warp::query::<ExecQuery>())
    .and(with_store(exec_store.clone()))
    .and_then(handle_get_executions);
  
  post_order.or(cancel_order).or(get_executions)
}

fn with_tx(tx: Sender<OrderMessage>) -> impl Filter<Extract = (Sender<OrderMessage>,)> + Clone {
  warp::any().map(move || tx.clone())
}

fn with_store(store: Arc<tokio::sync::Mutex<Vec<Execution>>>) -> impl Filter<Extract = (Arc<tokio::sync::Mutex<Vec<Execution>>>,)> + Clone {
  warp::any().map(move || store.clone())
}

async fn handle_post_order(new: NewOrder, tx: Sender<OrderMessage>) -> Result<impl Reply, Rejection> {
  // In a real system, we would validate account balance here
  
  let order = Order {
    order_id: uuid::Uuid::new_v4().to_string(),
    symbol: new.symbol,
    price: new.price,
    quantity: new.quantity,
    side: new.side,
    order_type: new.order_type,
    status: OrderStatus::New,
    filled_quantity: 0,
    remain_quantity: new.quantity,
    entry_time: Utc::now()
  };
  
  tx.send(OrderMessage(order.clone())).await.map_err(|_| warp::reject())?;
  Ok(warp::reply::with_status(warp::reply::json(&order), StatusCode::CREATED))
}

async fn handle_cancel_order(cancel: CancelOrder, tx: Sender<OrderMessage>) -> Result<impl Reply, Rejection> {
  // In a real system, we would validate order ownership here
  
  let order = Order {
    order_id: cancel.order_id,
    symbol: String::new(), // These fields would be populated from a database lookup
    price: 0,
    quantity: 0,
    side: Side::Buy,
    order_type: OrderType::Limit,
    status: OrderStatus::Cancelled,
    filled_quantity: 0,
    remain_quantity: 0,
    entry_time: Utc::now()
  };
  
  tx.send(OrderMessage(order.clone())).await.map_err(|_| warp::reject())?;
  Ok(warp::reply::with_status(warp::reply::json(&order), StatusCode::OK))
}

async fn handle_get_executions(q: ExecQuery, store: Arc<tokio::sync::Mutex<Vec<Execution>>>) -> Result<impl Reply, Rejection> {
  let data = store.lock().await.clone();
  
  let filtered: Vec<Execution> = data.into_iter().filter(|e| {
    if let Some(ref sym) = q.symbol {
      if &e.symbol != sym { return false; }
    }
    if let Some(ref oid) = q.order_id {
      if &e.order_id != oid { return false; }
    }
    // Time-based filtering could be added here
    true
  }).collect();
  
  Ok(warp::reply::json(&filtered))
}