/**
* filename : sequencer
* author : HAMA
* date: 2025. 4. 28.
* description: 
**/

use tokio::sync::mpsc::{Receiver, Sender, channel};
use crate::models::{OrderMessage, Execution};
use crate::matching_engine;

// Input sequencer receives orders from the API and forwards them to the matching engine
pub async fn run_input_sequencer(mut order_rx: Receiver<OrderMessage>, order_tx: Sender<OrderMessage>) {
  while let Some(order_message) = order_rx.recv().await {
    // In a real implementation, we could add sequence numbers, timestamps, etc.
    // We could also perform basic validation here
    if let Err(_) = order_tx.send(order_message).await {
      // Handle error (e.g., matching engine channel closed)
      break;
    }
  }
}

// Output sequencer receives executions from the matching engine and forwards them to storage
pub async fn run_output_sequencer(mut exec_rx: Receiver<Execution>, exec_tx: Sender<Execution>) {
  while let Some(execution) = exec_rx.recv().await {
    // In a real implementation, we could add sequence numbers, timestamps, etc.
    // We could also perform post-trade processing here
    if let Err(_) = exec_tx.send(execution).await {
      // Handle error (e.g., storage channel closed)
      break;
    }
  }
}

// Main sequencer coordinator
pub async fn run(order_rx: Receiver<OrderMessage>, exec_tx: Sender<Execution>) {
  // Create channels between input sequencer, matching engine, and output sequencer
  let (engine_order_tx, engine_order_rx) = channel(100);
  let (engine_exec_tx, engine_exec_rx) = channel(100);
  
  // Spawn input sequencer
  tokio::spawn(run_input_sequencer(order_rx, engine_order_tx));
  
  // Spawn matching engine
  tokio::spawn(matching_engine::run(engine_order_rx, engine_exec_tx));
  
  // Spawn output sequencer
  run_output_sequencer(engine_exec_rx, exec_tx).await;
}