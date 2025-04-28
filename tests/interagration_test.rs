/**
* filename : interagration_test
* author : HAMA
* date: 2025. 4. 10.
* description: 
**/

#[cfg(test)]
mod integration_tests {
  use warp::Filter;
  use tokio::sync::mpsc;
  use std::sync::Arc;
  use std::time::Duration;
  use xTraderz::models::{Order, OrderMessage, Side, OrderType, OrderStatus, Execution};
  use xTraderz::sequencer;
  use xTraderz::order_manager;
  use chrono::Utc;
  use warp::test::request;
  
  #[tokio::test]
  async fn integration_order_execution_flow() {
    // Setup channels and store
    let (order_tx, order_rx) = mpsc::channel(100);
    let (exec_tx, mut exec_rx) = mpsc::channel(100);
    let exec_store = Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let store_clone = exec_store.clone();
    
    // Spawn sequencer and persistence
    tokio::spawn(async move { sequencer::run(order_rx, exec_tx).await; });
    tokio::spawn(async move {
      while let Some(exec) = exec_rx.recv().await {
        store_clone.lock().await.push(exec);
      }
    });
    
    // Preload a sell order
    let sell = Order {
      order_id: "sell1".into(),
      symbol: "TST".into(),
      price: 100,
      quantity: 10,
      side: Side::Sell,
      order_type: OrderType::Limit,
      status: OrderStatus::New,
      filled_quantity: 0,
      remain_quantity: 10,
      entry_time: Utc::now()
    };
    order_tx.send(OrderMessage(sell)).await.unwrap();
    
    // Build API
    let api = order_manager::routes(order_tx.clone(), exec_store.clone());
    
    // Send buy order via HTTP POST
    let buy_req = serde_json::json!({
            "symbol": "TST",
            "side": "Buy",
            "price": 100,
            "order_type": "Limit",
            "quantity": 5
        });
    let resp = request()
      .method("POST")
      .path("/v1/order")
      .json(&buy_req)
      .reply(&api)
      .await;
    assert_eq!(resp.status(), warp::http::StatusCode::CREATED);
    
    // Allow matching to process
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Retrieve executions via HTTP GET
    let resp = request()
      .method("GET")
      .path("/v1/execution?symbol=TST")
      .reply(&api)
      .await;
    let executions: Vec<Execution> = serde_json::from_slice(resp.body()).unwrap();
    
    // We expect 2 executions - one for each side of the trade
    assert_eq!(executions.len(), 2);
    assert_eq!(executions[0].quantity, 5);
  }
}