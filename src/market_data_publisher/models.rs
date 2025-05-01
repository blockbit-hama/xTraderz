/**
* filename : models
* author : HAMA
* date: 2025. 5. 1.
* description: 
**/

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::Mutex;

// 시장 데이터 유형
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MarketDataType {
  OrderBook,   // 호가창 데이터
  Execution,   // 체결 데이터
  Candlestick, // 봉차트 데이터
  Statistics,  // 시장 통계 데이터
}

// 호가창 데이터
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderBookData {
  pub symbol: String,
  pub timestamp: DateTime<Utc>,
  pub bids: Vec<PriceLevel>,  // 매수 호가 (가격 내림차순)
  pub asks: Vec<PriceLevel>,  // 매도 호가 (가격 오름차순)
}

// 가격 레벨
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriceLevel {
  pub price: u64,
  pub volume: u64,
  pub order_count: usize,
}

// 체결 데이터
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionData {
  pub symbol: String,
  pub timestamp: DateTime<Utc>,
  pub price: u64,
  pub volume: u64,
  pub side: String,            // "Buy" 또는 "Sell"
  pub is_market_maker: bool,   // 메이커 여부
}

// 봉차트 데이터
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candle {
  pub symbol: String,
  pub open_time: DateTime<Utc>,
  pub close_time: DateTime<Utc>,
  pub interval: CandleInterval,
  pub open: u64,
  pub high: u64,
  pub low: u64,
  pub close: u64,
  pub volume: u64,
  pub trade_count: usize,
}

// 봉차트 간격
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CandleInterval {
  Minute1,
  Minute5,
  Minute15,
  Minute30,
  Hour1,
  Hour4,
  Day1,
  Week1,
}

impl CandleInterval {
  // 간격을 초 단위로 변환
  pub fn to_seconds(&self) -> i64 {
    match self {
      CandleInterval::Minute1 => 60,
      CandleInterval::Minute5 => 300,
      CandleInterval::Minute15 => 900,
      CandleInterval::Minute30 => 1800,
      CandleInterval::Hour1 => 3600,
      CandleInterval::Hour4 => 14400,
      CandleInterval::Day1 => 86400,
      CandleInterval::Week1 => 604800,
    }
  }
  
  // 문자열로 변환
  pub fn to_string(&self) -> String {
    match self {
      CandleInterval::Minute1 => "1m".to_string(),
      CandleInterval::Minute5 => "5m".to_string(),
      CandleInterval::Minute15 => "15m".to_string(),
      CandleInterval::Minute30 => "30m".to_string(),
      CandleInterval::Hour1 => "1h".to_string(),
      CandleInterval::Hour4 => "4h".to_string(),
      CandleInterval::Day1 => "1d".to_string(),
      CandleInterval::Week1 => "1w".to_string(),
    }
  }
  
  // 문자열에서 변환
  pub fn from_string(s: &str) -> Option<Self> {
    match s {
      "1m" => Some(CandleInterval::Minute1),
      "5m" => Some(CandleInterval::Minute5),
      "15m" => Some(CandleInterval::Minute15),
      "30m" => Some(CandleInterval::Minute30),
      "1h" => Some(CandleInterval::Hour1),
      "4h" => Some(CandleInterval::Hour4),
      "1d" => Some(CandleInterval::Day1),
      "1w" => Some(CandleInterval::Week1),
      _ => None,
    }
  }
}

// 시장 통계 데이터
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarketStatistics {
  pub symbol: String,
  pub timestamp: DateTime<Utc>,
  pub open_price_24h: u64,     // 24시간 시작가
  pub high_price_24h: u64,     // 24시간 최고가
  pub low_price_24h: u64,      // 24시간 최저가
  pub last_price: u64,         // 최근 거래가
  pub volume_24h: u64,         // 24시간 거래량
  pub price_change_24h: f64,   // 24시간 가격 변화율
  pub bid_price: u64,          // 최고 매수가
  pub ask_price: u64,          // 최저 매도가
}

// 원형 버퍼 구현 (봉차트 데이터 저장용)
pub struct CircularBuffer<T> {
  buffer: VecDeque<T>,
  capacity: usize,
}

impl<T: Clone> CircularBuffer<T> {
  // 새로운 원형 버퍼 생성
  pub fn new(capacity: usize) -> Self {
    CircularBuffer {
      buffer: VecDeque::with_capacity(capacity),
      capacity,
    }
  }
  
  // 값 추가
  pub fn push(&mut self, value: T) {
    if self.buffer.len() == self.capacity {
      self.buffer.pop_front(); // 가장 오래된 항목 제거
    }
    self.buffer.push_back(value);
  }
  
  // 모든 데이터 얻기
  pub fn get_all(&self) -> Vec<T> {
    self.buffer.iter().cloned().collect()
  }
  
  // 최근 N개 데이터 얻기
  pub fn get_recent(&self, count: usize) -> Vec<T> {
    let start = if count >= self.buffer.len() {
      0
    } else {
      self.buffer.len() - count
    };
    
    self.buffer.range(start..).cloned().collect()
  }
  
  // 버퍼 길이
  pub fn len(&self) -> usize {
    self.buffer.len()
  }
  
  // 버퍼가 비었는지 확인
  pub fn is_empty(&self) -> bool {
    self.buffer.is_empty()
  }
  
  // 버퍼 용량
  pub fn capacity(&self) -> usize {
    self.capacity
  }
}