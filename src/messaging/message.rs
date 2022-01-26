use uuid::Uuid;

pub type Timestamp = i64;
pub type Price = f64;
pub type PairId = &'static str;
pub type EventId = Uuid;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct PriceUpdated {
    pub id: EventId,
    pub pair_id: PairId,
    pub datetime: Timestamp,
    pub price: Price,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum OrderType {
    Buy,
    Sell,
}

#[derive(PartialEq, Debug, Clone)]
pub struct MarketOrder {
    pub bid_currency: String,
    pub ask_currency: String,
    pub order_type: OrderType,
    pub amount: f64,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Msg {
    LivePriceUpdated(PriceUpdated),
    AveragePriceUpdated(PriceUpdated),
    OrderExecuted(MarketOrder),
    Buy,
    Sell,
    Shutdown,
}
