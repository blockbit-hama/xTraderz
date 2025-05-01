/**
* filename : publisher
* author : HAMA
* date: 2025. 5. 1.
* description: 
**/

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::Utc;
use warp::Filter;
use serde::Serialize;
use warp::http::StatusCode;

use crate::models::{Order, Execution, OrderBook};
use super::candlestick::CandlestickManager;
use super::models::{OrderBookData, PriceLevel, ExecutionData, MarketStatistics, CandleInterval, Candle};

// 시장 데이터 발행자
pub struct MarketDataPublisher {
  // 심볼별 오더북 상태
  orderbooks: Arc<Mutex<HashMap<String, OrderBook>>>,
  // 최근 체결 내역 (심볼 -> 체결 목록)
  recent_executions: Arc<Mutex<HashMap<String, Vec<ExecutionData>>>>,
  // 시장 통계 (심볼 -> 통계)
  market_statistics: Arc<Mutex<HashMap<String, MarketStatistics>>>,
  // 캔들스틱 관리자
  candlestick_manager: Arc<CandlestickManager>,
  // 최대 저장 체결 수
  max_executions: usize,
}

impl MarketDataPublisher {
  // 새 시장 데이터 발행자 생성
  pub fn new() -> Self {
    MarketDataPublisher {
      orderbooks: Arc::new(Mutex::new(HashMap::new())),
      recent_executions: Arc::new(Mutex::new(HashMap::new())),
      market_statistics: Arc::new(Mutex::new(HashMap::new())),
      candlestick_manager: Arc::new(CandlestickManager::new()),
      max_executions: 1000, // 심볼당 최대 1000개 체결 저장
    }
  }
  
  // 오더북 업데이트
  pub fn update_orderbook(&self, symbol: &str, orderbook: OrderBook) {
    let mut orderbooks = self.orderbooks.lock().unwrap();
    orderbooks.insert(symbol.to_string(), orderbook);
  }
  
  // 체결 처리
  pub fn process_execution(&self, execution: &Execution) {
    // 1. 최근 체결 내역에 추가
    self.add_execution(execution);
    
    // 2. 캔들스틱 업데이트
    self.candlestick_manager.process_execution(execution);
    
    // 3. 시장 통계 업데이트
    self.update_market_statistics(execution);
  }
  
  // 체결 내역 추가
  fn add_execution(&self, execution: &Execution) {
    let mut executions = self.recent_executions.lock().unwrap();
    
    let symbol_executions = executions
      .entry(execution.symbol.clone())
      .or_insert_with(Vec::new);
    
    // 새 체결 데이터 생성
    let execution_data = ExecutionData {
      symbol: execution.symbol.clone(),
      timestamp: execution.transaction_time,
      price: execution.price,
      volume: execution.quantity,
      side: format!("{:?}", execution.side),
      is_market_maker: false, // 기본적으로 false로 설정
    };
    
    // 목록 시작에 추가 (최신 체결이 먼저 오도록)
    symbol_executions.insert(0, execution_data);
    
    // 최대 개수 제한
    if symbol_executions.len() > self.max_executions {
      symbol_executions.truncate(self.max_executions);
    }
  }
  
  // 시장 통계 업데이트
  fn update_market_statistics(&self, execution: &Execution) {
    let mut stats = self.market_statistics.lock().unwrap();
    let symbol = &execution.symbol;
    
    let symbol_stats = stats
      .entry(symbol.clone())
      .or_insert_with(|| MarketStatistics {
        symbol: symbol.clone(),
        timestamp: Utc::now(),
        open_price_24h: execution.price,
        high_price_24h: execution.price,
        low_price_24h: execution.price,
        last_price: execution.price,
        volume_24h: 0,
        price_change_24h: 0.0,
        bid_price: 0,
        ask_price: 0,
      });
    
    // 24시간 초과 시 통계 초기화
    let now = Utc::now();
    if (now - symbol_stats.timestamp).num_seconds() > 86400 {
      symbol_stats.timestamp = now;
      symbol_stats.open_price_24h = execution.price;
      symbol_stats.high_price_24h = execution.price;
      symbol_stats.low_price_24h = execution.price;
      symbol_stats.volume_24h = 0;
    }
    
    // 통계 업데이트
    symbol_stats.high_price_24h = symbol_stats.high_price_24h.max(execution.price);
    symbol_stats.low_price_24h = symbol_stats.low_price_24h.min(execution.price);
    symbol_stats.last_price = execution.price;
    symbol_stats.volume_24h += execution.quantity;
    
    // 가격 변화율 계산
    if symbol_stats.open_price_24h > 0 {
      let price_diff = execution.price as f64 - symbol_stats.open_price_24h as f64;
      symbol_stats.price_change_24h = price_diff / symbol_stats.open_price_24h as f64 * 100.0;
    }
    
    // 현재 최고 매수가, 최저 매도가 업데이트
    let orderbooks = self.orderbooks.lock().unwrap();
    if let Some(orderbook) = orderbooks.get(symbol) {
      if let Some(best_bid) = orderbook.buy_book.get_best_level() {
        symbol_stats.bid_price = best_bid.price;
      }
      
      if let Some(best_ask) = orderbook.sell_book.get_best_level() {
        symbol_stats.ask_price = best_ask.price;
      }
    }
  }
  
