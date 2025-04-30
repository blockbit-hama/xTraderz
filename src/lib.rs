/**
* filename : lib
* author : HAMA
* date: 2025. 4. 10.
* description: 
**/
pub mod models;
pub mod matching_engine;
pub mod sequencer;
pub mod order_manager;
pub mod websocket;
pub mod relay;
pub mod util;

// 편의를 위해 주요 구조체 및 기능 재내보내기
pub use models::{Order, OrderMessage, Execution, OrderBook, Side, OrderType, OrderStatus, PriceLevel};
pub use matching_engine::run as run_matching_engine;
pub use sequencer::run as run_sequencer;
pub use websocket::execution_push::ExecutionPushManager;
pub use websocket::orderbook_relay::OrderBookRelayManager;
pub use relay::server::RelayServer;
pub use relay::client_handler::{ClientHandler, ClientMessage, ClientUpdate};