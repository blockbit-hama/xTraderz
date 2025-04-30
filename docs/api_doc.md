# 주문 매칭 엔진 API 문서

이 문서는 주문 매칭 엔진의 API 엔드포인트와 WebSocket 인터페이스에 대해 설명합니다.

## 시스템 개요

주문 매칭 엔진은 두 가지 주요 인터페이스를 제공합니다:

1. **REST API**: 주문 생성, 취소, 조회를 위한 HTTP 기반 인터페이스
2. **WebSocket API**: 실시간 오더북 및 체결 정보를 위한 양방향 인터페이스

이 두 인터페이스는 별도의 데이터 흐름으로 작동합니다:
- **주문 처리 파이프라인**: REST API를 통해 접수된 주문을 처리하고 HTTP 응답 반환
- **UI 업데이트 파이프라인**: 오더북 상태 및 체결 정보를 WebSocket을 통해 클라이언트 UI에 실시간으로 전달

이러한 분리된 아키텍처는 주문 처리의 성능과 안정성을 해치지 않으면서도 많은 수의 UI 클라이언트에 실시간 데이터를 제공할 수 있도록 합니다.

## 기본 정보

- **REST API 기본 URL**: `http://127.0.0.1:3030`
- **WebSocket 기본 URL**: `ws://127.0.0.1:3030`
- **컨텐츠 타입**: `application/json`
- **인증**: 현재 버전에서는 인증이 구현되지 않았습니다. 프로덕션 환경에서는 적절한 인증 시스템이 필요합니다.

## REST API 엔드포인트

### 1. 주문 생성

새로운 주문을 생성합니다.

- **URL**: `/v1/order`
- **메서드**: `POST`
- **요청 본문**:

```json
{
  "symbol": "BTC-KRW",   // 거래 심볼
  "side": "Buy",         // 주문 방향: "Buy" 또는 "Sell"
  "price": 50000000,     // 주문 가격 (원)
  "order_type": "Limit", // 주문 유형: "Limit" 또는 "Market"
  "quantity": 1.5        // 주문 수량
}
```

- **응답**: 생성된 주문 정보

```json
{
  "order_id": "f8c3de3d-1fea-4d7c-a8b0-29f63c4c3454",
  "symbol": "BTC-KRW",
  "price": 50000000,
  "quantity": 1.5,
  "side": "Buy",
  "order_type": "Limit",
  "status": "New",
  "filled_quantity": 0,
  "remain_quantity": 1.5,
  "entry_time": "2023-04-30T12:34:56.789Z"
}
```

- **상태 코드**:
  - `201 Created`: 주문이 성공적으로 생성됨
  - `400 Bad Request`: 유효하지 않은 요청
  - `500 Internal Server Error`: 서버 오류

### 2. 주문 취소

기존 주문을 취소합니다.

- **URL**: `/v1/order/cancel`
- **메서드**: `POST`
- **요청 본문**:

```json
{
  "order_id": "f8c3de3d-1fea-4d7c-a8b0-29f63c4c3454"
}
```

- **응답**: 취소된 주문 정보

```json
{
  "order_id": "f8c3de3d-1fea-4d7c-a8b0-29f63c4c3454",
  "symbol": "BTC-KRW",
  "price": 50000000,
  "quantity": 1.5,
  "side": "Buy",
  "order_type": "Limit",
  "status": "Cancelled",
  "filled_quantity": 0.5,
  "remain_quantity": 1.0,
  "entry_time": "2023-04-30T12:34:56.789Z"
}
```

- **상태 코드**:
  - `200 OK`: 주문이 성공적으로 취소됨
  - `400 Bad Request`: 유효하지 않은 요청
  - `404 Not Found`: 주문을 찾을 수 없음
  - `500 Internal Server Error`: 서버 오류

### 3. 체결 내역 조회

주문 체결 내역을 조회합니다.

- **URL**: `/v1/execution`
- **메서드**: `GET`
- **쿼리 파라미터**:
  - `symbol` (선택): 심볼로 필터링
  - `order_id` (선택): 주문 ID로 필터링
  - `start_time` (선택): 시작 시간 (ISO 8601 형식)
  - `end_time` (선택): 종료 시간 (ISO 8601 형식)

- **응답**: 체결 내역 목록