  // HTTP API 라우트 설정
  pub fn routes(&self) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let orderbooks = self.orderbooks.clone();
    let executions = self.recent_executions.clone();
    let statistics = self.market_statistics.clone();
    let candlestick_manager = self.candlestick_manager.clone();
    
    // 오더북 조회 API
    let get_orderbook = warp::path!("api" / "v1" / "orderbook" / String)
      .and(warp::get())
      .map(move |symbol: String| {
        let orderbooks = orderbooks.lock().unwrap();
        
        if let Some(orderbook) = orderbooks.get(&symbol) {
          // 오더북 DTO로 변환
          let orderbook_data = convert_to_orderbook_data(&symbol, orderbook);
          warp::reply::json(&orderbook_data)
        } else {
          // 심볼을 찾을 수 없는 경우
          let empty_response = OrderBookData {
            symbol: symbol.clone(),
            timestamp: Utc::now(),
            bids: Vec::new(),
            asks: Vec::new(),
          };
          warp::reply::json(&empty_response)
        }
      });
    
    // 체결 내역 조회 API
    let get_executions = warp::path!("api" / "v1" / "executions" / String)
      .and(warp::get())
      .and(warp::query::<HashMap<String, String>>())
      .map(move |symbol: String, params: HashMap<String, String>| {
        let executions = executions.lock().unwrap();
        
        // 제한 개수 파라미터 파싱
        let limit = params.get("limit")
          .and_then(|v| v.parse::<usize>().ok())
          .unwrap_or(100);
        
        if let Some(symbol_executions) = executions.get(&symbol) {
          // 요청된 개수만큼 반환
          let result = symbol_executions.iter()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
          
          warp::reply::json(&result)
        } else {
          // 심볼을 찾을 수 없는 경우
          warp::reply::json(&Vec::<ExecutionData>::new())
        }
      });
    
    // 시장 통계 조회 API
    let get_statistics = warp::path!("api" / "v1" / "statistics" / String)
      .and(warp::get())
      .map(move |symbol: String| {
        let statistics = statistics.lock().unwrap();
        
        if let Some(stats) = statistics.get(&symbol) {
          warp::reply::json(&stats)
        } else {
          // 심볼을 찾을 수 없는 경우
          let empty_statistics = MarketStatistics {
            symbol: symbol.clone(),
            timestamp: Utc::now(),
            open_price_24h: 0,
            high_price_24h: 0,
            low_price_24h: 0,
            last_price: 0,
            volume_24h: 0,
            price_change_24h: 0.0,
            bid_price: 0,
            ask_price: 0,
          };
          warp::reply::json(&empty_statistics)
        }
      });
    
    // 캔들스틱 조회 API
    let get_candlesticks = warp::path!("api" / "v1" / "klines" / String / String)
      .and(warp::get())
      .and(warp::query::<HashMap<String, String>>())
      .map(move |symbol: String, interval_str: String, params: HashMap<String, String>| {
        // 간격 파라미터 파싱
        let interval = match CandleInterval::from_string(&interval_str) {
          Some(interval) => interval,
          None => {
            return warp::reply::with_status(
              warp::reply::json(&serde_json::json!({
                                "error": format!("Invalid interval: {}", interval_str)
                            })),
              StatusCode::BAD_REQUEST
            );
          }
        };
        
        // 제한 개수 파라미터 파싱
        let limit = params.get("limit")
          .and_then(|v| v.parse::<usize>().ok())
          .unwrap_or(100);
        
        // 캔들스틱 데이터 가져오기
        let candles = candlestick_manager.get_candles(&symbol, interval, Some(limit));
        
        // 현재 진행 중인 캔들 추가
        let mut result = candles;
        if let Some(current_candle) = candlestick_manager.get_current_candle(&symbol, interval) {
          result.insert(0, current_candle);
        }
        
        warp::reply::json(&result)
      });
    
    // 모든 라우트 결합
    get_orderbook
      .or(get_executions)
      .or(get_statistics)
      .or(get_candlesticks)
  }
}

// OrderBook을 OrderBookData로 변환하는 헬퍼 함수
fn convert_to_orderbook_data(symbol: &str, orderbook: &OrderBook) -> OrderBookData {
  let mut bids = Vec::new();
  let mut asks = Vec::new();
  
  // 매수 호가 변환
  for (&price, level) in &orderbook.buy_book.limits {
    bids.push(PriceLevel {
      price,
      volume: level.total_volume,
      order_count: level.orders.len(),
    });
  }
  
  // 매도 호가 변환
  for (&price, level) in &orderbook.sell_book.limits {
    asks.push(PriceLevel {
      price,
      volume: level.total_volume,
      order_count: level.orders.len(),
    });
  }
  
  // 가격 순서대로 정렬
  bids.sort_by(|a, b| b.price.cmp(&a.price)); // 내림차순 (최고가 먼저)
  asks.sort_by(|a, b| a.price.cmp(&b.price)); // 오름차순 (최저가 먼저)
  
  OrderBookData {
    symbol: symbol.to_string(),
    timestamp: Utc::now(),
    bids,
    asks,
  }
}