```json
[
  {
    "exec_id": "e1b724c2-5e61-4aba-8b8a-47d8a5a4f111",
    "order_id": "f8c3de3d-1fea-4d7c-a8b0-29f63c4c3454",
    "symbol": "BTC-KRW",
    "side": "Buy",
    "price": 50000000,
    "quantity": 0.5,
    "fee": 0.05,
    "transaction_time": "2023-04-30T12:35:10.123Z"
  },
  {
    "exec_id": "92a8b678-4321-4f9a-9876-1a2b3c4d5e6f",
    "order_id": "abc123de-5678-9f01-2345-67890abcdef1",
    "symbol": "BTC-KRW",
    "side": "Sell",
    "price": 50000000,
    "quantity": 0.5,
    "fee": 0.05,
    "transaction_time": "2023-04-30T12:35:10.123Z"
  }
]
```

- **상태 코드**:
  - `200 OK`: 성공
  - `400 Bad Request`: 유효하지 않은 요청
  - `500 Internal Server Error`: 서버 오류

## WebSocket API

WebSocket API는 **주문 처리 파이프라인과 독립적**으로 작동하는 별도의 UI 업데이트 파이프라인의 일부입니다. 이는 많은 수의 UI 연결로 인한 부하가 핵심 거래 기능에 영향을 주지 않도록 하기 위함입니다.

### 1. 체결 정보 스트림

실시간 체결 정보를 구독합니다.

- **URL**: `ws://127.0.0.1:3030/ws/executions`
- **메서드**: WebSocket 연결

- **응답 메시지 형식**: 체결이 발생할 때마다 다음 형식의 메시지가 전송됩니다.

```json
{
  "exec_id": "e1b724c2-5e61-4aba-8b8a-47d8a5a4f111",
  "order_id": "f8c3de3d-1fea-4d7c-a8b0-29f63c4c3454",
  "symbol": "BTC-KRW",
  "side": "Buy",
  "price": 50000000,
  "quantity": 0.5,
  "fee": 0.05,
  "transaction_time": "2023-04-30T12:35:10.123Z"
}
```

### 2. 오더북 스트림

특정 심볼의 오더북 정보를 구독합니다.

- **URL**: `ws://127.0.0.1:3030/ws/orderbook/{symbol}`
  - 예: `ws://127.0.0.1:3030/ws/orderbook/BTC-KRW`
- **메서드**: WebSocket 연결

- **초기 스냅샷 메시지**: 연결 직후 현재 오더북의 전체 스냅샷이 전송됩니다.

```json
{
  "type": "orderbook_snapshot",
  "data": {
    "symbol": "BTC-KRW",
    "timestamp": 1682859310123,
    "bids": [
      { "price": 50000000, "volume": 1.5, "order_count": 3 },
      { "price": 49990000, "volume": 2.7, "order_count": 5 }
    ],
    "asks": [
      { "price": 50010000, "volume": 1.2, "order_count": 2 },
      { "price": 50020000, "volume": 3.4, "order_count": 4 }
    ]
  }
}
```

- **업데이트 메시지**: 오더북이 변경될 때마다 업데이트가 전송됩니다.

```json
{
  "type": "orderbook",
  "data": {
    "symbol": "BTC-KRW",
    "timestamp": 1682859320456,
    "bids": [
      { "price": 50000000, "volume": 1.2, "order_count": 2 },
      { "price": 49990000, "volume": 2.7, "order_count": 5 }
    ],
    "asks": [
      { "price": 50010000, "volume": 1.5, "order_count": 3 },
      { "price": 50020000, "volume": 3.4, "order_count": 4 }
    ]
  }
}
```

## 오류 응답

오류가 발생하면 다음 형식의 JSON 응답이 반환됩니다:

```json
{
  "error": {
    "code": "invalid_request",
    "message": "Invalid request parameters"
  }
}
```

## 데이터 모델

### 주문 (Order)

| 필드             | 타입      | 설명                                     |
|-----------------|-----------|------------------------------------------|
| order_id        | string    | 주문 고유 식별자                          |
| symbol          | string    | 거래 심볼 (예: "BTC-KRW")                |
| price           | number    | 주문 가격                                |
| quantity        | number    | 주문 수량                                |
| side            | string    | 주문 방향 ("Buy" 또는 "Sell")            |
| order_type      | string    | 주문 유형 ("Limit" 또는 "Market")        |
| status          | string    | 주문 상태 ("New", "PartiallyFilled", "Filled", "Cancelled") |
| filled_quantity | number    | 체결된 수량                              |
| remain_quantity | number    | 남은 수량                                |
| entry_time      | string    | 주문 생성 시간 (ISO 8601 형식)           |

### 체결 (Execution)

| 필드             | 타입      | 설명                                     |
|-----------------|-----------|------------------------------------------|
| exec_id         | string    | 체결 고유 식별자                          |
| order_id        | string    | 연관된 주문 ID                           |
| symbol          | string    | 거래 심볼 (예: "BTC-KRW")                |
| side            | string    | 주문 방향 ("Buy" 또는 "Sell")            |
| price           | number    | 체결 가격                                |
| quantity        | number    | 체결 수량                                |
| fee             | number    | 거래 수수료                              |
| transaction_time | string   | 체결 시간 (ISO 8601 형식)                |

### 오더북 (OrderBook)

| 필드             | 타입      | 설명                                     |
|-----------------|-----------|------------------------------------------|
| symbol          | string    | 거래 심볼                                |
| timestamp       | number    | 타임스탬프 (밀리초 단위 UNIX 시간)        |
| bids            | array     | 매수 호가 배열 (가격 내림차순)             |
| asks            | array     | 매도 호가 배열 (가격 오름차순)             |

### 가격 레벨 (PriceLevel)

| 필드             | 타입      | 설명                                     |
|-----------------|-----------|------------------------------------------|
| price           | number    | 가격 수준                                |
| volume          | number    | 해당 가격 수준의 총 수량                  |
| order_count     | number    | 해당 가격 수준의 주문 수                  |

## API 사용 예시

### cURL을 사용한 주문 생성

```bash
curl -X POST http://127.0.0.1:3030/v1/order \
  -H "Content-Type: application/json" \
  -d '{"symbol":"BTC-KRW","side":"Buy","price":50000000,"order_type":"Limit","quantity":1.5}'
```

### cURL을 사용한 주문 취소

```bash
curl -X POST http://127.0.0.1:3030/v1/order/cancel \
  -H "Content-Type: application/json" \
  -d '{"order_id":"f8c3de3d-1fea-4d7c-a8b0-29f63c4c3454"}'
```

### cURL을 사용한 체결 내역 조회

```bash
curl -X GET "http://127.0.0.1:3030/v1/execution?symbol=BTC-KRW&order_id=f8c3de3d-1fea-4d7c-a8b0-29f63c4c3454"
```

### JavaScript를 사용한 WebSocket 연결 예시

```javascript
// 체결 정보 스트림 연결
const execSocket = new WebSocket('ws://127.0.0.1:3030/ws/executions');

execSocket.onmessage = (event) => {
  const execution = JSON.parse(event.data);
  console.log('새로운 체결:', execution);
};

// 오더북 스트림 연결
const orderbookSocket = new WebSocket('ws://127.0.0.1:3030/ws/orderbook/BTC-KRW');

orderbookSocket.onmessage = (event) => {
  const data = JSON.parse(event.data);
  
  if (data.type === 'orderbook_snapshot') {
    console.log('오더북 스냅샷:', data.data);
  } else if (data.type === 'orderbook') {
    console.log('오더북 업데이트:', data.data);
  }
};
```

## 성능 및 확장성

API는 두 개의 독립적인 파이프라인으로 설계되어 각각 최적의 성능과 확장성을 제공합니다:

1. **주문 처리 파이프라인(REST API)**: 낮은 지연 시간과 높은 처리량에 최적화되어 있으며, 주문의 유효성 검사, 매칭, 체결을 책임집니다.

2. **UI 업데이트 파이프라인(WebSocket)**: 많은 수의 동시 클라이언트 연결을 효율적으로 처리하도록 설계되었으며, 오더북 상태 및 체결 정보를 실시간으로 전달합니다.

이러한 분리된 아키텍처는 시스템 전체의 안정성을 향상시키고, 한 파이프라인의 과부하가 다른 파이프라인에 영향을 주지 않도록 합니다.

## 제한사항 및 고려사항

- **인증**: 현재 버전에서는 인증이 구현되지 않았습니다. 프로덕션 환경에서는 적절한 인증 시스템을 구현해야 합니다.
- **속도 제한**: 현재 구현에는 API 속도 제한이 없지만, 프로덕션 환경에서는 DDoS 공격 방지를 위해 필요합니다.
- **WebSocket 연결 수**: 서버 리소스에 따라 최대 WebSocket 연결 수가 제한될 수 있습니다.
- **지연 시간**: UI 업데이트 파이프라인은 주문 처리 파이프라인보다 약간의 지연이 있을 수 있습니다.
